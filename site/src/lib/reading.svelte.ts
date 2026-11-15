/**
 * Which section of the current page the reader is within, which the navigation marks in the page's
 * table of contents. The navigation is rendered twice (beside the page, and in the drawer), so this
 * is kept here, once, rather than by either rendering.
 */

import type { TocItem } from '#lib/content-types.ts';

/** How far down the window (in pixels) a heading can be for its section to count as being read. */
const THRESHOLD = 96;

let section = $state<string | null>(null);

/**
 * The heading whose link was last followed, which stays marked until the reader scrolls by
 * themselves. (A heading near the end of a page can't reach the top of the window, so the scroll
 * position alone wouldn't mark it.)
 */
let followed = $state<string | null>(null);

export const reading = {
  /** The id of the heading of the section being read, or `null` above the first heading. */
  get section() {
    return section;
  },
  /** Whether the section is marked because its link was followed, not because of the scrolling. */
  get followed() {
    return followed !== null;
  },
};

/** Marks a section whose link in the table of contents is being followed. */
export function follow(id: string) {
  followed = id;
  section = id;
}

/**
 * Keeps `reading` up to date for a page with these headings, as the window scrolls. Returns a
 * function that stops doing so.
 */
export function watchSections(items: TocItem[]): () => void {
  const headings = items
    .map((item) => document.getElementById(item.id))
    .filter((element) => element !== null);
  if (!headings.some((heading) => heading.id === followed)) followed = null;

  // The section being read is the last one whose heading has passed the top of the window.
  const update = () => {
    if (followed) return;
    let current: string | null = null;
    for (const heading of headings) {
      if (heading.getBoundingClientRect().top > THRESHOLD) break;
      current = heading.id;
    }
    section = current;
  };
  const release = () => {
    if (followed) {
      followed = null;
      update();
    }
  };

  update();
  const userInput = ['wheel', 'touchstart', 'keydown', 'pointerdown'] as const;
  window.addEventListener('scroll', update, { passive: true });
  for (const type of userInput) window.addEventListener(type, release, { passive: true });
  return () => {
    window.removeEventListener('scroll', update);
    for (const type of userInput) window.removeEventListener(type, release);
  };
}
