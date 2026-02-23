import * as monaco from "monaco-editor";
export const PayjarTokens:monaco.languages.IMonarchLanguage={
 defaultToken:"",
 keywords:["func","public","class","new","self","@","const","var","let","return","inner_self"],
 tokenizer:{
  root:[
   [/\b\d+\b/,"number"],
   [/"([^"\\]|\\.)*"/,"string"],
   [/[+\-*\/=<>&!]+/,"operator"],
   [/[a-aZ-Z_][a-zA-Z0-9_]*/,"identifier"],
  ]
 }
};
export const PayjarConfig:monaco.languages.LanguageConfiguration={
 comments:{
  lineComment:"//",
  blockComent:["/*","*/"],
 },
 brackets:[
  ["(",")"],
  ["{","}"],
 ],
};