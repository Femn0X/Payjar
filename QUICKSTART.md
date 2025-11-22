# Payjar IDE - Quick Start Guide

## 🚀 Launch the IDE in 3 Steps

### Step 1: Open Terminal
Navigate to the project directory:
```bash
cd /home/lucawinecker/Payjar-ide
```

### Step 2: Start the IDE
Run the launcher:
```bash
python3 payjar-ide.py
```

### Step 3: Start Coding!
Write Payjar code in the editor and press **Ctrl+R** to execute.

---

## 📝 IDE Overview

### Main Components

1. **Menu Bar** - File, Edit, Run, Help menus
2. **Toolbar** - Quick buttons for New, Open, Save, Run, Clear
3. **Code Editor** - Main editing area with syntax highlighting
4. **Input Tab** - Enter program input here
5. **Output Tab** - View program output here
6. **Status Bar** - Shows current operation status

### Important Features

| Feature | Keyboard | Menu |
|---------|----------|------|
| New File | Ctrl+N | File → New |
| Open File | Ctrl+O | File → Open |
| Save File | Ctrl+S | File → Save |
| Run Code | Ctrl+R | Run → Execute Code |
| Select All | Ctrl+A | Edit → Select All |

---

## 💡 Example: Your First Payjar Program

1. **Create a new file** (Ctrl+N)
2. **Type this code:**
```payjar
func greet(name) {
    println "Hello, " + name + "!"
}

greet("Payjar Developer")
```

3. **Run it** (Ctrl+R)
4. **See output** in the Output tab: `Hello, Payjar Developer!`

---

## 📂 Working with Files

### Opening Files
- Use **Ctrl+O** or **File → Open**
- Supports: `.pj` (Payjar), `.txt`, `.md`, and all file types

### Saving Files
- Use **Ctrl+S** to save or **File → Save As** for new name
- Default extension is `.pj` for Payjar files

### File Locations
Sample programs are in `examples/`:
- `hello.pj` - Simple hello world
- `advanced.pj` - Variables and control flow

---

## 🎯 Pro Tips

### Keyboard Efficiency
- Use **Ctrl+Z** / **Ctrl+Y** for undo/redo
- Use **Ctrl+X/C/V** for cut/copy/paste
- Use **Ctrl+A** to select all code

### Quick Workflow
1. Write code in the editor
2. Add test input in the **Input** tab
3. Press **Ctrl+R** to execute
4. Check **Output** tab for results
5. Use **Ctrl+S** to save

### Debugging
- Run code with **Ctrl+R**
- Check **Output** tab for error messages
- Fix errors and run again

---

## ⚙️ Settings & Customization

### Font
The editor uses Consolas font at size 10. To change, edit `Payjarnref/ui.py`:
```python
font=("Consolas", 10)  # Change font name or size
```

### Window Size
Modify `payjar-ide.py` or `Payjarnref/__main__.py`:
```python
createWindow(1200, 800)  # width, height in pixels
```

### Syntax Highlighting Colors
Edit `Payjarnref/syntax.py` to customize colors:
```python
self.text.tag_configure("keyword", foreground="#00f")  # Blue
```

---

## 🐛 Troubleshooting

### IDE Won't Launch
- Ensure Python 3.7+ is installed: `python3 --version`
- Check tkinter is available: `python3 -m tkinter`
- Try alternative launch: `python3 -m Payjarnref`

### Syntax Highlighting Not Working
- IDE still works normally
- This might happen on systems with limited Tk font support
- Try changing the font in `ui.py`

### File Dialog Issues
- Native system dialogs may be slow on first use
- They save the last directory used
- Start in your home directory (`~`)

### Code Execution Errors
- Errors are displayed in the **Output** tab
- Check for syntax issues in your Payjar code
- Refer to Payjar language documentation

---

## 📚 Learn More

### Documentation
- See `IDE_README.md` for detailed features
- Check language syntax in `Payjarnref/syntax.py`

### Package Structure
```
Payjarnref/
├── ui.py           ← IDE window and editor
├── syntax.py       ← Syntax highlighting
├── esolangInter.py ← Language interpreter
└── __main__.py     ← Module entry point
```

### Modify the IDE
All code is well-documented. Feel free to:
- Add menu items or toolbar buttons
- Customize colors and fonts
- Extend syntax highlighting
- Add new editor features

---

## 🎉 You're Ready!

Your professional Payjar IDE is now set up. Happy coding! 🚀

For help:
- Use **Help → About** in the IDE
- Check this guide
- Review `IDE_README.md`

---

**Payjar IDE v1.0** | A professional editor for the Payjar programming language
