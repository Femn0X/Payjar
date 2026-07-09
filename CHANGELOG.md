# Changelog

All notable changes to the PayJar project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---
## [1.4.0] - unreleased

## Added

- added package manager[repo alredy created]

##  Changed 

- N/A

## Changed

- N/A

## Fixed

- N/A

## [1.3.0] - 2026-07-08

## Added

- new net built-in for http/https networking — `https://` URLs now do a real TLS handshake (via `rustls`, verified against the OS trust store) instead of failing; `http://` still works over plain TCP
- Tkinter-style GUI window builder: `gui.window`, `gui.addLabel`, `gui.addButton`, `gui.addEntry`, `gui.addCheckbox`, `gui.addSeparator`, `gui.show` — build a whole form out of widgets and open it as a *single* window instead of one window per component; classic `gui.alert`/`confirm`/`prompt`/`notify` dialogs are unchanged
- top-level `def`/`class` (and `public class`) definitions are now allowed outside of `main`, in any order, before or after it
- class names can be used as variable/parameter/field types, Java-style, e.g. `let r:Rect = new Rect(10, 10);`
- package files (`package foo;`) can now `import` built-in modules (`math`, `net`, `gui`, `io`, `sqlite3`, `random`, `regex`, `os`, `sys`) and use them directly
- template string interpolation (`` `${...}` ``) now evaluates full expressions — function calls, indexing, arithmetic, etc. (e.g. `` `${square(4)}` ``, `` `${result["key"]}` ``) — not just bare variable/field names

## Removed

- N/A

## Changed

- Optimisted Payjar by editing Cargo.toml &  all files in src
- `rustls` dependency pinned to a real published version (`0.23`, ring backend) instead of a nonexistent `0.24.0-dev.0`

## Fixed

- fixed Type checking
- fixed `net.get`/`net.getStatus`/`net.post` argument handling (previously indexed past the given argument count and referenced an undefined variable, so HTTP requests could crash or fail to compile)

## [1.2.0] - 2025-05-25

## Added 

- buildin modules like sqlite3,math,os,csv

## Changed

- removed pjrt and "replaced" it with pagackes example :
```text
  <package-name>/
  |- init.pj #the file that gets imported
  |- ... more file
  |<file>.pj #can import <package-main>  
```

## fixed

- fixed import error for built-in modules
- fixed error when trying to run a file that use'S gui built-in

## [1.1.0] - 2026-05-09

### Added

- **Rust rewrite** — the interpreter core (`payjar.rs`) has been rewritten in Rust for improved performance, memory safety, and easier cross-platform distribution
- New built-ins: `printerr`, `strLen`, `charAt`, `strSlice`, `strContains`, `strSplit`, `strTrim`, `strStartsWith`, `strEndsWith`, `toFloat`, `typeOf`, `indexOf`, `pow`
- File I/O built-ins: `readFile`, `writeFile`, `appendFile`
- `exit` built-in for controlled program termination
- `range` built-in for generating numeric ranges

### Changed

- Interpreter binary is now compiled via `cargo build --release` instead of `make all`
- Updated README to document all built-in functions in a reference table
- `payjar-info.txt` expanded into a structured built-in reference with descriptions

### Fixed

- N/A

---

## [1.0.1]

### Added

- `for-in` and `for-range` loops for iterating over lists and ranges
- Template string support using backticks (`` ` ``) with embedded expressions
- Package loading logic (`pjrt`) capable of scanning directories for `.pj` files
- `readi` and `readf` built-ins for integer/float input
- Object field access and method lookup improvements
- `pjc` and `pjrt` utilities now respect `-d`/`--debug` flag

### Changed

- Parser rewritten to handle nested `else if` blocks and method definitions
- Runtime value system refactored with reference counting for better memory management
- Improved error messages during parsing and evaluation
- `makefile` updated to build both interpreter and runtime tools

### Fixed

- Crash when accessing undefined variables or list indices
- Scope resolution bugs with `let`/`const` declarations inside loops
- Several memory leaks in lexer and AST construction

---

## [1.0.0] - 2024-03-10

### Added

- Initial public release with basic interpreter supporting classes, functions, variables, I/O, arithmetic, conditionals, and loops
- Simple React‑based IDE for editing and running PayJar code

### Changed

- N/A

### Fixed

- N/A
