/**
 * The site's color theme.
 *
 * By default the site follows the browser's color scheme. Choosing a theme stores the choice and
 * sets `data-theme` on the root element, which `app.css` responds to. (`app.html` applies a stored
 * choice before the first paint.) Choosing the theme that the browser prefers anyway clears the
 * choice, so the site goes back to following the browser.
 */

import { browser } from '$app/env';

export type Theme = 'light' | 'dark';

const STORAGE_KEY = 'theme';
const prefersDark = browser ? window.matchMedia('(prefers-color-scheme: dark)') : null;

function storedChoice(): Theme | null {
  if (!browser) return null;
  const value = document.documentElement.dataset.theme;
  return value === 'light' || value === 'dark' ? value : null;
}

let choice = $state<Theme | null>(storedChoice());
let browserPrefersDark = $state(prefersDark?.matches ?? false);

prefersDark?.addEventListener('change', (event) => {
  browserPrefersDark = event.matches;
});

export const theme = {
  /** The theme in effect. On the server, where it can't be known, this is `light`. */
  get current(): Theme {
    return choice ?? (browserPrefersDark ? 'dark' : 'light');
  },

  set(value: Theme) {
    choice = value === (browserPrefersDark ? 'dark' : 'light') ? null : value;
    if (choice) document.documentElement.dataset.theme = choice;
    else delete document.documentElement.dataset.theme;
    try {
      if (choice) localStorage.setItem(STORAGE_KEY, choice);
      else localStorage.removeItem(STORAGE_KEY);
    } catch {
      // Storage may be unavailable (e.g. in a private window). The choice still applies to this page.
    }
  },

  toggle() {
    this.set(this.current === 'dark' ? 'light' : 'dark');
  },
};
