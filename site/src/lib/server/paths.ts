import path from 'node:path';

/** The `site` directory. Vite (and so every npm script) runs from it. */
export const SITE_DIR = process.cwd();
/** The root of the repository. */
export const REPO_DIR = path.resolve(SITE_DIR, '..');

/** The absolute path of a file given relative to the repository root. */
export const repoPath = (file: string) => path.join(REPO_DIR, file);
