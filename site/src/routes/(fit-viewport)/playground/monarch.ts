import type { languages } from 'monaco-editor';

export const monarch: languages.IMonarchLanguage = {
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
      [/(@identifier)/, 'base-table', '@root'],
    ],

    root: [
      { include: '@whitespace' },
      [/[{}()[\]]/, '@brackets'],
      [/(@identifier)/, 'column'],
      [/#(@identifier)/, 'table-with-many'],
      [/@(\d+[ymdwthsYMDWTHS])+/, 'duration'],
      [/@(@identifier)/, 'constant'],
      [/@\d\d\d\d-\d\d-\d\d/, 'date'],
      [/\./, 'path-separator'],
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

      [/(:|!)(~~|~|>=|>|<=|<)?/, 'comparison-operator'],

      [/\+\+|--/, 'has'],

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
