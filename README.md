# PayJar

<div align="center">
  <img src="payjar-logo.png" alt="Payjar logo" width="200" />

  # Payjar

  **A modern programming language.**
</div>

PayJar is a simple, lightweight interpreter for the **Payjar** programming language. The interpreter core is implemented in **Rust** for improved performance and memory safety. The language is designed to be familiar to Java/JavaScript developers with a minimal syntax, modern features, and a tiny runtime.

The repository also contains a React‑based IDE (`react-ide/`) that can be used to edit and run PayJar programs in the browser or as a Tauri desktop application.

---

## Language Features

- Class‑based, public/private methods, `this`/`self` support
- `main` entry point with `public class main(@self){ ... }`
- `let`/`const`/`var` declarations with block scoping
- Arithmetic operators (`+ - * / %`) and full comparison set (`== != < <= > >=`)
- `if` / `else` / `else if`, `while` loops, and C‑style `for` plus `for‑in` / `for‑range`
- First‑class functions with `def` and parameter lists
- Template strings (backtick literals) and normal string literals
- `println` / `print` and `readln` / `readi` / `readf` for I/O
- Lists, index access/assignment, and object creation with `new`

The language is intentionally small but expressive enough to build non‑trivial examples (see `examples/`).

---

## Built‑in Functions

| Function | Description |
|---|---|
| `print` | Print without newline |
| `println` | Print with newline |
| `printerr` | Print to stderr |
| `readln` | Read a line of input |
| `readi` | Read an integer from input |
| `readf` | Read a float from input |
| `toStr` | Convert value to string |
| `toInt` | Convert value to integer |
| `toFloat` | Convert value to float |
| `strLen` | Get string length |
| `charAt` | Get character at index |
| `strSlice` | Slice a string |
| `strContains` | Check if string contains substring |
| `strSplit` | Split a string by delimiter |
| `strTrim` | Trim whitespace |
| `strStartsWith` | Check string prefix |
| `strEndsWith` | Check string suffix |
| `readFile` | Read contents of a file |
| `writeFile` | Write contents to a file |
| `appendFile` | Append to a file |
| `exit` | Exit the program |
| `range` | Generate a numeric range |
| `typeOf` | Get the type of a value |
| `dir` | List visible names or an object's fields and methods |
| `indexOf` | Get index of element in list/string |
| `len` | Get length of list or string |
| `pow` | Raise a number to a power |

---

## Built‑in Modules

PayJar includes the following modules accessed via dot notation (e.g., `math.sin(x)`, `json.parse(str)`, etc.):

| Module | Description |
|---|---|
| `math` | Mathematical constants (pi, e, tau) and functions (sin, cos, sqrt, pow, etc.) |
| `cmath` | Complex number operations (complex, real, imag, abs, phase, polar, etc.) |
| `json` | JSON parsing and serialization (parse, stringify, readFile, writeFile) |
| `os` | Operating system functions (getcwd, chdir, listdir, mkdir, rename, etc.) |
| `sys` | System information (platform, arch, argv, exit, version, stdin, stdout, stderr) |
| `time` | Time and date operations (now, nowMs, sleep, sleepMs, format) |
| `base64` | Base64 encoding and decoding (encode, decode, encodeBytes) |
| `hash` | Cryptographic hashing (sha256, sha512, md5, sha256File) |
| `regex` | Regular expressions (test, match, matchGroup, findAll, replace, split, valid) |
| `random` | Random number generation (randint, random, randFloat, choice, shuffle) |
| `gui` | Graphical user interface (alert, confirm, prompt, notify) |
| `sqlite3` | SQLite database operations |
| `io` | Input/output functions (print, println, readln, readi, readf) |

---

## Runtime performance

The interpreter always applies a small, semantics-preserving integer constant-folding fast path. Pure integer expressions using `+`, `-`, and `*` are evaluated directly without allocating intermediate runtime values. Expressions involving variables, calls, floats, unsupported operators, or integer overflow continue through the normal interpreter, preserving its existing behavior (including integer-to-float promotion on overflow).

This optimization is automatic; no configuration change or environment variable is required. The former per-expression Cranelift JIT setup was removed because compiling a fresh native function for each tiny expression cost more than simply evaluating the expression.

Build with `cargo build --release`.

---

## Native assembly backend (experimental)

PayJar has a shared assembly-code-generation foundation in
`src/payjar/asm.rs`. It can emit GNU assembler source for x86-64 and AArch64
from pure integer-literal expressions using `+`, `-`, and `*`.

The generated function returns an `i64` using the target ABI's return
register (RAX on x86-64, X0 on AArch64). Unsupported AST nodes and integer
overflow are rejected so the existing interpreter remains the fallback.

A shared experimental bytecode VM lives in `src/payjar/asm_vm.rs`. It
supports integer locals, arithmetic/comparisons, `if`/`else`, `while`,
and `return`, with a portable reference executor. Its local-frame and
integer semantics are intentionally limited; unsupported programs must
continue through the existing interpreter.

This is still **not** native execution of the bytecode and is not yet a
replacement for the full interpreter or an executable native JIT. The broader
implementation plan, architecture notes, and milestones are in
[ASSEMBLY_BACKEND.md](ASSEMBLY_BACKEND.md).

## Building the Interpreter

Requires Rust toolchain (`rustup`/`cargo`):

```bash
cargo build --release
```

The binary will be placed at `target/release/Payjar`.

---

## Running Programs

Run a `.pj` file directly:

```bash
./target/release/Payjar examples/hello.pj
```

## Examples

The `examples/` directory contains sample programs:

- `hello.pj` – a "Hello, world!" program demonstrating basic syntax.
- `advanced.pj` – showcases classes, loops, I/O, and other language constructs.
- `examples/pack/` – a package example.

Feel free to modify or add your own `.pj` files and run them with `Payjar`.

---

## React IDE

The subdirectory `react-ide/` contains a browser‑based editor built with React and Monaco. To launch it:

```bash
cd react-ide
npm install
npm run dev
```

The IDE supports syntax highlighting and live execution via the PayJar interpreter.

See `react-ide/README.md` for more details.

---

## Changelog

See `CHANGELOG.md` for version history.

---

## License

PayJar is released under the MIT License. See `LICENSE` for full terms.
