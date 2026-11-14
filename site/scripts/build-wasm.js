// Builds the Querydown WASM bindings into `site/wasm`, where the site imports them from (see the
// `querydown-js` alias in vite.config.ts).
//
// With `--if-missing`, does nothing when the bindings have already been built. That keeps
// `npm run dev` fast to start; run `npm run wasm` to rebuild them after changing Rust code.

import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const crate = fileURLToPath(new URL('../../bindings/js', import.meta.url));
const output = fileURLToPath(new URL('../wasm', import.meta.url));

if (process.argv.includes('--if-missing') && existsSync(`${output}/querydown_js_bg.wasm`)) {
  process.exit(0);
}

const result = spawnSync(
  'wasm-pack',
  ['build', crate, '--target', 'web', '--out-dir', output, '--no-pack'],
  { stdio: 'inherit' },
);
if (result.error) {
  console.error(
    `Could not run wasm-pack: ${result.error.message}\n` +
      'Install it from https://rustwasm.github.io/wasm-pack/ (the dev container provides it).',
  );
  process.exit(1);
}
process.exit(result.status ?? 1);
