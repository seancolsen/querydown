// See https://svelte.dev/docs/kit/types#app.d.ts
import type { TocItem } from '#lib/content-types.ts';

declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    interface PageData {
      /** The page's table of contents, if it has one, which the navigation shows. */
      toc?: TocItem[];
    }
    // interface PageState {}
    // interface Platform {}
  }
}

export {};
