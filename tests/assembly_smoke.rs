// Standalone backend harness: no GUI dependencies or cross Rust std needed.
mod payjar {
    pub mod ast { include!("../src/payjar/ast.rs"); }
    pub mod asm { include!("../src/payjar/asm.rs"); }
}
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
    let value: i64 = std::env::args().nth(2).unwrap().parse().unwrap();
    let mut node = AstNode::new(NodeType::Int);
    node.int_val = value;
    print!("{}", emit_integer_expression(&node, target).unwrap());
}
