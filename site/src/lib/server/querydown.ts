/**
 * The Querydown compiler, for use while rendering pages on the server (that is, at build time).
 *
 * The WASM bindings are built for the browser (`wasm-pack --target web`), which is also what the
 * playground uses. Here we hand them the bytes of the `.wasm` file instead of a URL to fetch.
 */

import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import path from 'node:path';

import init, { compile, is_querydown_error, tokenize } from 'querydown-js';

import { repoPath, SITE_DIR } from './paths';

let ready: Promise<unknown> | undefined;

/** Loads the compiler. Await this before calling {@link compileExample} or {@link tokenize}. */
export function loadQuerydown(): Promise<unknown> {
  ready ??= readFile(path.join(SITE_DIR, 'wasm/querydown_js_bg.wasm')).then((bytes) =>
    init({ module_or_path: bytes }),
  );
  return ready;
}

export const SAMPLE_SCHEMA_FILE = 'compiler/resources/test/issue_schema.json';

/** The sample issue-tracker schema, which every example in the docs is written against. */
export function sampleSchema(): string {
  return readFileSync(repoPath(SAMPLE_SCHEMA_FILE), 'utf8');
}

export interface CompileOptions {
  limit?: number;
  offset?: number;
}

export type CompileOutcome =
  { ok: true; sql: string } | { ok: false; message: string; hint: string | null };

/** Compiles an example against the sample schema, for PostgreSQL. */
export function compileExample(source: string, options: CompileOptions = {}): CompileOutcome {
  try {
    return { ok: true, sql: compile(sampleSchema(), 'postgres', source, options).sql };
  } catch (error) {
    if (!is_querydown_error(error)) throw error;
    return { ok: false, message: error.reason, hint: error.hint };
  }
}

export { tokenize };
