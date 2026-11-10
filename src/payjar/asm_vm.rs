//! Experimental shared bytecode VM for PayJar's native backends.
//!
//! This is the architecture-neutral execution layer. It intentionally supports
//! only integer locals, integer arithmetic/comparisons, if/while, and return.
//! Unsupported AST nodes must continue through the existing interpreter.
//! Native x86-64/AArch64 lowering is a separate layer built on this stable ISA.

use crate::payjar::ast::{AstNode, NodeType};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Const(i64),
    LoadLocal(usize),
    StoreLocal(usize),
    Add,
    Sub,
    Mul,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    JumpIfZero(usize),
    Jump(usize),
    Return,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    pub code: Vec<Instruction>,
    pub local_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    UnsupportedNode,
    UnsupportedOperator,
    UnknownVariable(String),
    DuplicateVariable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmError {
    StackUnderflow,
    InvalidLocal(usize),
    InvalidJump(usize),
    MissingReturn,
    StepLimitExceeded,
    IntegerOverflow,
}

#[derive(Debug, Clone, Copy)]
pub struct Vm {
    /// A guard against accidental infinite loops in experimental bytecode.
    pub max_steps: usize,
}

impl Default for Vm {
    fn default() -> Self {
        Self { max_steps: 1_000_000 }
    }
}

impl Vm {
    pub fn execute(&self, program: &Program) -> Result<i64, VmError> {
        let mut locals = vec![0_i64; program.local_count];
        let mut stack: Vec<i64> = Vec::new();
        let mut pc = 0_usize;
        let mut steps = 0_usize;

        loop {
            if steps >= self.max_steps {
                return Err(VmError::StepLimitExceeded);
            }
            steps += 1;
            let instruction = program.code.get(pc).ok_or(VmError::MissingReturn)?;
            pc += 1;

            match *instruction {
                Instruction::Const(value) => stack.push(value),
                Instruction::LoadLocal(index) => {
                    stack.push(*locals.get(index).ok_or(VmError::InvalidLocal(index))?);
                }
                Instruction::StoreLocal(index) => {
                    let value = stack.pop().ok_or(VmError::StackUnderflow)?;
                    *locals.get_mut(index).ok_or(VmError::InvalidLocal(index))? = value;
                }
                Instruction::Add | Instruction::Sub | Instruction::Mul
                | Instruction::Eq | Instruction::Ne | Instruction::Lt
                | Instruction::Le | Instruction::Gt | Instruction::Ge => {
                    let rhs = stack.pop().ok_or(VmError::StackUnderflow)?;
                    let lhs = stack.pop().ok_or(VmError::StackUnderflow)?;
                    let value = match instruction {
                        Instruction::Add => lhs.checked_add(rhs).ok_or(VmError::IntegerOverflow)?,
                        Instruction::Sub => lhs.checked_sub(rhs).ok_or(VmError::IntegerOverflow)?,
                        Instruction::Mul => lhs.checked_mul(rhs).ok_or(VmError::IntegerOverflow)?,
                        Instruction::Eq => if lhs == rhs { 1 } else { 0 },
                        Instruction::Ne => if lhs != rhs { 1 } else { 0 },
                        Instruction::Lt => if lhs < rhs { 1 } else { 0 },
                        Instruction::Le => if lhs <= rhs { 1 } else { 0 },
                        Instruction::Gt => if lhs > rhs { 1 } else { 0 },
                        Instruction::Ge => if lhs >= rhs { 1 } else { 0 },
                        _ => unreachable!(),
                    };
                    stack.push(value);
                }
                Instruction::JumpIfZero(target) => {
                    let condition = stack.pop().ok_or(VmError::StackUnderflow)?;
                    if condition == 0 {
                        if target > program.code.len() { return Err(VmError::InvalidJump(target)); }
                        pc = target;
                    }
                }
                Instruction::Jump(target) => {
                    if target > program.code.len() { return Err(VmError::InvalidJump(target)); }
                    pc = target;
                }
                Instruction::Return => return stack.pop().ok_or(VmError::StackUnderflow),
            }
        }
    }
}

/// Compile a straight-line/block-structured subset of the PayJar AST.
/// Locals share one function frame; lexical shadowing is deliberately rejected
/// until the runtime's scope rules are represented in the bytecode format.
pub fn compile(nodes: &[AstNode]) -> Result<Program, CompileError> {
    let mut compiler = Compiler { code: Vec::new(), locals: HashMap::new() };
    compiler.statements(nodes)?;
    // Every control-flow path needs a valid fallthrough return, including
    // the false edge of an if-without-else.
    compiler.code.push(Instruction::Const(0));
    compiler.code.push(Instruction::Return);
    Ok(Program { code: compiler.code, local_count: compiler.locals.len() })
}

struct Compiler {
    code: Vec<Instruction>,
    locals: HashMap<String, usize>,
}

impl Compiler {
    fn expression(&mut self, node: &AstNode) -> Result<(), CompileError> {
        match &node.typ {
            NodeType::Int => self.code.push(Instruction::Const(node.int_val)),
            NodeType::VarAccess => {
                let name = node.name.as_deref().unwrap_or_default();
                let index = *self.locals.get(name)
                    .ok_or_else(|| CompileError::UnknownVariable(name.to_owned()))?;
                self.code.push(Instruction::LoadLocal(index));
            }
            NodeType::BinaryOp => {
                self.expression(node.left.as_deref().ok_or(CompileError::UnsupportedNode)?)?;
                self.expression(node.right.as_deref().ok_or(CompileError::UnsupportedNode)?)?;
                let instruction = match node.str_val.as_deref() {
                    Some("+") => Instruction::Add,
                    Some("-") => Instruction::Sub,
                    Some("*") => Instruction::Mul,
                    Some("==") => Instruction::Eq,
                    Some("!=") => Instruction::Ne,
                    Some("<") => Instruction::Lt,
                    Some("<=") => Instruction::Le,
                    Some(">") => Instruction::Gt,
                    Some(">=") => Instruction::Ge,
                    _ => return Err(CompileError::UnsupportedOperator),
                };
                self.code.push(instruction);
            }
            _ => return Err(CompileError::UnsupportedNode),
        }
        Ok(())
    }

    fn statements(&mut self, nodes: &[AstNode]) -> Result<(), CompileError> {
        for node in nodes {
            match &node.typ {
                NodeType::VarDecl => {
                    let name = node.name.as_deref().unwrap_or_default().to_owned();
                    if self.locals.contains_key(&name) {
                        return Err(CompileError::DuplicateVariable(name));
                    }
                    let index = self.locals.len();
                    self.locals.insert(name, index);
                    self.expression(node.expr.as_deref().ok_or(CompileError::UnsupportedNode)?)?;
                    self.code.push(Instruction::StoreLocal(index));
                }
                NodeType::Assign => {
                    let name = node.name.as_deref().unwrap_or_default();
                    let index = *self.locals.get(name)
                        .ok_or_else(|| CompileError::UnknownVariable(name.to_owned()))?;
                    self.expression(node.expr.as_deref().ok_or(CompileError::UnsupportedNode)?)?;
                    self.code.push(Instruction::StoreLocal(index));
                }
                NodeType::If => {
                    self.expression(node.expr.as_deref().ok_or(CompileError::UnsupportedNode)?)?;
                    let branch = self.code.len();
                    self.code.push(Instruction::JumpIfZero(usize::MAX));
                    self.statements(&node.body)?;
                    if node.else_body.is_empty() {
                        let end = self.code.len();
                        self.code[branch] = Instruction::JumpIfZero(end);
                    } else {
                        let skip_else = self.code.len();
                        self.code.push(Instruction::Jump(usize::MAX));
                        let else_start = self.code.len();
                        self.code[branch] = Instruction::JumpIfZero(else_start);
                        self.statements(&node.else_body)?;
                        let end = self.code.len();
                        self.code[skip_else] = Instruction::Jump(end);
                    }
                }
                NodeType::While => {
                    let loop_start = self.code.len();
                    self.expression(node.expr.as_deref().ok_or(CompileError::UnsupportedNode)?)?;
                    let exit_jump = self.code.len();
                    self.code.push(Instruction::JumpIfZero(usize::MAX));
                    self.statements(&node.body)?;
                    self.code.push(Instruction::Jump(loop_start));
                    let end = self.code.len();
                    self.code[exit_jump] = Instruction::JumpIfZero(end);
                }
                NodeType::Return => {
                    self.expression(node.expr.as_deref().ok_or(CompileError::UnsupportedNode)?)?;
                    self.code.push(Instruction::Return);
                }
                _ => return Err(CompileError::UnsupportedNode),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(value: i64) -> AstNode {
        let mut n = AstNode::new(NodeType::Int);
        n.int_val = value;
        n
    }

    fn var(name: &str) -> AstNode {
        let mut n = AstNode::new(NodeType::VarAccess);
        n.name = Some(name.to_owned());
        n
    }

    fn binary(op: &str, left: AstNode, right: AstNode) -> AstNode {
        let mut n = AstNode::new(NodeType::BinaryOp);
        n.str_val = Some(op.to_owned());
        n.left = Some(Box::new(left));
        n.right = Some(Box::new(right));
        n
    }

    fn declaration(name: &str, value: AstNode) -> AstNode {
        let mut n = AstNode::new(NodeType::VarDecl);
        n.name = Some(name.to_owned());
        n.expr = Some(Box::new(value));
        n
    }

    fn assignment(name: &str, value: AstNode) -> AstNode {
        let mut n = AstNode::new(NodeType::Assign);
        n.name = Some(name.to_owned());
        n.expr = Some(Box::new(value));
        n
    }

    fn while_loop(condition: AstNode, body: Vec<AstNode>) -> AstNode {
        let mut n = AstNode::new(NodeType::While);
        n.expr = Some(Box::new(condition));
        n.body = body;
        n
    }

    fn returning(value: AstNode) -> AstNode {
        let mut n = AstNode::new(NodeType::Return);
        n.expr = Some(Box::new(value));
        n
    }

    #[test]
    fn executes_locals_arithmetic_and_return() {
        let ast = vec![
            declaration("x", int(40)),
            assignment("x", binary("+", var("x"), int(2))),
            returning(var("x")),
        ];
        let program = compile(&ast).unwrap();
        assert_eq!(Vm::default().execute(&program), Ok(42));
        assert_eq!(program.local_count, 1);
    }

    #[test]
    fn executes_while_loop_and_comparison() {
        let ast = vec![
            declaration("i", int(0)),
            declaration("sum", int(0)),
            while_loop(
                binary("<", var("i"), int(5)),
                vec![
                    assignment("sum", binary("+", var("sum"), var("i"))),
                    assignment("i", binary("+", var("i"), int(1))),
                ],
            ),
            returning(var("sum")),
        ];
        let program = compile(&ast).unwrap();
        assert_eq!(Vm::default().execute(&program), Ok(10));
    }

    #[test]
    fn executes_if_else() {
        let mut branch = AstNode::new(NodeType::If);
        branch.expr = Some(Box::new(binary(">", int(9), int(3))));
        branch.body = vec![returning(int(1))];
        branch.else_body = vec![returning(int(0))];
        let program = compile(&[branch]).unwrap();
        assert_eq!(Vm::default().execute(&program), Ok(1));
    }

    #[test]
    fn rejects_unsupported_ast_and_guards_infinite_loops() {
        let mut float = AstNode::new(NodeType::Float);
        float.fval = 1.25;
        assert_eq!(compile(&[returning(float)]), Err(CompileError::UnsupportedNode));

        let loop_node = while_loop(int(1), vec![]);
        let program = compile(&[loop_node]).unwrap();
        let vm = Vm { max_steps: 12 };
        assert_eq!(vm.execute(&program), Err(VmError::StepLimitExceeded));
    }
}
