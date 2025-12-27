"""
Payjar Language IDE
A professional IDE for the Payjar programming language with syntax highlighting,
file management, and code execution.
"""
import tkinter as tk
from tkinter import filedialog, messagebox, ttk
from tkinter import scrolledtext
import os
from pathlib import Path
from .esolangInter import PJS, PJRT
from .syntax import SyntaxHighlighter


class Window:
    """Main IDE window for Payjar language."""
    
    def __init__(self, w, h):
        self.root = tk.Tk()
        self.root.title("Payjar IDE - Professional Editor")
        self.root.geometry(f"{w}x{h}")
        self.root.minsize(800, 600)
        
        # State tracking
        self.current_file = None
        self.modified = False
        
        # Create the UI
        self._create_menu_bar()
        self._create_toolbar()
        self._create_main_paned_window()
        self._bind_shortcuts()
        
        self.root.mainloop()
    
    def _create_menu_bar(self):
        """Create the menu bar with File, Edit, Run, and Help menus."""
        menubar = tk.Menu(self.root)
        self.root.config(menu=menubar)
        
        # File menu
        file_menu = tk.Menu(menubar, tearoff=0)
        menubar.add_cascade(label="File", menu=file_menu)
        file_menu.add_command(label="New", command=self.new_file, accelerator="Ctrl+N")
        file_menu.add_command(label="Open", command=self.openFile, accelerator="Ctrl+O")
        file_menu.add_command(label="Save", command=self.saveFile, accelerator="Ctrl+S")
        file_menu.add_command(label="Save As", command=self.save_file_as)
        file_menu.add_separator()
        file_menu.add_command(label="Exit", command=self.root.quit)
        
        # Edit menu
        edit_menu = tk.Menu(menubar, tearoff=0)
        menubar.add_cascade(label="Edit", menu=edit_menu)
        edit_menu.add_command(label="Undo", command=lambda: self.text1.edit_undo(), accelerator="Ctrl+Z")
        edit_menu.add_command(label="Redo", command=lambda: self.text1.edit_redo(), accelerator="Ctrl+Y")
        edit_menu.add_separator()
        edit_menu.add_command(label="Cut", command=lambda: self._cut(), accelerator="Ctrl+X")
        edit_menu.add_command(label="Copy", command=lambda: self._copy(), accelerator="Ctrl+C")
        edit_menu.add_command(label="Paste", command=lambda: self._paste(), accelerator="Ctrl+V")
        edit_menu.add_separator()
        edit_menu.add_command(label="Select All", command=lambda: self._select_all(), accelerator="Ctrl+A")
        
        # Run menu
        run_menu = tk.Menu(menubar, tearoff=0)
        menubar.add_cascade(label="Run", menu=run_menu)
        run_menu.add_command(label="Execute Code", command=self.run, accelerator="Ctrl+R")
        run_menu.add_command(label="Clear Output", command=self.clear_output)
        
        # Help menu
        help_menu = tk.Menu(menubar, tearoff=0)
        menubar.add_cascade(label="Help", menu=help_menu)
        help_menu.add_command(label="About", command=self._show_about)
    
    def _create_toolbar(self):
        """Create a toolbar with quick action buttons."""
        toolbar = ttk.Frame(self.root)
        toolbar.pack(side=tk.TOP, fill=tk.X, padx=5, pady=5)
        
        ttk.Button(toolbar, text="📁 New", command=self.new_file).pack(side=tk.LEFT, padx=2)
        ttk.Button(toolbar, text="📂 Open", command=self.openFile).pack(side=tk.LEFT, padx=2)
        ttk.Button(toolbar, text="💾 Save", command=self.saveFile).pack(side=tk.LEFT, padx=2)
        
        ttk.Separator(toolbar, orient=tk.VERTICAL).pack(side=tk.LEFT, fill=tk.Y, padx=5)
        
        ttk.Button(toolbar, text="▶️  Run", command=self.run).pack(side=tk.LEFT, padx=2)
        ttk.Button(toolbar, text="🗑️  Clear", command=self.clear_output).pack(side=tk.LEFT, padx=2)
        
        # Status label
        self.status_label = ttk.Label(toolbar, text="Ready", relief=tk.SUNKEN)
        self.status_label.pack(side=tk.RIGHT, padx=5)
    
    def _create_main_paned_window(self):
        """Create the main editor and output pane."""
        paned = ttk.PanedWindow(self.root, orient=tk.VERTICAL)
        paned.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        # Editor pane with line numbers concept
        editor_frame = ttk.Frame(paned)
        paned.add(editor_frame, weight=3)
        
        ttk.Label(editor_frame, text="Code Editor:", font=("Arial", 10, "bold")).pack(anchor="w")
        self.text1 = scrolledtext.ScrolledText(
            editor_frame, wrap=tk.WORD, width=100, height=30,
            font=("Consolas", 10), bg="#f5f5f5"
        )
        self.text1.pack(fill=tk.BOTH, expand=True)
        self.text1.focus()
        
        # Attach syntax highlighter
        try:
            self.highlighter = SyntaxHighlighter(self.text1)
        except Exception as e:
            self.highlighter = None
            print(f"Syntax highlighter disabled: {e}")
        
        # Input/Output pane
        io_frame = ttk.Frame(paned)
        paned.add(io_frame, weight=1)
        
        io_notebook = ttk.Notebook(io_frame)
        io_notebook.pack(fill=tk.BOTH, expand=True)
        
        # Input tab
        input_frame = ttk.Frame(io_notebook)
        io_notebook.add(input_frame, text="Input")
        ttk.Label(input_frame, text="Program Input:", font=("Arial", 9, "bold")).pack(anchor="w", padx=5, pady=2)
        self.text_input = scrolledtext.ScrolledText(
            input_frame, wrap=tk.WORD, height=6,
            font=("Consolas", 9), bg="#fff"
        )
        self.text_input.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
        
        # Output tab
        output_frame = ttk.Frame(io_notebook)
        io_notebook.add(output_frame, text="Output")
        ttk.Label(output_frame, text="Program Output:", font=("Arial", 9, "bold")).pack(anchor="w", padx=5, pady=2)
        self.text2 = scrolledtext.ScrolledText(
            output_frame, wrap=tk.WORD, height=6,
            font=("Consolas", 9), bg="#f0f0f0", state=tk.DISABLED
        )
        self.text2.pack(fill=tk.BOTH, expand=True, padx=5, pady=5)
    
    def _bind_shortcuts(self):
        """Bind keyboard shortcuts."""
        self.root.bind('<Control-s>', lambda e: self.saveFile())
        self.root.bind('<Control-o>', lambda e: self.openFile())
        self.root.bind('<Control-r>', lambda e: self.run())
        self.root.bind('<Control-n>', lambda e: self.new_file())
        self.root.bind('<Control-a>', lambda e: self._select_all())
    
    def new_file(self):
        """Create a new file."""
        if self.modified:
            response = messagebox.askyesnocancel("Unsaved Changes", "Save changes before creating a new file?")
            if response is None:
                return
            elif response:
                self.saveFile()
        
        self.text1.delete('1.0', tk.END)
        self.text_input.delete('1.0', tk.END)
        self.current_file = None
        self.modified = False
        self._update_status("New file created")
    
    def run(self):
        """Execute the code in the editor."""
        code = self.text1.get("1.0", "end-1c")
        self.text2.config(state=tk.NORMAL)
        self.text2.delete('1.0', tk.END)
        self.text2.insert(tk.END, "Executing...\n")
        self.root.update()
        
        try:
            # Run PJS with code and input
            result = PJS(code, self.root)
            self.text2.delete('1.0', tk.END)
            self.text2.insert(tk.END, f"=== PJS Output ===\n{result}\n")
            self._update_status("Code executed successfully")
        except Exception as e:
            self.text2.delete('1.0', tk.END)
            self.text2.insert(tk.END, f"Error in PJS:\n{str(e)}\n")
            self._update_status(f"Error: {str(e)[:50]}")
        
        try:
            # Also run PJRT as a secondary check
            PJRT(code, 1)
        except Exception:
            # PJRT errors are non-fatal
            pass
        
        self.text2.config(state=tk.DISABLED)
    
    def saveFile(self):
        """Save the current file."""
        if self.current_file:
            self._save_to_path(self.current_file)
        else:
            self.save_file_as()
    
    def save_file_as(self):
        """Save the file with a new name."""
        filePath = filedialog.asksaveasfilename(
            title='Save file as',
            defaultextension='.pj',
            filetypes=[
                ('Payjar files', '*.pj'),
                ('Text files', '*.txt'),
                ('Markdown files', '*.md'),
                ('All files', '*.*')
            ],
            initialdir=str(Path.home())
        )
        if filePath:
            self.current_file = filePath
            self._save_to_path(filePath)
    
    def _save_to_path(self, filePath):
        """Helper to save content to a file."""
        try:
            content = self.text1.get('1.0', tk.END)
            with open(filePath, 'w') as f:
                f.write(content)
            self.modified = False
            self._update_status(f"Saved: {Path(filePath).name}")
        except Exception as e:
            messagebox.showerror("Save Error", f"Could not save file:\n{str(e)}")
            self._update_status(f"Save failed: {str(e)[:30]}")
    
    def openFile(self):
        """Open a file."""
        filePath = filedialog.askopenfilename(
            title="Open file",
            filetypes=[
                ("All files", "*.*"),
                ("Payjar files", "*.pj"),
                ("Text files", "*.txt"),
                ("Markdown files", "*.md")
            ],
            initialdir=str(Path.home())
        )
        if filePath:
            try:
                with open(filePath, 'r') as file:
                    content = file.read()
                    self.text1.delete('1.0', tk.END)
                    self.text1.insert(tk.END, content)
                    self.current_file = filePath
                    self.modified = False
                    self._update_status(f"Opened: {Path(filePath).name}")
            except Exception as e:
                messagebox.showerror("Open Error", f"Could not open file:\n{str(e)}")
                self._update_status(f"Open failed: {str(e)[:30]}")
    
    def clear_output(self):
        """Clear the output pane."""
        self.text2.config(state=tk.NORMAL)
        self.text2.delete('1.0', tk.END)
        self.text2.config(state=tk.DISABLED)
        self._update_status("Output cleared")
    
    def _cut(self):
        """Cut selected text."""
        try:
            self.text1.event_generate("<<Cut>>")
        except:
            pass
    
    def _copy(self):
        """Copy selected text."""
        try:
            self.text1.event_generate("<<Copy>>")
        except:
            pass
    
    def _paste(self):
        """Paste text."""
        try:
            self.text1.event_generate("<<Paste>>")
        except:
            pass
    
    def _select_all(self):
        """Select all text in editor."""
        self.text1.tag_add(tk.SEL, "1.0", tk.END)
        self.text1.mark_set(tk.INSERT, "1.0")
        self.text1.see(tk.INSERT)
        return 'break'
    
    def _update_status(self, message):
        """Update the status bar."""
        self.status_label.config(text=message)
    
    def _show_about(self):
        """Show about dialog."""
        messagebox.showinfo(
            "About Payjar IDE",
            "Payjar IDE v1.0\n\n"
            "A professional editor for the Payjar programming language.\n\n"
            "Features:\n"
            "• Syntax highlighting\n"
            "• File management (Open/Save)\n"
            "• Code execution\n"
            "• Input/Output panels\n\n"
            "Keyboard Shortcuts:\n"
            "Ctrl+N - New file\n"
            "Ctrl+O - Open file\n"
            "Ctrl+S - Save file\n"
            "Ctrl+R - Run code\n"
            "Ctrl+A - Select all"
        )