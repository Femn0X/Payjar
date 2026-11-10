//! Fast, semantics-preserving evaluation of pure integer constant expressions.
//!
//! This deliberately avoids compiling a fresh native function for every
//! expression. The previous Cranelift path paid JIT setup and code-generation
//! costs on every evaluation, which dwarfed the work for small expressions.
//! Unsupported trees and overflowing integer operations fall back to the
//! normal interpreter, which preserves PayJar's integer-to-float promotion.

use crate::payjar::ast::{AstNode, NodeType};

/// Evaluate a pure integer-literal arithmetic tree without allocating
/// intermediate runtime Values. Returns None for unsupported nodes/operators
/// or overflow, leaving those cases to the regular interpreter.
pub fn try_eval_integer_constant(node: &AstNode) -> Option<i64> {
    match &node.typ {
        NodeType::Int => Some(node.int_val),
        NodeType::BinaryOp => {
            let left = try_eval_integer_constant(node.left.as_deref()?)?;
            let right = try_eval_integer_constant(node.right.as_deref()?)?;
            match node.str_val.as_deref()? {
                "+" => left.checked_add(right),
                "-" => left.checked_sub(right),
                "*" => left.checked_mul(right),
                _ => None,
            }
        }
        _ => None,
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
    fn folds_nested_integer_arithmetic() {
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

    #[test]
    fn non_constant_children_fall_back() {
        let mut variable = AstNode::new(NodeType::VarAccess);
        variable.name = Some("x".to_owned());
        let expression = binary("+", int(1), variable);
        assert_eq!(try_eval_integer_constant(&expression), None);
    }
}
