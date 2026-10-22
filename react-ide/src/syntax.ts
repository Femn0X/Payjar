import * as monaco from 'monaco-editor'

export const PayjarTokens: monaco.languages.IMonarchLanguage = {
  defaultToken: '',
  keywords: [
    'func', 'public', 'private', 'class', 'enum', 'new', 'self', 'const', 'var',
    'let', 'return', 'inner_self', 'innerSelf', 'main', 'println', 'printerr',
    'readln', 'readi', 'readf', 'pass', 'if', 'else', 'while', 'for', 'in',
    'range', 'break', 'continue', 'import', 'package', 'try', 'except',
    'finally', 'throw', 'async', 'await', 'and', 'or', 'not', 'as',
    'Int', 'Float', 'String', 'Null', 'Array', 'Complex', 'Any', 'true', 'false', 'null',
  ],
  tokenizer: {
    root: [
      [/\/\/.*$/, 'comment'],
      [/\/\*/, { token: 'comment', next: '@comment' }],
      [/\`([^\`\\]|\\.)*\`/, 'string'],
      [/"([^"\\]|\\.)*"/, 'string'],
      [/'([^'\\]|\\.)*'/, 'string'],
      [/\d+\.\d+/, 'number.float'],
      [/\d+i/, 'number.complex'],
      [/\d+/, 'number'],
      [/[a-zA-Z_][a-zA-Z0-9_]*/, { cases: { '@keywords': 'keyword', '@default': 'identifier' } }],
      [/\+\+|--|\+=|-=|->|==|!=|<=|>=|&&|\|\|/, 'operator'],
      [/[+*/%!=<>~&|^?-]/, 'operator'],
      [/[{}()\[\]]/, '@brackets'],
      [/[;,.@:]/, 'delimiter'],
    ],
    comment: [
      [/[^/*]+/, 'comment'],
      [/\*\//, { token: 'comment', next: '@pop' }],
      [/[/*]/, 'comment'],
    ],
  },
}

export const PayjarConfig: monaco.languages.LanguageConfiguration = {
  comments: {
    lineComment: '//',
    blockComment: ['/*', '*/'],
  },
  brackets: [
    ['(', ')'],
    ['{', '}'],
    ['[', ']'],
  ],
  autoClosingPairs: [
    { open: '(', close: ')' },
    { open: '{', close: '}' },
    { open: '[', close: ']' },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
    { open: '`', close: '`' },
  ],
  surroundingPairs: [
    { open: '(', close: ')' },
    { open: '{', close: '}' },
    { open: '[', close: ']' },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
    { open: '`', close: '`' },
  ],
}
