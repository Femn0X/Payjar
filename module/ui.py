import os
import tkinter as tk
from tkinter import filedialog, messagebox
from .esolangInter import PJS, PJRT
from .syntax import *


class Window:
    def __init__(self, w, h):
        self.root = tk.Tk()
        self.root.title('Payjar IDE')
        self.root.geometry(f"{w}x{h}")

        # Main editor
        # Enable widget-level undo so undo/redo work
        self.text1 = tk.Text(self.root, wrap='word', width=100, height=40, undo=True)
        self.text1.grid(row=0, column=0, sticky='nsew')

        # Bottom area used for input / output
        self.text2 = tk.Text(self.root, wrap='word', width=100, height=10, undo=True)
        self.text2.grid(row=1, column=0, sticky='nsew')

        # Simple button bar
        tk.Button(self.root, text="Save file", command=self.saveFile).grid(row=2, column=0, sticky='w', padx=4, pady=4)
        tk.Button(self.root, text='Run code (F5)', command=self.run).grid(row=2, column=0)
        tk.Button(self.root, text="Open file", command=self.openFile).grid(row=2, column=0, sticky='e', padx=4, pady=4)

        # Keyboard shortcuts (requested mappings)
        self.root.bind('<Control-n>', lambda e: self.new_file())
        self.root.bind('<Control-o>', lambda e: self.openFile())
        self.root.bind('<Control-s>', lambda e: self.saveFile())
        self.root.bind('<Control-r>', lambda e: self.run())
        self.root.bind('<Control-a>', lambda e: self.select_all())
        self.root.bind('<Control-z>', lambda e: self.undo())
        self.root.bind('<Control-y>', lambda e: self.redo())
        self.root.bind('<Control-x>', lambda e: self.cut())
        self.root.bind('<Control-c>', lambda e: self.copy())
        self.root.bind('<Control-v>', lambda e: self.paste())
        self.root.bind('<F5>', lambda e: self.run())

        # Make grid resize nicely
        self.root.grid_rowconfigure(0, weight=3)
        self.root.grid_rowconfigure(1, weight=1)
        self.root.grid_columnconfigure(0, weight=1)

        self.root.mainloop()

    def run(self):
        code = self.text1.get("1.0", "end-1c")
        user_input = self.text2.get('1.0', 'end-1c')
        try:
            # Clear the bottom area and show running message
            self.text2.delete('1.0', tk.END)
            self.text2.insert(tk.END, 'Running...\n')

            # PJS will use PJRT internally; use str() to show a readable output
            result = PJS(code, user_input)
            self.text2.delete('1.0', tk.END)
            try:
                self.text2.insert(tk.END, str(result))
            except Exception:
                # Fallback: show a minimal representation
                self.text2.insert(tk.END, repr(result))
        except Exception as e:
            messagebox.showerror('Run error', str(e))

    # --- Extra editor helpers for keyboard shortcuts ---
    def new_file(self):
        if messagebox.askyesno('New file', 'Clear editor? Unsaved changes will be lost.'):
            self.text1.delete('1.0', tk.END)
            self.text2.delete('1.0', tk.END)

    def select_all(self):
        widget = self.root.focus_get()
        if isinstance(widget, tk.Text):
            widget.tag_add(tk.SEL, '1.0', 'end-1c')
            widget.mark_set(tk.INSERT, '1.0')
            widget.see(tk.INSERT)

    def _focused_text_widget(self):
        w = self.root.focus_get()
        return w if isinstance(w, tk.Text) else self.text1

    def undo(self):
        w = self._focused_text_widget()
        try:
            w.edit_undo()
        except Exception:
            pass

    def redo(self):
        w = self._focused_text_widget()
        try:
            w.edit_redo()
        except Exception:
            pass

    def cut(self):
        w = self._focused_text_widget()
        try:
            w.event_generate('<<Cut>>')
        except Exception:
            pass

    def copy(self):
        w = self._focused_text_widget()
        try:
            w.event_generate('<<Copy>>')
        except Exception:
            pass

    def paste(self):
        w = self._focused_text_widget()
        try:
            w.event_generate('<<Paste>>')
        except Exception:
            pass

    def saveFile(self):
        filePath = filedialog.asksaveasfilename(
            title='Save file',
            defaultextension='.txt',
            filetypes=[('Text file', '*.txt'), ('Payjar file', '*.pj'), ('MD file', '*.md')],
            initialdir=os.path.expanduser('~')
        )
        if not filePath:
            return
        try:
            content = self.text1.get('1.0', tk.END)
            with open(filePath, 'w', encoding='utf-8') as outputFile:
                outputFile.write(content)
        except Exception as e:
            messagebox.showerror('Save error', str(e))

    def openFile(self):
        filePath = filedialog.askopenfilename(
            title="Select file",
            filetypes=[("All files", "*.*"), ("Text files", "*.txt"), ("Payjar files", "*.pj")],
            initialdir=os.path.expanduser('~')
        )
        if filePath:
            try:
                with open(filePath, 'r', encoding='utf-8') as file:
                    content = file.read()
                    self.text1.delete('1.0', tk.END)
                    self.text1.insert(tk.END, content)
            except Exception as e:
                messagebox.showerror('Open error', str(e))