# Querydown Site

This directory holds the code for https://querydown.org

It is built with [SvelteKit](https://kit.svelte.dev/) using

## Local development

```
npm install
```

```
npm run dev
```

## Refreshing changes from Rust

When you modify some rust code, you'll need to re-generate the wasm files used by the front end.

1. Run:

   ```
   npm install
   ```

1. If your dev server is still running, that's fine. Hard-refresh your browser.

## Building

Build is done via [adapter-static](https://kit.svelte.dev/docs/adapter-static) to produce a static site.

```
npm run build
```

```
npm run preview
```
