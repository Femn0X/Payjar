import * as monaco from 'monaco-editor'

export const PayjarTokens: monaco.languages.IMonarchLanguage = {
  defaultToken: '',
  keywords: [
    'func', 'public', 'private', 'class', 'new', 'self', 'const', 'var',
    'let', 'return', 'inner_self', 'main', 'println', 'readln', 'pass',
  ],
  tokenizer: {
    root: [
      [/\/\/.*$/, 'comment'],
      [/\/\*/, { token: 'comment', next: '@comment' }],
      [/[a-zA-Z_][a-zA-Z0-9_]*/, { cases: { '@keywords': 'keyword', '@default': 'identifier' } }],
      [/\d+/, 'number'],
      [/"([^"\\]|\\.)*"/, 'string'],
      [/'([^'\\]|\\.)*'/, 'string'],
      [/[+*\/%=!<>&|~-]+/, 'operator'],
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
  ],
  surroundingPairs: [
    { open: '(', close: ')' },
    { open: '{', close: '}' },
    { open: '[', close: ']' },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
}
