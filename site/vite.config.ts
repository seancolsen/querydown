import { fileURLToPath } from 'node:url';

import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, type Plugin } from 'vite';

const repo = (path: string) => fileURLToPath(new URL(`../${path}`, import.meta.url));

/**
 * Reloads page data in the browser when a Markdown content file changes.
 *
 * The content is read from disk by server `load` functions, so Vite doesn't know that pages depend
 * on it. This plugin watches the content directories and tells the client (see
 * `src/routes/+layout.svelte`) to re-run its `load` functions.
 */
function contentReload(): Plugin {
  const dirs = [repo('docs'), repo('site/content'), repo('compiler/resources/test')];
  return {
    name: 'querydown-content-reload',
    apply: 'serve',
    configureServer(server) {
      server.watcher.add(dirs);
      const notify = (file: string) => {
        if (dirs.some((dir) => file.startsWith(dir))) {
          server.ws.send({ type: 'custom', event: 'content:update' });
        }
      };
      server.watcher.on('change', notify);
      server.watcher.on('add', notify);
      server.watcher.on('unlink', notify);
    },
  };
}

export default defineConfig({
  plugins: [
    sveltekit({
      // `404.html` is what static hosts serve for a URL that isn't a page.
      adapter: adapter({ fallback: '404.html', strict: true }),
      compilerOptions: {
        runes: ({ filename }) =>
          filename.split(/[/\\]/).includes('node_modules') ? undefined : true,
      },
      prerender: {
        // A broken internal link or anchor fails the build. `scripts/check-links.js` checks the
        // built site again, more thoroughly.
        handleHttpError: 'fail',
        handleMissingId: ({ path, message }) => {
          // The playground keeps its state in the URL's fragment, which isn't the id of anything.
          if (path === '/playground') return;
          throw new Error(message);
        },
      },
      paths: {
        // The value of `url.origin` while prerendering, which absolute URLs are built from.
        origin: 'https://querydown.org',
      },
    }),
    contentReload(),
  ],
  resolve: {
    alias: [
      // The WASM bindings, built into `site/wasm` by `npm run wasm`.
      { find: /^querydown-js$/, replacement: repo('site/wasm/querydown_js.js') },
      { find: /^querydown-js\//, replacement: repo('site/wasm/') },
      // The CodeMirror package is used straight from its TypeScript source, so that it needs no
      // build step of its own.
      { find: /^querydown-codemirror$/, replacement: repo('bindings/codemirror/src/index.ts') },
    ],
    // The CodeMirror package lives outside of this directory, so without this its imports could
    // resolve to a second copy of these libraries, which CodeMirror doesn't tolerate.
    dedupe: [
      '@codemirror/autocomplete',
      '@codemirror/language',
      '@codemirror/state',
      '@codemirror/view',
      '@lezer/common',
      '@lezer/highlight',
    ],
  },
  optimizeDeps: {
    exclude: ['querydown-js', 'querydown-codemirror'],
  },
  server: {
    fs: {
      // The site reads content, the sample schema, and the CodeMirror package from elsewhere in
      // the repository.
      allow: [repo('')],
    },
  },
});
