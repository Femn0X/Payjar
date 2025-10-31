import tkinter as tk
from tkinter import filedialog,messagebox
from esolangInter import *
from syntax import *
class Window:
    def __init__(self,w,h):
        self.root=tk.Tk()
        self.root.geometry(f"{w}x{h}")
        self.text1=tk.Text(self.root,wrap='word',width=100,height=40)
        self.text1.grid(row=0,column=0)
        self.text2=tk.Text(self.root,wrap='word',width=100,height=10)
        self.text2.grid(row=1,column=0)
        tk.Button(self.root,text="Save file",command=self.saveFile).grid(row=2,column=0,columnspan=4)
        tk.Button(self.root,text='Run code',command=self.run).grid(row=2,column=5,columnspan=4)
        tk.Button(self.root,text="Open file",command=self.openFile).grid(row=2,column=10,columnspan=4)
        self.root.mainloop()
    def run(self):
        code=self.text1.get("1.0","end-1c")
        input=self.text2.get('1.0','end-1c')
        self.text2.delete('1.0',tk.END)
        self.text2.insert(tk.END,PJS(code,input))
        PJRT(code,1)
    def saveFile(self):
        filePath=filedialog.asksaveasfilename(
            title='Save file',
            defaultextension='*.txt*',
            filetypes=[('Text file','*.txt*'),('Payjar file','*.pj*'),('MD file','*.md*')],
            initialdir='~'
        )
        if not filePath:
            return
        try:
            content=self.text1.get('1.0',tk.END)
            with open(filePath,'w') as outputFile:
                outputFile.write(content)
        except Exception as e:
            print(e)
    def openFile(self):
        filePath=filedialog.askopenfilename(
            title="Select file",
            filetypes=[("All files","*.*"),("Text files","*.txt*"),("Payjar files","*.pj*j")],
            initialdir="~"
        )
        if filePath:
            try:
                with open(filePath,'r') as file:
                    content=file.read()
                    self.text1.delete('1.0',tk.END)
                    self.text1.insert(tk.END,content)
            except Exception as e:
                print(e)