# PayJar

PayJar is a simple, lightweight interpreter for the **Paygar** programming language. The interpreter core is implemented in **Rust** for improved performance and memory safety. The language is designed to be familiar to Java/JavaScript developers with a minimal syntax, modern features, and a tiny runtime.

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
- Packages with simple file loading via `pjrt`

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
| `strSplit` | Split string by delimiter |
| `strTrim` | Trim whitespace |
| `strStartsWith` | Check string prefix |
| `strEndsWith` | Check string suffix |
| `readFile` | Read contents of a file |
| `writeFile` | Write contents to a file |
| `appendFile` | Append to a file |
| `exit` | Exit the program |
| `range` | Generate a numeric range |
| `typeOf` | Get the type of a value |
| `indexOf` | Get index of element in list/string |
| `len` | Get length of list or string |
| `pow` | Raise a number to a power |

---

## Building the Interpreter

### Rust

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

Feel free to modify or add your own `.pj` files and run them with `Payjar` .

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

See [CHANGELOG.md](CHANGELOG.md) for version history.

---

## License

PayJar is released under the MIT License. See `LICENSE` for full terms.
