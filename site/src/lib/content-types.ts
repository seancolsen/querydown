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

export interface Feature {
  /** The feature's name, as plain text. */
  name: string;
  /** Where the docs explain the feature, if the name is a link. */
  href: string | null;
  /** The example's description, as HTML. */
  caption: string;
  /** The example's Querydown. */
  source: string;
  /** The highlighted Querydown, as HTML. */
  code: string;
  /** A link that opens the example in the playground. */
  playground: string;
}

/** An example in the prose: its Querydown, and what that compiles to. */
export interface Example {
  /** The example's description, as HTML, if it has one. */
  caption: string | null;
  /** The example's Querydown. */
  source: string;
  /** The highlighted Querydown, as HTML. */
  code: string;
  /** A link that opens the example in the playground, unless it's a fragment. */
  playground: string | null;
  /**
   * The SQL that the example compiles to, if the page should offer to show it. (It doesn't for a
   * fragment, an example that fails to compile, or one whose output the page shows itself.)
   */
  sql: string | null;
  /**
   * Why the example doesn't compile, if it's expected not to. The compiler quotes code with
   * backticks, as Markdown does.
   */
  error: { message: string; hint: string | null } | null;
}

/**
 * A piece of prose: HTML rendered from Markdown, with a Svelte component in place of each code
 * block. HTML can't hold a component, so an element that has one within it (such as the list item
 * of a step in some instructions) is a block of its own, with its contents as blocks.
 */
export type Block =
  | { type: 'html'; html: string }
  | {
      type: 'element';
      tag: string;
      attributes: Record<string, string>;
      children: Block[];
    }
  | {
      type: 'code';
      /** The code, to copy. */
      source: string;
      /** The highlighted code, as the HTML of a `<pre>` element. */
      html: string;
    }
  | ({ type: 'example' } & Example);

/**
 * A page is a sequence of segments: mostly prose, with the occasional component that a Svelte page
 * renders (requested in Markdown with a marker comment such as `<!-- er-diagram -->`).
 */
export type Segment =
  | { type: 'prose'; blocks: Block[] }
  | {
      type: 'slideshow';
      slides: Slide[];
      /** A note about all of the slides, as HTML, shown beneath them. */
      note: string | null;
    }
  | {
      type: 'features';
      /** The section's heading, as HTML, shown within the component. */
      heading: string | null;
      features: Feature[];
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
