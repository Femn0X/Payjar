Short guide for AI coding agents working on Payjar

Overview
- The repository contains two primary surfaces:
  - A Python-based minimal desktop IDE (`module/`, `Main.py`) using `tkinter` and a custom esolang runtime.
  - A web/desktop frontend scaffold (Tauri + React + Vite) described by `package.json` (Monaco editor + Tauri).

Key entry points
- Python IDE: run `python Main.py` or import `module` and call `module.createWindow(w,h)`.
  - `module/__init__.py` exposes `createWindow` -> returns `Window` from `module/ui.py`.
  - `module/ui.py` contains the `Window` class: UI layout, keyboard shortcuts, file dialogs, and the run loop.
    - `Window.run()` collects code from the editor and calls `PJS(code, user_input)` then writes output to the bottom pane.
- Language/runtime: `module/esolangInter.py` implements the Lexer, Parser, AST utilities and runtime helpers.
  - Public runtime entrypoints referenced by the UI are `PJS` and `PJRT` (look for these symbols when changing runtime behavior).
- Syntax/highlighting helpers: `module/syntax.py` contains `Logic` and `SytaxHighlight` classes used to derive token->color mappings.

Developer workflows
- Launch the Python IDE quickly: `python Main.py` (use a virtualenv with Python >= 3.8).
- Frontend/dev for the Tauri/React part: install Node, then `npm install` and `npm run dev` (see `package.json` scripts).
- Packaging metadata is in `py-project.toml` (setuptools backend). Use `python -m build` / `pip install -e .` if needed.

Project-specific patterns and conventions
- The esolang uses its own keywords mapping in the lexer: e.g. `func` -> `DEF`, `println` -> `PRINT`, `readln` -> `READLN`, `let/const/var` -> `LET/CONST/VAR`.
- Parser is a handwritten recursive-descent implementation with explicit token types; keep token names stable when changing lexer/parser.
- UI and runtime are coupled: `Window.run()` directly calls the runtime (`PJS`) with raw source and text input — avoid changing signatures without updating `ui.py`.
- Error handling is UI-centric: runtime/parse errors are often surfaced through `tkinter.messagebox` (see `ui.py`).
- Naming is inconsistent in places (e.g., `Windoow` / `Window`, `createWindow` returns a `Window`). When refactoring, update `module/__init__.py`, `Main.py`, and any tests or examples.

Files to inspect when working on a change
- UI / user flows: `module/ui.py`, `Main.py`, `module/__main__.py`.
- Language runtime / interpreter: `module/esolangInter.py` (Lexer/Parser/Runtime functions such as `PJS`, `PJRT`).
- Syntax highlighting: `module/syntax.py` (token -> color logic).
- Packaging & frontend: `py-project.toml`, `package.json`, and `setup.sh` for clone/install hints.

Testing and verification
- There are no formal pytest configurations in the repo. If adding tests, place them under `tests/` and run `pytest`.
- Manual verification for the IDE:
  - Start the Python UI: `python Main.py`.
  - Paste a small Payjar program into the editor, supply input in the bottom pane, press `F5` or click "Run code".
  - Inspect output and any messagebox errors.

Contributions and PR guidance for agents
- Preserve public APIs in `module/__init__.py` and `Main.py` unless you update all references.
- When modifying lexer/parser, include regression snippets (small source examples) demonstrating the intended behavior.
- For UI changes, test in the running `tkinter` window; small visual regressions are possible because layout is manual grid-based.

If something's unclear
- Ask for: which surface to prioritize (Python UI vs Tauri web UI), sample Payjar source to use as regression tests, or permission to run the UI in the container.

References
- Examples / quickstart: `QUICKSTART.md`, `INDEX.md`, `README.md`.
- Main Python files: `Main.py`, `module/__init__.py`, `module/ui.py`, `module/esolangInter.py`, `module/syntax.py`.

Known issues & recommended fixes
- **PJS runtime mismatch**: `module/esolangInter.py` defines `class PJS` (constructor `__init__(self, code, textin)`) while `module/ui.py` calls `PJS(code, user_input)` and treats the result as an immediately printable value. In the current implementation `PJS.__str__()` calls `interpret()` which itself doesn't reliably return a final value — this leads to confusing output and non-deterministic behavior.
  - Quick fixes:
    - Option A (recommended): Convert `PJS` into a functional API `def PJS(code, textin) -> str` that parses, interprets, and returns a string result. This keeps `ui.py` simple (no calling-site changes). Minimal shape:
      ```python
      # module/esolangInter.py (sketch)
      def PJS(code, textin):
          tokens, ast = PJRT(code, debug_on=1).run_code(0)
          comp = Comp(tokens, textin)
          compRe = comp.parse()
          interpreter = Interpreter()
          output = interpreter.interpret(ast)
          return str(output)  # deterministic string for the UI
      ```
    - Option B: Keep `PJS` as a class but make its instance return a clear value via an explicit method (e.g., `run()` or `result()`), and update `module/ui.py` to call that method instead of treating the instance as the result. Example UI change:
      ```python
      # in module/ui.py
      pjs = PJS(code, user_input)
      result = pjs.run()  # new method that returns deterministic output
      ```
  - What to verify after changes:
    - Running `python Main.py` with a simple Payjar program returns the expected string in the bottom pane.
    - Exceptions in parsing or runtime are propagated back to the UI and shown via `messagebox.showerror` (or captured and displayed in the output pane).

- **UI upgrade suggestions**: the current `Window.run()` calls the runtime synchronously on the main Tk thread which will block the UI for long-running programs and can deadlock on input. Recommended improvements:
  - Run the interpreter in a background thread (use `threading.Thread`) and update the Tk widgets with `tkinter`'s `after()` method to avoid cross-thread widget access.
  - Separate input vs output: keep a dedicated input field (Entry or small Text) and an output `Text` that appends results and preserves history.
  - Improve error handling: catch and format parse/runtime errors; include source location when available.
  - Add a simple status label (e.g., `Running...` / `Done`) and disable Run while active.

If any of these recommendations look good, I can implement either Option A or B for `PJS` and a minimal non-blocking `Window.run()` upgrade and submit a patch.

End of guide — ask for feedback to iterate.
