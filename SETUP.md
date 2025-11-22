# Payjar IDE - Setup & Installation Guide

## ✅ Installation Complete!

Your **Payjar IDE v1.0** is fully set up and ready to use.

---

## 🚀 How to Launch

### Method 1: Direct Script (Recommended)
```bash
python3 payjar-ide.py
```

### Method 2: Python Module
```bash
python3 -m Payjarnref
```

### Method 3: From Python
```python
from Payjarnref import createWindow
createWindow(1200, 800)
```

---

## 📋 What's Included

### IDE Features
✨ Modern Tkinter-based code editor
✨ Syntax highlighting for Payjar language
✨ File management (Open/Save/New)
✨ Built-in code execution
✨ Tabbed Input/Output panels
✨ Full menu bar and toolbar
✨ 10+ keyboard shortcuts
✨ Status bar with operation feedback

### Files Created
```
payjar-ide.py              ← Main launcher script
IDE_README.md              ← Full IDE documentation
QUICKSTART.md              ← Quick start guide
examples/
├── hello.pj              ← Simple example
└── advanced.pj           ← Advanced example
```

### Package Structure (Unchanged)
```
Payjarnref/
├── __init__.py           ← Package entry point
├── __main__.py           ← Module launcher
├── ui.py                 ← Enhanced IDE UI (UPDATED)
├── syntax.py             ← Syntax highlighter
├── esolangInter.py       ← Language interpreter
├── setup.py              ← Package setup
└── pyjroject.toml        ← Project metadata
```

---

## 🎯 Quick Reference

### Keyboard Shortcuts
```
Ctrl+N  → New file
Ctrl+O  → Open file
Ctrl+S  → Save file
Ctrl+R  → Run code
Ctrl+A  → Select all
Ctrl+Z  → Undo
Ctrl+Y  → Redo
Ctrl+X  → Cut
Ctrl+C  → Copy
Ctrl+V  → Paste
```

### Menu Structure
```
File    → New, Open, Save, Save As, Exit
Edit    → Undo, Redo, Cut, Copy, Paste, Select All
Run     → Execute Code, Clear Output
Help    → About
```

---

## 📊 System Requirements

- **Python**: 3.7 or higher
- **Dependencies**: tkinter (included with Python)
- **OS**: Windows, macOS, Linux
- **RAM**: 100 MB minimum
- **Disk**: 5 MB for IDE files

### Check Your System
```bash
python3 --version              # Should show 3.7+
python3 -m tkinter             # Should open a window
```

---

## 🔧 Configuration

### Change Window Size
Edit `payjar-ide.py` line 10:
```python
createWindow(1200, 800)  # width, height
```

### Change Font
Edit `Payjarnref/ui.py` line 93:
```python
font=("Consolas", 10)  # (font_name, size)
```

### Change Colors
Edit `Payjarnref/syntax.py` line 24-29:
```python
self.text.tag_configure("keyword", foreground="#00f")  # Blue
```

---

## 📚 Documentation

| File | Purpose |
|------|---------|
| `IDE_README.md` | Complete feature documentation |
| `QUICKSTART.md` | Quick start for new users |
| This file | Installation and setup |

---

## ✨ Features Overview

### Editor
- Syntax highlighting for Payjar
- Line wrapping and word wrap
- Scrollbars for navigation
- Context-aware editing

### Execution
- Real-time code execution
- Program input support
- Error reporting
- Output display

### File Management
- Create new files
- Open existing files
- Save with custom names
- Support multiple file types (`.pj`, `.txt`, `.md`)

### User Interface
- Professional menu bar
- Quick-access toolbar
- Tabbed I/O panels
- Status bar feedback
- Keyboard shortcuts

---

## 🎓 Getting Started

### 1. First Launch
```bash
python3 payjar-ide.py
```

### 2. Create Your First Program
- Press **Ctrl+N** for new file
- Type your Payjar code
- Press **Ctrl+R** to run
- Check output in the **Output** tab

### 3. Open an Example
- Press **Ctrl+O**
- Navigate to `examples/`
- Open `hello.pj`
- Press **Ctrl+R** to see it work

### 4. Save Your Work
- Press **Ctrl+S**
- Choose filename and location
- Code is saved automatically

---

## 🐛 Troubleshooting

### Issue: IDE won't start
**Solution:**
```bash
python3 --version  # Check Python version (need 3.7+)
python3 -m tkinter # Test tkinter installation
```

### Issue: Syntax highlighting not showing
**Solution:** IDE still works! Highlighting disabled on some systems. Continue coding normally.

### Issue: File dialog not responding
**Solution:** Native dialogs may be slow. Wait a moment or try again.

### Issue: Code won't run
**Solution:** Check output tab for error messages. Verify Payjar syntax.

---

## 🔐 Security Note

The IDE runs Payjar code locally on your machine. Code execution happens within the Payjar interpreter with standard Python security.

---

## 📦 Distribution

To share this IDE with others:

1. **Package as Python module:**
   ```bash
   pip install -e .
   ```

2. **Share the directory:**
   ```bash
   git clone <repo>
   cd Payjar-ide
   python3 payjar-ide.py
   ```

3. **Create standalone executable:**
   ```bash
   pip install pyinstaller
   pyinstaller --onefile payjar-ide.py
   ```

---

## 🚀 Next Steps

1. **Read** `QUICKSTART.md` for quick reference
2. **Review** `IDE_README.md` for detailed features
3. **Try** the examples in `examples/` folder
4. **Write** your own Payjar programs
5. **Explore** menu items and shortcuts

---

## 💬 Support

For issues or questions:
1. Check the troubleshooting section above
2. Review documentation files
3. Examine IDE code in `Payjarnref/ui.py`
4. Check language specification in `Payjarnref/esolangInter.py`

---

## 📝 What Was Fixed/Added

### Fixed Issues
- ✅ Indentation error at ui.py line 27
- ✅ Fixed imports (package-relative)
- ✅ Proper `__init__` method structure

### Added Features
- ✅ Professional menu bar
- ✅ Toolbar with quick buttons
- ✅ Tabbed Input/Output panels
- ✅ Keyboard shortcuts (10+ shortcuts)
- ✅ Status bar with feedback
- ✅ Enhanced file dialog
- ✅ Better error handling
- ✅ Proper window management
- ✅ Code syntax highlighting integration
- ✅ Launch scripts and documentation

---

## 🎉 You're All Set!

Your professional Payjar IDE is ready to use. Enjoy coding! 🚀

**Version:** 1.0
**Created:** 2025-11-22
**Status:** Production Ready ✨
