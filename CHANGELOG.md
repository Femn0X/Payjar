# Changelog

All notable changes to the PayJar project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [1.0.1]
### Added
- `for-in` and `for-range` loops for iterating over lists and ranges
- Template string support using backticks (`) with embedded expressions
- Package loading logic (`pjrt`) capable of scanning directories for `.pj` files
- `readi` and `readf` built‑ins for integer/float input
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

## [1.0.0] - 2024-03-10
### Added
- Initial public release with basic interpreter supporting classes, functions,
  variables, I/O, arithmetic, conditionals, and loops.
- Simple React‑based IDE for editing and running PayJar code.

### Changed
- N/A

### Fixed
- N/A
