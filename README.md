# Payjar IDE - Professional Editor

A feature-rich Python IDE for the **Payjar programming language** with modern UI, syntax highlighting, and built-in execution.

## Features

✨ **Professional IDE Features:**
- **Code Editor** with syntax highlighting for Payjar keywords and built-ins
- **Tabbed Input/Output** panels for program I/O
- **File Management** - Create, Open, and Save Payjar files
- **Code Execution** - Run code directly from the editor
- **Menu Bar** - Organize operations by File, Edit, Run, and Help
- **Toolbar** - Quick access to common actions
- **Keyboard Shortcuts** - Fast navigation and commands
- **Status Bar** - Real-time feedback on operations

## Getting Started

### Launch the IDE

**Option 1: Direct Python Script** (Recommended)
```bash
python3 payjar-ide.py
```

**Option 2: Python Module**
```bash
python3 -m Payjarnref
```

**Option 3: From Python Code**
```python
from Payjarnref import createWindow
createWindow(1200, 800)  # width, height
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| **Ctrl+N** | New file |
| **Ctrl+O** | Open file |
| **Ctrl+S** | Save file |
| **Ctrl+R** | Run code |
| **Ctrl+A** | Select all |
| **Ctrl+X** | Cut |
| **Ctrl+C** | Copy |
| **Ctrl+V** | Paste |
| **Ctrl+Z** | Undo |
| **Ctrl+Y** | Redo |

## Menu Guide

### File Menu
- **New** - Create a new file (Ctrl+N)
- **Open** - Open an existing file (Ctrl+O)
- **Save** - Save the current file (Ctrl+S)
- **Save As** - Save with a new filename
- **Exit** - Close the IDE

### Edit Menu
- **Undo/Redo** - Standard undo/redo operations
- **Cut/Copy/Paste** - Standard editing operations
- **Select All** - Select all text in editor (Ctrl+A)

### Run Menu
- **Execute Code** - Run the Payjar code (Ctrl+R)
- **Clear Output** - Clear the output panel

### Help Menu
- **About** - Show IDE version and features

## Supported File Types

- **Payjar Files** (`.pj`) - Native Payjar programs
- **Text Files** (`.txt`) - Plain text
- **Markdown Files** (`.md`) - Documentation
- **All Files** - Any file type

## IDE Layout

```
┌─────────────────────────────────────────┐
│  File  Edit  Run  Help                  │
├─────────────────────────────────────────┤
│  [📁 New] [📂 Open] [💾 Save] | ▶️ Run  │
├─────────────────────────────────────────┤
│                                         │
│  Code Editor (Main)                     │
│  - Syntax highlighting                  │
│  - Line wrapping                        │
│  - Scrollable                           │
│                                         │
├─────────────────────────────────────────┤
│  [Input] | [Output]                     │
│                                         │
│  Input/Output Tabs                      │
│  - Program input                        │
│  - Execution output                     │
│                                         │
└─────────────────────────────────────────┘
```

## Syntax Highlighting

The IDE automatically highlights Payjar language syntax:

- **Keywords**: `func`, `public`, `private`, `class`, `new`, `@`, `self`, `var`, `let`, `const`, `return`
- **Built-ins**: `readln`, `println`
- **Strings**: Single `'`, double `"`, and backtick `` ` ``
- **Comments**: `//` line comments and `/* */` block comments
- **Numbers**: Integer and float literals

## Code Execution

1. Write or paste Payjar code in the editor
2. (Optional) Add program input in the **Input** tab
3. Press **Ctrl+R** or click **Run** to execute
4. View output in the **Output** tab

## Troubleshooting

### Syntax Highlighter Issues
If syntax highlighting doesn't appear, the IDE continues to work normally. This may happen on systems with limited Tk font support.

### File Dialog Not Responding
The file open/save dialogs use the system's native dialog. They may take a moment to appear on first use.

### Code Execution Errors
Errors in Payjar code are displayed in the **Output** panel with error messages to help debugging.

## Architecture

```
Payjarnref/
├── __init__.py           # Package entry point & createWindow()
├── __main__.py           # Module entry point (python -m Payjarnref)
├── ui.py                 # Main IDE window & editor UI
├── syntax.py             # Syntax highlighter
├── esolangInter.py       # Payjar language interpreter
└── ...

payjar-ide.py            # Standalone launcher script
```

## Requirements

- Python 3.7+
- tkinter (usually included with Python)
- No external dependencies!

## Development

To modify the IDE:

1. **UI Layout**: Edit `Payjarnref/ui.py` - `_create_main_paned_window()`
2. **Syntax Highlighting**: Update `Payjarnref/syntax.py` - keyword definitions
3. **Language Features**: Modify `Payjarnref/esolangInter.py` - interpreter logic
4. **Entry Points**: Update `Payjarnref/__main__.py` or `payjar-ide.py`

## Future Enhancements

- [ ] Project explorer/file tree view
- [ ] Debug mode with breakpoints
- [ ] Language server protocol (LSP) support
- [ ] Theme customization (light/dark mode)
- [ ] Code completion and IntelliSense
- [ ] Plugin system
- [ ] Web-based IDE (Tauri integration)
- [ ] Collaborative editing

## License

Open source - Payjar IDE

---

**Happy coding with Payjar!** 🚀
=======
# Payjar 
wip
>>>>>>> 1874ff2b74a13adf17314473452a305958ba4d44
