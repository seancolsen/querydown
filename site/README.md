# Querydown Site

This directory holds the code for https://querydown.org

It is built with [SvelteKit](https://kit.svelte.dev/) using

## Developing

⚠️ **TODO** ⚠️ currently `npm install` has a preinstall command which installs the stable rust toolchain and the wasm-pack command. I did this to make the build step work with Netlify deploy. We need to fix this.

```
npm install
```

```
npm run dev
```

## Building

Build is done via [adapter-static](https://kit.svelte.dev/docs/adapter-static) to produce a static site.

```
npm run build
```

```
npm run preview
```
