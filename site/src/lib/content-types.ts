/** The shapes of rendered content, shared by the server (which renders it) and the pages. */

export interface TocItem {
  id: string;
  text: string;
  /** The heading's level, from 2 to 4. */
  depth: number;
}

/** The text under one heading of a page, for the search index. */
export interface SearchSection {
  /** The heading's id, or `null` for the text before the first heading. */
  id: string | null;
  heading: string;
  /** The headings above this one, outermost first. */
  parents: string[];
  text: string;
}

export interface Slide {
  /** The example's description, as HTML. */
  caption: string;
  /** The highlighted Querydown, as HTML. */
  code: string;
  /** The highlighted SQL that the example compiles to, as HTML. */
  sql: string;
  /** A link that opens the example in the playground. */
  playground: string;
}

/**
 * A page is a sequence of segments: mostly HTML, with the occasional component that a Svelte page
 * renders (requested in Markdown with a marker comment such as `<!-- er-diagram -->`).
 */
export type Segment =
  | { type: 'html'; html: string }
  | {
      type: 'slideshow';
      slides: Slide[];
      /** A note about all of the slides, as HTML, shown beneath them. */
      note: string | null;
    }
  | { type: 'er-diagram' };

export interface RenderedMarkdown {
  title: string;
  description: string;
  segments: Segment[];
  toc: TocItem[];
  sections: SearchSection[];
}

export interface Page extends RenderedMarkdown {
  url: string;
  /** The Markdown file, relative to the repository root. */
  file: string;
  /** The page's own CSS, from the `.css` file beside its Markdown file, if it has one. */
  styles: string | null;
}
