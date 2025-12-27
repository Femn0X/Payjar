# syntax.py
import re
import tkinter as tk
import tkinter.font as tkfont

KEYWORDS = [
    "func", "public", "class", "new",
    "@", "self", "inner_self", "const", "var", "let", "return"
]

BUILTINS = ["readln", "println"]

COMMENT_PATTERN = r"(?://.*|/\*.*?\*/)"
STRING_PATTERN = r"(?:'[^']*'|\"[^\"]*\"|`[^`]*`)"
NUMBER_PATTERN = r"\b\d+(?:\.\d+)?\b"


class SyntaxHighlighter:
    def __init__(self, text_widget: tk.Text):
        self.text = text_widget
        self._setup_tags()
        self.text.bind("<KeyRelease>", self._on_key_release)

    def _setup_tags(self):
        """Define visual tags for syntax elements."""
        # Try to use system text font, fallback to Consolas 12
        try:
            default_font = tkfont.nametofont("TkTextFont")
        except tk.TclError:
            default_font = tkfont.Font(family="Consolas", size=12)

        italic_font = default_font.copy()
        italic_font.configure(slant="italic")

        self.text.tag_configure("keyword", foreground="#00f")
        self.text.tag_configure("builtin", foreground="#f0f")
        self.text.tag_configure("string", foreground="#f00")
        self.text.tag_configure("comment", foreground="#0f0", font=italic_font)
        self.text.tag_configure("number", foreground="#777")

    def _on_key_release(self, event=None):
        self.highlight()

    def highlight(self):
        """Apply syntax highlighting to the text widget."""
        content = self.text.get("1.0", "end-1c")

        for tag in self.text.tag_names():
            self.text.tag_remove(tag, "1.0", "end")

        for match in re.finditer(COMMENT_PATTERN, content):
            self._apply_tag(match, "comment")

        for match in re.finditer(STRING_PATTERN, content):
            self._apply_tag(match, "string")

        for match in re.finditer(NUMBER_PATTERN, content):
            self._apply_tag(match, "number")

        for word in KEYWORDS:
            for match in re.finditer(rf"\b{re.escape(word)}\b", content):
                self._apply_tag(match, "keyword")

        for func in BUILTINS:
            for match in re.finditer(rf"\b{func}\b", content):
                self._apply_tag(match, "builtin")

    def _apply_tag(self, match, tag_name):
        start_idx = f"1.0+{match.start()}c"
        end_idx = f"1.0+{match.end()}c"
        self.text.tag_add(tag_name, start_idx, end_idx)
