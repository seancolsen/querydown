<!--
  An entity-relationship diagram of the sample issue-tracker schema.

  It's drawn from the same schema JSON that the docs' examples are compiled against, so its tables,
  columns, and links can't drift from the schema. Where the tables go, and how the links between
  them run, is decided by hand, below. The links run with right angles, in vertical lanes between
  the columns of tables, and the diagram refuses to draw two of them along the same line.
-->
<script lang="ts" module>
  import schema from '../../../../compiler/resources/test/issue_schema.json';

  const COLUMN_WIDTH = 176;
  const ROW_HEIGHT = 22;
  /** Room around the diagram's edges: just enough for the strokes, dots, and arrows there. */
  const INSET = 3;
  /** The narrowest that the gap between two columns of tables can be. */
  const MIN_GAP = 48;
  /**
   * The distance between a table and the nearest lane to it, so that the arrow and the round corner
   * on a link's end both fit, with room between them.
   */
  const LANE_MARGIN = 24;
  /** The distance between neighboring lanes in a gap. */
  const LANE_SPACING = 16;
  const CORNER_RADIUS = 5;
  /** The radius of a corner where a link merges into another. It must be less than LANE_SPACING. */
  const MERGE_RADIUS = 10;

  /**
   * Where each table goes: which column of the grid, and which row its heading sits in. A row is the
   * height of one line of a table, so a table with n columns takes up n + 1 rows. A table can sit
   * between rows, where it needs to be clear of a link in the next column.
   *
   * Links can only join tables in the same column or in neighboring columns.
   */
  const PLACEMENT: Record<string, { column: number; row: number }> = {
    issue_labels: { column: 0, row: 0 },
    blocks: { column: 0, row: 5 },
    issues: { column: 0, row: 10 },
    projects: { column: 0, row: 21 },
    products: { column: 0, row: 28 },
    labels: { column: 1, row: 0 },
    assignments: { column: 1, row: 4 },
    comments: { column: 1, row: 9 },
    // Half a row lower than the others, so that `users.id` isn't level with `issues.project`. Then
    // the two links there don't look like one line.
    users: { column: 1, row: 16.5 },
    teams: { column: 1, row: 24.5 },
    clients: { column: 1, row: 28.5 },
  };

  /**
   * How each link runs, keyed by its foreign key.
   *
   * A link runs in a gap: between two columns of tables, or outside the first or last column. A
   * link between neighboring columns runs in the gap between them. A link between tables in the
   * same column runs along the `side` of the column that it's given.
   *
   * In its gap, a link runs from the foreign key across to its `lane`, along the lane, and across to
   * the column it refers to. Lanes are numbered from 0 at the left of the gap, and the gap is as
   * wide as its lanes need. A foreign key in the same row as the column it refers to gets a straight
   * line instead, and no lane.
   *
   * Links to the same column can share a lane, branching off it to each foreign key, since it
   * doesn't matter which branch a reader follows. But the foreign keys sharing a lane must all be on
   * one side of it, so that a reader can't follow a link from one foreign key to another. Links to
   * different columns can share a lane where they're apart.
   */
  const ROUTES: Record<string, { side?: 'left' | 'right'; lane?: number }> = {
    // Left of the first column
    'issues.duplicate_of': { side: 'left', lane: 0 },
    'issue_labels.issue': { side: 'left', lane: 1 },
    // Between the two columns
    'blocks.blocking': { side: 'right', lane: 0 },
    'issues.project': { side: 'right', lane: 0 },
    'projects.product': { side: 'right', lane: 0 },
    'blocks.blocker': { side: 'right', lane: 1 },
    'issues.author': { lane: 1 },
    'issue_labels.label': { lane: 2 },
    'assignments.issue': { lane: 2 },
    'comments.issue': {},
    'products.client': { lane: 2 },
    // Right of the second column
    'comments.user': { side: 'right', lane: 0 },
    'users.team': { side: 'right', lane: 0 },
    'assignments.user': { side: 'right', lane: 1 },
  };

  interface Box {
    name: string;
    column: number;
    y: number;
    height: number;
    columns: { name: string; type: string; y: number; links: boolean }[];
  }

  const linked = new Set(schema.links.map((link) => `${link.from.table}.${link.from.column}`));

  for (const key of Object.keys(ROUTES)) {
    if (!linked.has(key)) {
      throw new Error(
        `ROUTES in ErDiagram.svelte has a route for ${key}, which isn't a foreign key in the ` +
          'sample schema.',
      );
    }
  }

  const boxes: Box[] = schema.tables.map((table) => {
    const placement = PLACEMENT[table.name];
    if (!placement) {
      throw new Error(
        `The sample schema has a table (${table.name}) that the ER diagram doesn't place. ` +
          'Add it to PLACEMENT in ErDiagram.svelte.',
      );
    }
    const y = INSET + placement.row * ROW_HEIGHT;
    return {
      name: table.name,
      column: placement.column,
      y,
      height: (table.columns.length + 1) * ROW_HEIGHT,
      columns: table.columns.map((column, index) => ({
        name: column.name,
        type: column.type,
        y: y + (index + 1.5) * ROW_HEIGHT,
        links: linked.has(`${table.name}.${column.name}`),
      })),
    };
  });

  const columnCount = Math.max(...boxes.map((box) => box.column)) + 1;

  /**
   * Where a link meets a table: a row of the table, on the side facing the gap the link runs in.
   * Gap g is the one to the left of column g, so there's one more gap than there are columns.
   */
  interface End {
    box: Box;
    y: number;
    /** Whether the table is on the left of the gap (so the link leaves from the table's right). */
    fromLeft: boolean;
  }

  function end(table: string, column: string, gap: number): End {
    const box = boxes.find((b) => b.name === table);
    const row = box?.columns.find((c) => c.name === column);
    if (!box || !row)
      throw new Error(`The sample schema links to ${table}.${column}, which is missing.`);
    return { box, y: row.y, fromLeft: box.column < gap };
  }

  /** A horizontal or vertical stretch of a link, ignoring its round corners. */
  interface Segment {
    vertical: boolean;
    /** The y of a horizontal segment, or the x of a vertical one. */
    at: number;
    from: number;
    to: number;
  }

  interface Link {
    key: string;
    label: string;
    gap: number;
    /** Links to the same column in the same gap merge where they meet, and have the same group. */
    group: string;
    source: End;
    target: End;
    /** The link's lane, unless it's a straight line. */
    lane?: number;
  }

  const links: Link[] = schema.links.map((link) => {
    const key = `${link.from.table}.${link.from.column}`;
    const route = ROUTES[key];
    if (!route) {
      throw new Error(
        `The sample schema has a link (from ${key}) that the ER diagram doesn't route. ` +
          'Add it to ROUTES in ErDiagram.svelte.',
      );
    }
    const fromColumn = PLACEMENT[link.from.table].column;
    const toColumn = PLACEMENT[link.to.table].column;
    let gap: number;
    if (fromColumn === toColumn) {
      if (!route.side) {
        throw new Error(
          `In the ER diagram, ${key} links two tables in the same column, so its route in ` +
            'ROUTES needs a side.',
        );
      }
      gap = route.side === 'left' ? fromColumn : fromColumn + 1;
    } else if (Math.abs(fromColumn - toColumn) === 1) {
      if (route.side) {
        throw new Error(
          `In the ER diagram, ${key} links tables in neighboring columns, so it runs between ` +
            "them, and its route in ROUTES can't have a side.",
        );
      }
      gap = Math.max(fromColumn, toColumn);
    } else {
      throw new Error(
        `In the ER diagram, ${key} links to a table that's more than one column away. ` +
          'Change PLACEMENT in ErDiagram.svelte so that the two tables are closer.',
      );
    }
    const source = end(link.from.table, link.from.column, gap);
    const target = end(link.to.table, link.to.column, gap);
    if (source.y === target.y && route.lane !== undefined) {
      throw new Error(
        `In the ER diagram, ${key} runs straight across to the column it refers to, so its ` +
          "route in ROUTES can't have a lane.",
      );
    }
    if (source.y !== target.y && route.lane === undefined) {
      throw new Error(`In the ER diagram, ${key} needs a lane in ROUTES.`);
    }
    return {
      key,
      label: `${key} refers to ${link.to.table}.${link.to.column}`,
      gap,
      group: `${gap}:${link.to.table}.${link.to.column}`,
      source,
      target,
      lane: route.lane,
    };
  });

  for (const link of links) {
    const sharing = links.filter((l) => l.group === link.group && l.lane === link.lane);
    if (
      link.lane !== undefined &&
      sharing.some((l) => l.source.fromLeft !== link.source.fromLeft)
    ) {
      throw new Error(
        `In the ER diagram, ${link.key} shares a lane with a foreign key on the other side of it. ` +
          'Give one of them another lane in ROUTES in ErDiagram.svelte.',
      );
    }
  }

  /** How many lanes each gap has. */
  const laneCounts = Array.from({ length: columnCount + 1 }, (_, gap) =>
    Math.max(0, ...links.filter((l) => l.gap === gap).map((l) => (l.lane ?? -1) + 1)),
  );

  /** How wide a gap's lanes are, from the first to the last. */
  const lanesWidth = (lanes: number) => Math.max(0, lanes - 1) * LANE_SPACING;

  const gapWidths = laneCounts.map((lanes, gap) => {
    if (lanes === 0) return gap === 0 || gap === columnCount ? INSET : MIN_GAP;
    // The lanes outside the first and last columns keep close to them.
    if (gap === 0 || gap === columnCount) return INSET + lanesWidth(lanes) + LANE_MARGIN;
    return Math.max(MIN_GAP, LANE_MARGIN * 2 + lanesWidth(lanes));
  });
  const gapStarts = gapWidths.map((_, gap) => {
    let x = 0;
    for (let g = 0; g < gap; g++) x += gapWidths[g] + COLUMN_WIDTH;
    return x;
  });

  function laneX(gap: number, lane: number): number {
    const start = gapStarts[gap];
    const lanes = laneCounts[gap];
    if (gap === 0) return start + INSET + lane * LANE_SPACING;
    if (gap === columnCount) return start + LANE_MARGIN + lane * LANE_SPACING;
    // Center the lanes in a gap that's wider than they need.
    return start + (gapWidths[gap] - lanesWidth(lanes)) / 2 + lane * LANE_SPACING;
  }

  const boxX = (box: Box) => gapStarts[box.column] + gapWidths[box.column];
  /** Where a link touches a table's edge. */
  const edgeX = (e: End) => boxX(e.box) + (e.fromLeft ? COLUMN_WIDTH : 0);

  const width = gapStarts[columnCount] + gapWidths[columnCount];
  const height = INSET * 2 + Math.max(...boxes.map((box) => box.y - INSET + box.height));

  /**
   * A line from (x1, y1) across to the lane, along it, and across to (x2, y2), with round corners of
   * radius r1 and r2.
   */
  function route(
    x1: number,
    y1: number,
    lane: number,
    x2: number,
    y2: number,
    r1: number,
    r2: number,
  ): string {
    if (y1 === y2) return `M${x1},${y1} H${x2}`;
    const dy = Math.sign(y2 - y1);
    const out = Math.sign(lane - x1);
    const back = Math.sign(x2 - lane);
    return (
      `M${x1},${y1} H${lane - out * r1} Q${lane},${y1} ${lane},${y1 + dy * r1} ` +
      `V${y2 - dy * r2} Q${lane},${y2} ${lane + back * r2},${y2} H${x2}`
    );
  }

  const span = (a: number, b: number) => ({ from: Math.min(a, b), to: Math.max(a, b) });

  const paths = links.map((link) => {
    const x1 = edgeX(link.source);
    const x2 = edgeX(link.target);
    const y1 = link.source.y;
    const y2 = link.target.y;
    const lane = link.lane === undefined ? x2 : laneX(link.gap, link.lane);
    const segments: Segment[] =
      y1 === y2
        ? [{ vertical: false, at: y1, ...span(x1, x2) }]
        : [
            { vertical: false, at: y1, ...span(x1, lane) },
            { vertical: true, at: lane, ...span(y1, y2) },
            { vertical: false, at: y2, ...span(lane, x2) },
          ];
    return { ...link, x1, y1, lane, x2, y2, segments };
  });

  // Links that don't merge mustn't touch, except to cross.
  paths.forEach((a, i) => {
    for (const b of paths.slice(i + 1)) {
      if (a.group === b.group) continue;
      const touch = a.segments.some((s) =>
        b.segments.some(
          (t) =>
            s.vertical === t.vertical &&
            s.at === t.at &&
            Math.max(s.from, t.from) <= Math.min(s.to, t.to),
        ),
      );
      if (touch) {
        throw new Error(
          `In the ER diagram, the links from ${a.key} and ${b.key} would run into each other. ` +
            'Change their lanes in ROUTES, or move a table in PLACEMENT, in ErDiagram.svelte.',
        );
      }
    }
  });

  /** In how many directions the lines of a group of links leave a point. */
  function directions(group: string, x: number, y: number): number {
    const found = new Set<string>();
    for (const s of paths.filter((p) => p.group === group).flatMap((p) => p.segments)) {
      const [along, across] = s.vertical ? [y, x] : [x, y];
      if (s.at !== across || along < s.from || along > s.to) continue;
      if (s.from < along) found.add(s.vertical ? 'up' : 'left');
      if (s.to > along) found.add(s.vertical ? 'down' : 'right');
    }
    return found.size;
  }

  /**
   * The radius of a link's corner at (x, y). Where the link merges into another there, the corner
   * is wider, so that it reads as a branch turning the way the links run, toward the column they
   * refer to.
   */
  function radius(link: (typeof paths)[number], x: number, y: number): number {
    const r = directions(link.group, x, y) >= 3 ? MERGE_RADIUS : CORNER_RADIUS;
    return Math.min(r, Math.abs(link.y2 - link.y1) / 2);
  }

  const edges = paths.map((p) => {
    const r1 = radius(p, p.lane, p.y1);
    const r2 = radius(p, p.lane, p.y2);
    return { ...p, path: route(p.x1, p.y1, p.lane, p.x2, p.y2, r1, r2) };
  });
</script>

<figure>
  <div class="scroll">
    <svg viewBox="0 0 {width} {height}" {width} {height} role="img" aria-labelledby="er-title">
      <title id="er-title">
        Entity-relationship diagram of the sample issue-tracker schema: {boxes.length} tables and the
        {edges.length} links between them.
      </title>
      <defs>
        <marker
          id="er-arrow"
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto-start-reverse"
        >
          <path d="M1,1.5 L9,5 L1,8.5" class="arrow" />
        </marker>
      </defs>

      <g class="links">
        {#each edges as edge (edge.key)}
          <path class="edge" d={edge.path} marker-end="url(#er-arrow)"
            ><title>{edge.label}</title></path
          >
        {/each}
      </g>
      {#each edges as edge (edge.key)}
        <circle class="dot" cx={edge.x1} cy={edge.y1} r="2.5" />
      {/each}

      {#each boxes as box (box.name)}
        {@const x = boxX(box)}
        <g>
          <rect class="table" {x} y={box.y} width={COLUMN_WIDTH} height={box.height} rx="5" />
          <path
            class="heading"
            d="M{x},{box.y + ROW_HEIGHT} v{-ROW_HEIGHT + 5} a5,5 0 0 1 5,-5 h{COLUMN_WIDTH -
              10} a5,5 0 0 1 5,5 v{ROW_HEIGHT - 5} z"
          />
          <text class="name" x={x + 10} y={box.y + ROW_HEIGHT / 2}>{box.name}</text>
          {#each box.columns as column (column.name)}
            <text class="column" class:links={column.links} x={x + 10} y={column.y}>
              {column.name}
            </text>
            <text class="type" x={x + COLUMN_WIDTH - 10} y={column.y}>{column.type}</text>
          {/each}
          <rect class="outline" {x} y={box.y} width={COLUMN_WIDTH} height={box.height} rx="5" />
        </g>
      {/each}
    </svg>
  </div>
  <figcaption>Each arrow points from a foreign key column to the column it refers to.</figcaption>
</figure>

<style>
  figure {
    margin: 1.5rem 0;
  }

  .scroll {
    overflow-x: auto;
  }

  svg {
    display: block;
    max-width: 100%;
    height: auto;
    /* Narrow enough to fit the smallest phones, but no narrower, so that the text stays legible. */
    min-width: 18rem;
    margin: 0 auto;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  figcaption {
    margin-top: 0.5rem;
    color: var(--text-muted);
    font-size: 0.875rem;
    text-align: center;
  }

  text {
    dominant-baseline: central;
    fill: var(--text);
  }

  .table {
    fill: var(--bg);
  }

  .heading {
    fill: var(--bg-inset);
  }

  .outline {
    fill: none;
    stroke: var(--border-strong);
  }

  .name {
    font-weight: 700;
  }

  .column.links {
    fill: var(--accent);
  }

  .type {
    fill: var(--text-muted);
    opacity: 0.65;
    text-anchor: end;
  }

  /* On the group, so that where branches of a link overlap, they don't draw any darker. */
  .links {
    opacity: 0.75;
  }

  .edge {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.4;
  }

  .dot {
    fill: var(--accent);
  }

  .arrow {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.6;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
</style>
