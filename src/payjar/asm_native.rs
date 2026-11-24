//! Experimental ELF/System V native lowering for shared integer bytecode.
use super::asm::AssemblyTarget;
use super::asm_vm::{Instruction as I, Program};
use std::collections::VecDeque;
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeError {
    UnsupportedTarget,
    InvalidProgram,
    FrameTooLarge,
}

/// Verify reachable control flow and calculate maximum operand stack depth.
/// Reject invalid indices even in unreachable instructions.
pub fn verify(program: &Program) -> Result<usize, NativeError> {
    let n = program.code.len();
    if n == 0 { return Err(NativeError::InvalidProgram); }
    for op in &program.code {
        match *op {
            I::LoadLocal(i) | I::StoreLocal(i) if i >= program.local_count =>
                return Err(NativeError::InvalidProgram),
            I::Jump(i) | I::JumpIfZero(i) if i >= n =>
                return Err(NativeError::InvalidProgram),
            _ => {}
        }
    }
    let mut heights = vec![None; n];
    heights[0] = Some(0usize);
    let mut queue = VecDeque::from([0usize]);
    let mut max = 0;
    while let Some(pc) = queue.pop_front() {
        let height = heights[pc].unwrap();
        let (take, give) = match program.code[pc] {
            I::Const(_) | I::LoadLocal(_) => (0, 1),
            I::StoreLocal(_) | I::JumpIfZero(_) | I::Return => (1, 0),
            I::Jump(_) => (0, 0),
            _ => (2, 1),
        };
        if height < take { return Err(NativeError::InvalidProgram); }
        let next = height - take + give;
        max = max.max(height).max(next);
        let mut edge = |dest: usize| -> Result<(), NativeError> {
            let slot = heights.get_mut(dest).ok_or(NativeError::InvalidProgram)?;
            match *slot {
                Some(old) if old != next => Err(NativeError::InvalidProgram),
                Some(_) => Ok(()),
                None => { *slot = Some(next); queue.push_back(dest); Ok(()) }
            }
        };
        match program.code[pc] {
            I::Return => {}
            I::Jump(dest) => edge(dest)?,
            I::JumpIfZero(dest) => { edge(dest)?; edge(pc + 1)?; }
            _ => edge(pc + 1)?,
        }
    }
    Ok(max)
}

/// Emit int payjar_run(int64_t *result, uint64_t fuel).
/// Status: 0 success, 1 integer overflow, 2 fuel exhausted.
/// Caller must supply a writable result pointer; failures leave it unchanged.
pub fn emit_program(program: &Program, target: AssemblyTarget) -> Result<String, NativeError> {
    if target != AssemblyTarget::X86_64 { return Err(NativeError::UnsupportedTarget); }
    let depth = verify(program)?;
    let slots = program.local_count.checked_add(depth).ok_or(NativeError::FrameTooLarge)?;
    // Bound native stack use; the current backend is deliberately experimental.
    if slots > 8192 { return Err(NativeError::FrameTooLarge); }
    let frame = (slots * 8 + 15) & !15;
    let mut out = String::from(".text\n.intel_syntax noprefix\n.globl payjar_run\n.type payjar_run, @function\npayjar_run:\n    push rbp\n    mov rbp, rsp\n");
    writeln!(out, "    sub rsp, {frame}").unwrap();
    // Locals begin zeroed, matching the portable VM. r8 points at operand storage.
    for i in 0..program.local_count {
        writeln!(out, "    mov QWORD PTR [rsp + {}], 0", i * 8).unwrap();
    }
    writeln!(out, "    lea r8, [rsp + {}]", program.local_count * 8).unwrap();
    for (pc, op) in program.code.iter().enumerate() {
        writeln!(out, ".Lpj_{pc}:\n    test rsi, rsi\n    jz .Lpj_fuel\n    dec rsi").unwrap();
        match *op {
            I::Const(v) => {
                writeln!(out, "    movabs rax, {v}\n    mov [r8], rax\n    add r8, 8").unwrap();
            }
            I::LoadLocal(i) => {
                writeln!(out, "    mov rax, [rsp + {}]\n    mov [r8], rax\n    add r8, 8", i * 8).unwrap();
            }
            I::StoreLocal(i) => {
                writeln!(out, "    sub r8, 8\n    mov rax, [r8]\n    mov [rsp + {}], rax", i * 8).unwrap();
            }
            I::Jump(dest) => { writeln!(out, "    jmp .Lpj_{dest}").unwrap(); }
            I::JumpIfZero(dest) => {
                writeln!(out, "    sub r8, 8\n    cmp QWORD PTR [r8], 0\n    je .Lpj_{dest}").unwrap();
            }
            I::Return => out.push_str("    mov rax, [r8 - 8]\n    mov [rdi], rax\n    xor eax, eax\n    jmp .Lpj_exit\n"),
            _ => {
                out.push_str("    sub r8, 8\n    mov rcx, [r8]\n    mov rax, [r8 - 8]\n");
                match op {
                    I::Add | I::Sub | I::Mul => {
                        let mnemonic = match op { I::Add => "add", I::Sub => "sub", _ => "imul" };
                        writeln!(out, "    {mnemonic} rax, rcx\n    jo .Lpj_overflow").unwrap();
                    }
                    _ => {
                        let cc = match op {
                            I::Eq => "e", I::Ne => "ne", I::Lt => "l", I::Le => "le",
                            I::Gt => "g", I::Ge => "ge", _ => unreachable!(),
                        };
                        writeln!(out, "    cmp rax, rcx\n    set{cc} al\n    movzx eax, al").unwrap();
                    }
                }
                out.push_str("    mov [r8 - 8], rax\n");
            }
        }
    }
    out.push_str(".Lpj_overflow:\n    mov eax, 1\n    jmp .Lpj_exit\n.Lpj_fuel:\n    mov eax, 2\n.Lpj_exit:\n    leave\n    ret\n.size payjar_run, .-payjar_run\n.section .note.GNU-stack,\"\",@progbits\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(code: Vec<I>) -> Program { Program { code, local_count: 0 } }
    #[test]
    fn verifies_stack_and_branches() {
        assert_eq!(verify(&p(vec![I::Const(1), I::Const(2), I::Add, I::Return])), Ok(2));
        assert_eq!(verify(&p(vec![I::Add, I::Return])), Err(NativeError::InvalidProgram));
        assert_eq!(verify(&p(vec![I::Jump(1)])), Err(NativeError::InvalidProgram));
        assert_eq!(verify(&p(vec![I::Const(1), I::JumpIfZero(3), I::Const(2), I::Jump(3)])), Err(NativeError::InvalidProgram));
    }
    #[test]
    fn emits_checked_arithmetic_and_fuel() {
        let program = p(vec![I::Const(i64::MAX), I::Const(1), I::Add, I::Return]);
        let asm = emit_program(&program, AssemblyTarget::X86_64).unwrap();
        assert!(asm.contains("add rax, rcx\n    jo .Lpj_overflow"));
        assert!(asm.contains("jz .Lpj_fuel"));
        assert_eq!(emit_program(&program, AssemblyTarget::Arm), Err(NativeError::UnsupportedTarget));
    }
}
