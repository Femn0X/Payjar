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

## Architecture

Both targets share the PayJar AST and the same validation/lowering decisions.
Target-specific code generation is isolated behind `AssemblyTarget`:

- `AssemblyTarget::X86_64`: GNU assembler Intel syntax, ELF symbol directives.
- `AssemblyTarget::Aarch64`: GNU assembler AArch64 syntax, ELF symbol directives.

The first implementation uses a simple stack-based expression lowering strategy
rather than register allocation. This keeps the initial code generator small
and makes evaluation order explicit.

## Planned milestones

1. **Assembly foundation** — shared target selection, safe expression subset,
   and testable x86-64/AArch64 source emission. (In progress.)
2. **Native interpreter core** — define a stable PayJar VM instruction set and
   runtime calling convention; execute supported instructions through a small
   native runtime while unsupported language features remain on the Rust path.
3. **Full expression and statement lowering** — variables, comparisons,
   branches, loops, calls, and returns.
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
