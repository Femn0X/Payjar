from .esolangInter import *
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
    def __init__(self,data:Logic|list[dict]):
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
