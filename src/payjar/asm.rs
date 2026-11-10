//! First native-assembly backend for PayJar.
//!
//! This module deliberately starts with a small, auditable subset: pure
//! integer-literal expressions using +, -, and *. Both targets consume the
//! same AST and emit a zero-argument function returning an i64 in the
//! platform's standard integer return register.
//!
//! Output uses GNU assembler syntax for ELF targets (Linux). It is assembly
//! text, not executable machine code; executable JIT memory management and
//! the full interpreter/runtime bridge are later milestones.

use crate::payjar::ast::{AstNode, NodeType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssemblyTarget {
    X86_64,
    Aarch64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssemblyError {
    UnsupportedExpression,
    IntegerOverflow,
}

impl std::fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedExpression => write!(f, "expression is not supported by the assembly backend"),
            Self::IntegerOverflow => write!(f, "expression overflows PayJar's integer range"),
        }
    }
}

impl std::error::Error for AssemblyError {}

/// Validate the supported subset and preserve checked-integer semantics.
fn validate(node: &AstNode) -> Result<i64, AssemblyError> {
    match &node.typ {
        NodeType::Int => Ok(node.int_val),
        NodeType::BinaryOp => {
            let left = validate(node.left.as_deref().ok_or(AssemblyError::UnsupportedExpression)?)?;
            let right = validate(node.right.as_deref().ok_or(AssemblyError::UnsupportedExpression)?)?;
            match node.str_val.as_deref() {
                Some("+") => left.checked_add(right).ok_or(AssemblyError::IntegerOverflow),
                Some("-") => left.checked_sub(right).ok_or(AssemblyError::IntegerOverflow),
                Some("*") => left.checked_mul(right).ok_or(AssemblyError::IntegerOverflow),
                _ => Err(AssemblyError::UnsupportedExpression),
            }
        }
        _ => Err(AssemblyError::UnsupportedExpression),
    }
}

/// Emit a complete native function named `payjar_entry`.
///
/// x86-64 uses the System V ABI (result in RAX); AArch64 uses AAPCS64
/// (result in X0). Output currently targets GNU assembler on ELF platforms.
pub fn emit_integer_expression(
    node: &AstNode,
    target: AssemblyTarget,
) -> Result<String, AssemblyError> {
    validate(node)?;
    let mut out = String::new();
    match target {
        AssemblyTarget::X86_64 => {
            out.push_str(".text\n.globl payjar_entry\n.type payjar_entry, @function\npayjar_entry:\n");
            emit_x86(node, &mut out)?;
            out.push_str("    pop rax\n    ret\n.size payjar_entry, .-payjar_entry\n");
        }
        AssemblyTarget::Aarch64 => {
            out.push_str(".text\n.globl payjar_entry\n.type payjar_entry, %function\npayjar_entry:\n");
            out.push_str("    stp x29, x30, [sp, #-16]!\n    mov x29, sp\n");
            emit_aarch64(node, &mut out)?;
            out.push_str("    ldp x29, x30, [sp], #16\n    ret\n.size payjar_entry, .-payjar_entry\n");
        }
    }
    Ok(out)
}

fn emit_x86(node: &AstNode, out: &mut String) -> Result<(), AssemblyError> {
    match &node.typ {
        NodeType::Int => {
            out.push_str(&format!("    movabs rax, {}\n    push rax\n", node.int_val));
        }
        NodeType::BinaryOp => {
            emit_x86(node.left.as_deref().ok_or(AssemblyError::UnsupportedExpression)?, out)?;
            emit_x86(node.right.as_deref().ok_or(AssemblyError::UnsupportedExpression)?, out)?;
            out.push_str("    pop rbx\n    pop rax\n");
            match node.str_val.as_deref() {
                Some("+") => out.push_str("    add rax, rbx\n"),
                Some("-") => out.push_str("    sub rax, rbx\n"),
                Some("*") => out.push_str("    imul rax, rbx\n"),
                _ => return Err(AssemblyError::UnsupportedExpression),
            }
            out.push_str("    push rax\n");
        }
        _ => return Err(AssemblyError::UnsupportedExpression),
    }
    Ok(())
}

fn emit_aarch64(node: &AstNode, out: &mut String) -> Result<(), AssemblyError> {
    match &node.typ {
        NodeType::Int => {
            let bits = node.int_val as u64;
            for shift in (0..64).step_by(16) {
                let part = (bits >> shift) & 0xffff;
                if shift == 0 {
                    out.push_str(&format!("    movz x0, #{}\n", part));
                } else if part != 0 {
                    out.push_str(&format!("    movk x0, #{}, lsl #{}\n", part, shift));
                }
            }
            
        }
        NodeType::BinaryOp => {
            emit_aarch64(node.left.as_deref().ok_or(AssemblyError::UnsupportedExpression)?, out)?;
            out.push_str("    sub sp, sp, #16\n    str x0, [sp]\n");
            emit_aarch64(node.right.as_deref().ok_or(AssemblyError::UnsupportedExpression)?, out)?;
            out.push_str("    mov x9, x0\n    ldr x0, [sp]\n    add sp, sp, #16\n");
            match node.str_val.as_deref() {
                Some("+") => out.push_str("    add x0, x0, x9\n"),
                Some("-") => out.push_str("    sub x0, x0, x9\n"),
                Some("*") => out.push_str("    mul x0, x0, x9\n"),
                _ => return Err(AssemblyError::UnsupportedExpression),
            }
            }
        _ => return Err(AssemblyError::UnsupportedExpression),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(value: i64) -> AstNode {
        let mut node = AstNode::new(NodeType::Int);
        node.int_val = value;
        node
    }

    fn binary(op: &str, left: AstNode, right: AstNode) -> AstNode {
        let mut node = AstNode::new(NodeType::BinaryOp);
        node.str_val = Some(op.to_owned());
        node.left = Some(Box::new(left));
        node.right = Some(Box::new(right));
        node
    }

    #[test]
    fn emits_x86_64_integer_arithmetic_and_balances_stack() {
        let expr = binary("+", int(10), binary("*", int(6), int(7)));
        let asm = emit_integer_expression(&expr, AssemblyTarget::X86_64).unwrap();
        assert!(asm.contains("imul rax, rbx"));
        assert!(asm.contains("add rax, rbx"));
        assert!(asm.contains("movabs rax, 10"));
        assert!(asm.contains("    pop rax\n    ret"));
    }

    #[test]
    fn emits_aarch64_integer_arithmetic_and_restores_frame() {
        let expr = binary("-", int(100), int(42));
        let asm = emit_integer_expression(&expr, AssemblyTarget::Aarch64).unwrap();
        assert!(asm.contains("sub x0, x0, x9"));
        assert!(asm.contains("movz x0, #100"));
        assert!(asm.contains("ldr x0, [sp]\n    add sp, sp, #16\n    sub x0, x0, x9"));
    }

    #[test]
    fn rejects_overflow_and_unsupported_nodes() {
        let overflow = binary("+", int(i64::MAX), int(1));
        assert_eq!(emit_integer_expression(&overflow, AssemblyTarget::X86_64), Err(AssemblyError::IntegerOverflow));
        let mut float = AstNode::new(NodeType::Float);
        float.fval = 1.5;
        assert_eq!(emit_integer_expression(&float, AssemblyTarget::X86_64), Err(AssemblyError::UnsupportedExpression));
    }
}
