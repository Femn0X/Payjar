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
    let value = validate(node)?;
    let mut out = String::new();
    match target {
        AssemblyTarget::X86_64 => {
            out.push_str(".text\n.intel_syntax noprefix\n.globl payjar_entry\n.type payjar_entry, @function\npayjar_entry:\n");
            if i32::try_from(value).is_ok() {
                out.push_str(&format!("    mov rax, {}\n", value));
            } else {
                out.push_str(&format!("    movabs rax, {}\n", value));
            }
            out.push_str("    ret\n.size payjar_entry, .-payjar_entry\n.section .note.GNU-stack,\"\",@progbits\n");
        }
        AssemblyTarget::Aarch64 => {
            out.push_str(".text\n.globl payjar_entry\n.type payjar_entry, %function\npayjar_entry:\n");
            emit_aarch64_constant(value, &mut out);
            out.push_str("    ret\n.size payjar_entry, .-payjar_entry\n.section .note.GNU-stack,\"\",%progbits\n");
        }
    }
    Ok(out)
}

/// Choose MOVZ or MOVN according to which needs fewer MOVK patches.
fn emit_aarch64_constant(value: i64, out: &mut String) {
    let bits = value as u64;
    let parts = [bits as u16, (bits >> 16) as u16, (bits >> 32) as u16, (bits >> 48) as u16];
    let inverted = parts.iter().filter(|&&p| p != u16::MAX).count()
        < parts.iter().filter(|&&p| p != 0).count();
    let fill = if inverted { u16::MAX } else { 0 };
    let first = parts.iter().position(|&p| p != fill).unwrap_or(0);
    let imm = if inverted { !parts[first] } else { parts[first] };
    let op = if inverted { "movn" } else { "movz" };
    out.push_str(&format!("    {} x0, #{}, lsl #{}\n", op, imm, first * 16));
    for (i, &part) in parts.iter().enumerate() {
        if i != first && part != fill {
            out.push_str(&format!("    movk x0, #{}, lsl #{}\n", part, i * 16));
        }
    }
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
    fn folds_x86_expression_without_stack_traffic() {
        let expr = binary("+", int(10), binary("*", int(6), int(7)));
        let asm = emit_integer_expression(&expr, AssemblyTarget::X86_64).unwrap();
        assert!(asm.contains("mov rax, 52\n    ret"));
        assert!(!asm.contains("push"));
        assert!(!asm.contains("imul"));
        assert!(asm.contains(".note.GNU-stack"));
    }

    #[test]
    fn folds_aarch64_expression_without_stack_frame() {
        let expr = binary("-", int(100), int(42));
        let asm = emit_integer_expression(&expr, AssemblyTarget::Aarch64).unwrap();
        assert!(asm.contains("movz x0, #58, lsl #0\n    ret"));
        assert!(!asm.contains("[sp"));
    }

    #[test]
    fn materializes_full_width_and_negative_constants() {
        let asm = emit_integer_expression(&int(i64::MIN), AssemblyTarget::X86_64).unwrap();
        assert!(asm.contains("movabs rax, -9223372036854775808"));
        let asm = emit_integer_expression(&int(-1), AssemblyTarget::Aarch64).unwrap();
        assert!(asm.contains("movn x0, #0, lsl #0"));
        assert!(!asm.contains("movk"));
        let asm = emit_integer_expression(&int(i64::MIN), AssemblyTarget::Aarch64).unwrap();
        assert!(asm.contains("movz x0, #32768, lsl #48"));
        assert!(!asm.contains("movk"));
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
