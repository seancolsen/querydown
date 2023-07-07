import type { languages, editor } from 'monaco-editor';

export const qd_monarch: languages.IMonarchLanguage = {
  defaultToken: 'invalid',
  brackets: [
    { open: '{', close: '}', token: 'delimiter.curly' },
    { open: '[', close: ']', token: 'delimiter.square' },
    { open: '(', close: ')', token: 'delimiter.parenthesis' },
  ],

  // C# style strings
  escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,

  identifier: /[a-zA-Z_][a-zA-Z0-9_]*/,

  tokenizer: {
    baseTable: [
      { include: '@whitespace' },
      [/#(@identifier)/, 'base-table', '@root'],
    ],

    root: [
      { include: '@whitespace' },
      [/[{}()[\]]/, '@brackets'],
      [/(@identifier)/, 'column'],
      [/#(@identifier)/, 'table-with-many'],
      [/@(\d+[ymdwthsYMDWTHS])+/, 'duration'],
      [/@(@identifier)/, 'constant'],
      [/@\d\d\d\d-\d\d-\d\d/, 'date'],

      [/(\.\.)?(:|!)(~~|~|>=|>|<=|<)?(\.\.)?/, 'comparison-operator'],

      [/<?\.\.<?/, 'range-separator'],
      
      [/\./, 'path-separator'],
      [/\*/, 'glob'],
      [/\|/, 'scalar-pipe', '@scalarFunction'],
      [/%/, 'aggregate-pipe', '@aggregateFunction'],

      // numbers
      [/\d*\.\d+([eE][-+]?\d+)?/, 'qd-number'],
      [/\d+/, 'qd-number'],

      // strings
      [/"([^"\\]|\\.)*$/, 'string.invalid' ],  // non-teminated string
      [/"/,  { token: 'string.quote', bracket: '@open', next: '@string' } ],

      // characters
      [/'[^\\']'/, 'string'],
      [/(')(@escapes)(')/, ['string','string.escape','string']],
      [/'/, 'string.invalid'],

      [/\+\+|--/, 'has'],

      [/(\+|-|\*\/)/, 'operator'],

      [/\$/, 'column-prefix'],

      [/->/, 'alias-prefix', '@alias'],

      [/\\[a-z][a-z0-9]*/, 'column-control'],
    ],

    scalarFunction: [
      [/(@identifier)/, 'scalar-function', '@pop'],
    ],

    aggregateFunction: [
      [/(@identifier)/, 'aggregate-function', '@pop'],
    ],

    alias: [
      { include: '@whitespace' },
      [/(@identifier)/, 'alias', '@pop'],
    ],

    comment: [
      [/[^/*]+/, 'comment' ],
      [/\/\*/,   'comment', '@push' ], // nested comment
      ["\\*/",   'comment', '@pop'  ],
      [/[/*]/,   'comment' ]
    ],

    string: [
      [/[^\\"]+/,  'string'],
      [/@escapes/, 'string.escape'],
      [/\\./,      'string.escape.invalid'],
      [/"/,        { token: 'string.quote', bracket: '@close', next: '@pop' } ]
    ],

    whitespace: [
      [/[ \t\r\n]+/, 'white'],
      [/\/\*/,       'comment', '@comment' ],
      [/\/\/.*$/,    'comment'],
    ],
  },
};

export const qd_theme: editor.IStandaloneThemeData = {
  base: 'vs',
  colors: {},
  inherit: true,
  rules: [
    { token: 'base-table', foreground: '287f90', fontStyle: 'bold underline' },
    { token: 'table-with-many', foreground: '287f90' },
    { token: 'column', foreground: '1e1f63' },
    { token: 'column-control', foreground: '737373', fontStyle: 'bold' },
    { token: 'column-prefix', foreground: 'c963e3', fontStyle: 'bold' },
    { token: 'glob', foreground: '634181', fontStyle: 'bold' },
    { token: 'duration', foreground: 'a63f87' },
    { token: 'date', foreground: 'a63f87' },
    { token: 'qd-number', foreground: 'a66565' },
    { token: 'comparison-operator', foreground: '000be3', fontStyle: 'bold' },
    { token: 'range-separator', foreground: 'a19483' },
    { token: 'has', foreground: '787113', fontStyle: 'bold' },
    { token: 'scalar-pipe', foreground: '787113', fontStyle: 'bold' },
    { token: 'scalar-function', foreground: '787113' },
    { token: 'aggregate-pipe', foreground: 'b1ad70', fontStyle: 'bold' },
    { token: 'aggregate-function', foreground: '787113', fontStyle: 'bold' },
    { token: 'invalid', foreground: 'ffa1a6', fontStyle: 'strikethrough' },
    { token: 'alias-prefix', foreground: 'a5a5a5' },
    { token: 'alias', foreground: '7e52a5', fontStyle: 'italic' },
  ],
};
