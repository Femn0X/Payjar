//! Small Cranelift-backed fast path for pure integer arithmetic.
//!
//! Unsupported expressions deliberately return None so the tree-walking
//! interpreter remains the semantic authority and compatibility fallback.
use crate::payjar::ast::{AstNode, NodeType};
use cranelift_codegen::ir::{types, AbiParam, InstBuilder};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};
use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    /// Cache results for repeated identical constant trees so the JIT compiler
    /// is not recreated on every evaluation of the same expression.
    static CONSTANT_CACHE: RefCell<HashMap<String, i64>> = RefCell::new(HashMap::new());
}

fn cache_key(node: &AstNode, out: &mut String) -> Option<()> {
    match &node.typ {
        NodeType::Int => out.push_str(&format!("i{};", node.int_val)),
        NodeType::BinaryOp => {
            let op = node.str_val.as_deref()?;
            if !matches!(op, "+" | "-" | "*") { return None; }
            out.push('(');
            out.push_str(op);
            out.push(':');
            cache_key(node.left.as_deref()?, out)?;
            cache_key(node.right.as_deref()?, out)?;
            out.push(')');
        }
        _ => return None,
    }
    Some(())
}

fn lower(node: &AstNode, builder: &mut FunctionBuilder<'_>) -> Option<cranelift_codegen::ir::Value> {
    match &node.typ {
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
        match &node.typ {
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
    let code = unsafe { module.get_finalized_function(id) };
    let f: extern "C" fn() -> i64 = unsafe { std::mem::transmute(code) };
    let actual = f();
    if actual != expected {
        return None;
    }
    /* WIP
    CONSTANT_CACHE.with(|cache| {
        cache.borrow_mut().insert(key, actual);
    });*/
    Some(actual)
}


#[cfg(test)]
mod tests {
    use super::*;

    fn int(value: i64) -> AstNode {
        let mut n = AstNode::new(NodeType::Int);
        n.int_val = value;
        n
    }

    fn binary(op: &str, left: AstNode, right: AstNode) -> AstNode {
        let mut n = AstNode::new(NodeType::BinaryOp);
        n.str_val = Some(op.to_string());
        n.left = Some(Box::new(left));
        n.right = Some(Box::new(right));
        n
    }

    #[test]
    fn jit_evaluates_nested_integer_arithmetic() {
        let expression = binary("+", int(10), binary("*", int(6), int(7)));
        assert_eq!(try_eval_integer_constant(&expression), Some(52));
    }

    #[test]
    fn unsupported_operators_fall_back() {
        let expression = binary("/", int(8), int(2));
        assert_eq!(try_eval_integer_constant(&expression), None);
    }

    #[test]
    fn overflowing_integer_expression_falls_back() {
        let expression = binary("+", int(i64::MAX), int(1));
        assert_eq!(try_eval_integer_constant(&expression), None);
    }
}
