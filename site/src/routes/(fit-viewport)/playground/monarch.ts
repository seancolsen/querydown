import type { languages, editor } from 'monaco-editor';

export const qd_monarch: languages.IMonarchLanguage = {
  defaultToken: 'invalid',
  brackets: [
    { open: '{', close: '}', token: 'delimiter.curly' },
    { open: '[', close: ']', token: 'delimiter.square' },
    { open: '(', close: ')', token: 'delimiter.parenthesis' },
  ],

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
      [/(\d+(\.\d+)?(?:[mM][iI][nN]|[ymwdhsYMWDHS]))+/, 'duration'],
      [/@(@identifier)/, 'constant'],
      [/@\d\d\d\d-\d\d-\d\d/, 'date'],

      // a regex match with flags, e.g. `:\cm~`
      [/(:)(\\[a-zA-Z]*)(~~?)/, ['comparison-operator', 'column-control', 'comparison-operator']],
      [/(\.\.)?(:|!)(~~|~|>=|>|<=|<)?(\.\.)?/, 'comparison-operator'],

      [/<?\.\.<?/, 'range-separator'],
      
      [/\./, 'path-separator'],
      [/\*/, 'glob'],
      [/\|/, 'scalar-pipe', '@scalarFunction'],
      [/%/, 'aggregate-pipe', '@aggregateFunction'],

      // numbers
      [/\d*\.\d+([eE][-+]?\d+)?/, 'qd-number'],
      [/\d+/, 'qd-number'],

      // strings (raw, with no escape sequences, and possibly spanning lines)
      [/"/,  { token: 'string.quote', bracket: '@open', next: '@stringDouble' } ],
      [/'/,  { token: 'string.quote', bracket: '@open', next: '@stringSingle' } ],
      // flagged strings, e.g. `^f"Hi {name}"`: the state's arguments are the quote and the flags
      [/\^([a-zA-Z]*)(["'^#/|])/, { token: 'string.flags', next: '@flagged.$2.$1' } ],

      // before `++`, which begins it; the next table is the base table of the union's next query
      [/\+\+\+/, 'union', '@unionBaseTable'],
      [/\+\+|--/, 'has'],

      [/(\+|-|\*\/)/, 'operator'],

      [/\$/, 'column-prefix'],

      [/->/, 'alias-prefix', '@alias'],

      [/\\[a-z][a-z0-9]*/, 'column-control'],
    ],

    unionBaseTable: [
      { include: '@whitespace' },
      [/#(@identifier)/, 'base-table', '@pop'],
      [/(?=.)/, '', '@pop'],
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

    stringDouble: [
      [/[^"]+/, 'string'],
      [/"/,     { token: 'string.quote', bracket: '@close', next: '@pop' } ]
    ],

    stringSingle: [
      [/[^']+/, 'string'],
      [/'/,     { token: 'string.quote', bracket: '@close', next: '@pop' } ]
    ],

    flagged: [
      [/\\/, { cases: { '$S3~.*e.*': { token: 'string.escape', next: '@escape' }, '@default': 'string' } } ],
      [/\{\{|\}\}/, { cases: { '$S3~.*f.*': 'string.escape', '@default': 'string' } } ],
      [/\{/, { cases: { '$S3~.*f.*': { token: 'string.interpolation', next: '@interpolation' }, '@default': 'string' } } ],
      [/[^\\{}"'^#/|]+/, 'string'],
      [/./, { cases: { '$#==$S2': { token: 'string.quote', next: '@pop' }, '@default': 'string' } } ],
    ],

    // what follows a backslash in a string with the `e` flag
    escape: [
      [/u\{[0-9a-fA-F]*\}?|./, { token: 'string.escape', next: '@pop' } ],
    ],

    // the code within a formatted string's `{ }`
    interpolation: [
      [/\}/, { token: 'string.interpolation', next: '@pop' } ],
      { include: '@root' },
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
    { token: 'union', foreground: 'c2185b', fontStyle: 'bold' },
    { token: 'scalar-pipe', foreground: '787113', fontStyle: 'bold' },
    { token: 'scalar-function', foreground: '787113' },
    { token: 'aggregate-pipe', foreground: 'b1ad70', fontStyle: 'bold' },
    { token: 'aggregate-function', foreground: '787113', fontStyle: 'bold' },
    { token: 'invalid', foreground: 'ffa1a6', fontStyle: 'strikethrough' },
    { token: 'alias-prefix', foreground: 'a5a5a5' },
    { token: 'alias', foreground: '7e52a5', fontStyle: 'italic' },
    { token: 'string.flags', fontStyle: 'bold' },
    { token: 'string.escape', foreground: 'c4510f', fontStyle: 'bold' },
    { token: 'string.interpolation', foreground: 'c4510f', fontStyle: 'bold' },
  ],
};
