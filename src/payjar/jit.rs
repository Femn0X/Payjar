//! Small Cranelift-backed fast path for pure integer arithmetic.
//!
//! Unsupported expressions deliberately return None so the tree-walking
//! interpreter remains the semantic authority and compatibility fallback.
use crate::payjar::ast::{AstNode, NodeType};
use cranelift_codegen::ir::{types, AbiParam, InstBuilder};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};

fn lower(node: &AstNode, builder: &mut FunctionBuilder<'_>) -> Option<cranelift_codegen::ir::Value> {
    match node.typ {
        NodeType::Int => Some(builder.ins().iconst(types::I64, node.int_val)),
        NodeType::BinaryOp => {
            let op = node.str_val.as_deref()?;
            if !matches!(op, "+" | "-" | "*") { return None; }
            let left = lower(node.left.as_deref()?, builder)?;
            let right = lower(node.right.as_deref()?, builder)?;
            Some(match op {
                "+" => builder.ins().iadd(left, right),
                "-" => builder.ins().isub(left, right),
                "*" => builder.ins().imul(left, right),
                _ => return None,
            })
        }
        _ => None,
    }
}

/// JIT a pure integer-literal expression. Returns None for unsupported ASTs,
/// backend errors, or arithmetic whose interpreter semantics could overflow.
pub fn try_eval_integer_constant(node: &AstNode) -> Option<i64> {
    fn checked(node: &AstNode) -> Option<i64> {
        match node.typ {
            NodeType::Int => Some(node.int_val),
            NodeType::BinaryOp => {
                let a = checked(node.left.as_deref()?)?;
                let b = checked(node.right.as_deref()?)?;
                match node.str_val.as_deref()? {
                    "+" => a.checked_add(b),
                    "-" => a.checked_sub(b),
                    "*" => a.checked_mul(b),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    // Preflight preserves the interpreter's overflow behavior (promote to Float).
    let expected = checked(node)?;
    let builder = JITBuilder::new(default_libcall_names()).ok()?;
    let mut module = JITModule::new(builder);
    let mut ctx = module.make_context();
    let mut sig = module.make_signature();
    sig.returns.push(AbiParam::new(types::I64));
    ctx.func.signature = sig;
    let mut fb_ctx = FunctionBuilderContext::new();
    {
        let mut fb = FunctionBuilder::new(&mut ctx.func, &mut fb_ctx);
        let entry = fb.create_block();
        fb.switch_to_block(entry);
        fb.seal_block(entry);
        let value = lower(node, &mut fb)?;
        fb.ins().return_(&[value]);
        fb.finalize();
    }
    let id = module.declare_function("payjar_jit_const", Linkage::Local, &ctx.func.signature).ok()?;
    module.define_function(id, &mut ctx).ok()?;
    module.clear_context(&mut ctx);
    module.finalize_definitions().ok()?;
    let code = module.get_finalized_function(id);
    let f: extern "C" fn() -> i64 = unsafe { std::mem::transmute(code) };
    let actual = f();
    (actual == expected).then_some(actual)
}
