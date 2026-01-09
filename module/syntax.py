<<<<<<< HEAD
from .esolangInter import *
from typing import List,Dict,Union
import tkinter as tk
"""WIP sytax highlighter for Payjar language"""
class Logic:
 def __init__(self,code):
    self.tokens=Lexer(code).tokenize()
    ast=Parser(self.tokens).parse()
 def __repr__(self):
    re=[]
    for token in self.tokens:
        re.append(self.tokenToHEX(token))
    return f':Logic{re}'
 def tokenToHEX(self,token):
    text='#fff'
    local_vars='#777'
    keyword='#00f'
    string='#f70' 
    brasces=[' #ff0','#f0f','#00f']
    class_clall='#0ff'
    coment='#0f0'
    i=-1
    if token.type=='PUBLIC':return {'type':'keyword','color':keyword}
    elif token.type=='PRIVATE':return {'type':'keyword','color':keyword}
    elif token.type=='CLASS':return {'type':'keyword','color':keyword}
    elif token.type=='MAIN':return {'type':'','color':class_clall}
    elif token.type=='SELF':return {'type':'','color':local_vars}
    elif token.type=='INNERSELF':return {'type':'','color':local_vars}
    elif token.type=='DEF':return {'type':'keyword','color':keyword}
    elif token.type=='PRINT':return  {'type':'text','color':text}
    elif token.type=='PASS':return {'type':'keyword','color':keyword}
    elif token.type=='LET':return {'type':'keyword','color':keyword}
    elif token.type=='CONST':return {'type':'keyword','color':keyword}
    elif token.type=='VAR':return {'type':'keyword','color':keyword}
    elif token.type=='NEW':return {'type':'keyword','color':keyword}
    elif token.type=='READLN':return {'type':'text','color':text}
    elif token.type=='RETURN':return {'type':'keyword','color':keyword}
    elif token.type=='IDENTIFIER':{'type':'','color':''}
    elif token.type=='PLUS':return {'type':'text','color':text}
    elif token.type=='MINUS':return {'type':'text','color':text}
    elif token.type=='MULTIPLY':return {'type':'text','color':text}
    elif token.type=='DIVIDE':return {'type':'text','color':text}
    elif token.type=='MODULO':return {'type':'text','color':text}
    elif token.type=='EQUAL_EQUAL':return {'type':'text','color':text}
    elif token.type=='NOT_EQUAL':return {'type':'text','color':text}
    elif token.type=='LESS_EQUAL':return {'type':'text','color':text}
    elif token.type=='LESS_THAN':return {'type':'text','color':text}
    elif token.type=='GREATER_EQUAL':return {'type':'text','color':text}
    elif token.type=='GREATER_THAN':return {'type':'text','color':text}
    elif token.type=='LPAREN':
        i+=1
        return {'type':'brace','color':brasces[i]}
    elif token.type=='RPAREN':
        i-=1
        return {'type':'brace','color':brasces[i]}
    elif token.type=='LBRACE':
        i+=1
        return {'type':'brace','color':brasces[i]}
    elif token.type=='RBRACE':
        i-=1
        return {'type':'brace','color':brasces[i]}
    elif token.type=='COLON':return {'type':'keyword','color':keyword}
    elif token.type=='SEMICOLON':return {'type':'keyword','color':keyword}
    elif token.type=='STRING_LITERAL':return {'type':'lieral','color':string}
    elif token.type=='BACKSTRING_LITERAL':return {'type':'lieral','color':string}
    elif token.type=='AT':return {'type':'text','color':text}
    elif token.type=='COMMA':return {'type':'text','color':text}
    elif token.type=='DOT':return {'type':'text','color':text}
    elif token.type=='NUMBER':return {'type':'text','color':text}
    else:raise Exception(f'Unknown Token: {token}')
class SytaxHighlight:
    def __init__(self,data:Union['Logic',List[Dict[str,str]]]):
        self.data=data
    def __repr__(self):
        for e in self.data:
            if e.color=='#fff':return ('default',e.color)
            elif e.color=='##777':return ('local_vars',e.color)
            elif e.color=='#00f':
                if e.type=='keyword':return ('keywords',e.color)
                elif e.type=='brace':return ('brace',e.color)
            elif e.color=='#f70':return ('string',e.color)
            elif e.color=='#0ff':return ('class_call',e.color)
            elif e.color=='#0f0':return ('comment',e.color)
            elif e.color=='#ff0':return ('brace',e.color)
            elif e.color=='#f0f':return ('brace',e.color)
=======
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
>>>>>>> 1874ff2b74a13adf17314473452a305958ba4d44
