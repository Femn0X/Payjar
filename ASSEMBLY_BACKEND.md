# PayJar native assembly backend

## Status

The first backend foundation lives in `src/payjar/asm.rs`. It emits GNU assembler
source for ELF/Linux targets from a deliberately narrow AST subset:

- integer literals;
- binary `+`, `-`, and `*`;
- checked i64 arithmetic only.

The emitted function is named `payjar_entry` and returns its result in RAX
(x86-64 System V) or X0 (AArch64/AAPCS64). Unsupported AST nodes and checked
integer overflow are reported to the caller so the existing interpreter can
remain the semantic fallback.

This is an assembly **source emitter**, not yet an executable JIT and not yet
a replacement for the complete tree-walking interpreter. No generated code is
executed by this module.

## Shared VM v1

The architecture-neutral bytecode layer is in `src/payjar/asm_vm.rs`.
It currently compiles a constrained AST subset into a stack-based instruction
stream and executes that stream in a portable VM:

- integer constants and local variables;
- integer `+`, `-`, `*` and comparisons;
- `if` / `else`, `while`, and `return`;
- checked arithmetic, explicit compile/runtime errors, and a configurable
  instruction-step limit.

The first VM version uses one flat local frame. Lexical shadowing, block-scope
lifetimes, floats, strings, collections, function calls, exceptions, and
built-ins are not yet supported. In particular, integer overflow returns a VM
error rather than silently changing PayJar's integer value into a float. The
caller must therefore fall back to the existing interpreter for unsupported
programs or VM errors.

Example:

```rust
use crate::payjar::asm_vm::{compile, Vm};

let program = compile(&ast.body)?;
let result = Vm::default().execute(&program)?;
```

This VM is a shared execution specification; it does not yet emit native
instructions from bytecode. The x86-64 and AArch64 assembly emitters remain
separate, earlier-stage source generators.

## Architecture

Both targets share the PayJar AST and the same validation/lowering decisions.
Target-specific code generation is isolated behind `AssemblyTarget`:

- `AssemblyTarget::X86_64`: GNU assembler Intel syntax, ELF symbol directives.
- `AssemblyTarget::Aarch64`: GNU assembler AArch64 syntax, ELF symbol directives.

The first implementation uses a simple stack-based expression lowering strategy
rather than register allocation. This keeps the initial code generator small
and makes evaluation order explicit.

## Planned milestones

1. **Assembly foundation** — shared target selection and testable x86-64/AArch64
   source emission for constant integer arithmetic. (Complete.)
2. **Shared VM v1** — architecture-neutral bytecode, local slots, arithmetic,
   comparisons, branches, loops, returns, and a portable reference executor.
   (Implemented experimentally; native lowering and full scope semantics remain.)
3. **Native x86-64/AArch64 lowering** — translate the shared VM instruction set
   into target-specific machine code/source and validate both against the VM.
4. **Runtime bridge** — PayJar values, strings, lists, objects, exceptions,
   built-ins, and module calls with interpreter-compatible semantics.
5. **Native JIT** — assemble/encode generated instructions, allocate executable
   memory safely, enforce W^X permissions, and manage code lifetime.
6. **Second architecture parity** — run the same conformance suite on x86-64
   and AArch64; add platform-specific ABI and object-format support.
7. **Optimization** — register allocation, liveness analysis, constant folding,
   and dead-code elimination, measured against the interpreter.

## API sketch

```rust
use crate::payjar::asm::{emit_integer_expression, AssemblyTarget};

let assembly = emit_integer_expression(&ast, AssemblyTarget::X86_64)?;
```

The caller chooses the target explicitly. Automatic host detection will be
added only when the runtime can guarantee that the generated object format,
ABI, and executable-memory policy match the host.
