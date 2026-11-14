/** What the search index (`/search.json`) holds: one entry for each section of each page. */
export interface SearchEntry {
  id: number;
  /** The section's URL, including the fragment for its heading. */
  url: string;
  /** The title of the page. */
  page: string;
  /** The section's heading. Empty for the text before a page's first heading. */
  heading: string;
  /** The navigation groups, the page, and the headings above the section, outermost first. */
  trail: string[];
  text: string;
}

/**
 * Splits text into the terms to index and to search for.
 *
 * Querydown's syntax is mostly symbols, so runs of symbols are kept as terms of their own. That
 * way a search for `++` or `%%` finds the sections about those operators.
 */
export function tokenize(text: string): string[] {
  return text.match(/[\p{L}\p{N}_]+|[^\s\p{L}\p{N}_]+/gu) ?? [];
}
