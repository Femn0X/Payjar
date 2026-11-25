// Standalone backend harness: no GUI dependencies or cross Rust std needed.
extern crate self as payjar;
#[path = "../src/payjar/ast.rs"]
pub mod ast;
#[path = "../src/payjar/asm.rs"]
pub mod asm;
#[path = "../src/payjar/asm_vm.rs"]
pub mod asm_vm;
#[path = "../src/payjar/asm_native.rs"]
pub mod asm_native;
use payjar::{asm::{AssemblyTarget, emit_integer_expression}, ast::{AstNode, NodeType}};
fn main() {
    let target = match std::env::args().nth(1).as_deref() {
        Some("x86_64") => AssemblyTarget::X86_64,
        Some("aarch64") => AssemblyTarget::Aarch64,
        Some("riscv64") => AssemblyTarget::Riscv64,
        Some("arm") => AssemblyTarget::Arm,
        Some("x86") => AssemblyTarget::X86,
        _ => panic!("unknown architecture"),
    };
    if std::env::args().nth(2).as_deref() == Some("native") {
        use asm_vm::{Instruction as I, Program};
        let case = std::env::args().nth(3).unwrap();
        let (code, local_count) = match case.as_str() {
            "loop" => (vec![I::Const(5), I::StoreLocal(0), I::Const(0), I::StoreLocal(1),
                I::LoadLocal(0), I::JumpIfZero(15), I::LoadLocal(1), I::LoadLocal(0),
                I::Add, I::StoreLocal(1), I::LoadLocal(0), I::Const(1), I::Sub,
                I::StoreLocal(0), I::Jump(4), I::LoadLocal(1), I::Return], 2),
            "overflow" => (vec![I::Const(i64::MAX), I::Const(1), I::Add, I::Return], 0),
            "fuel" => (vec![I::Jump(0)], 0),
            _ => panic!("unknown native case"),
        };
        print!("{}", asm_native::emit_program(&Program { code, local_count }, target).unwrap());
        return;
    }
    let value: i64 = std::env::args().nth(2).unwrap().parse().unwrap();
    let mut node = AstNode::new(NodeType::Int);
    node.int_val = value;
    print!("{}", emit_integer_expression(&node, target).unwrap());
}
