<!--
  An entity-relationship diagram of the sample issue-tracker schema.

  It's drawn from the same schema JSON that the docs' examples are compiled against, so its tables,
  columns, and links can't drift from the schema. The placement of the tables is decided by hand,
  below. The links are routed automatically, with right angles, in vertical lanes between the
  columns of tables, so that no two links ever run along the same line.
-->
<script lang="ts" module>
  import schema from '../../../../compiler/resources/test/issue_schema.json';

  const COLUMN_WIDTH = 176;
  const ROW_HEIGHT = 22;
  const MARGIN = 28;
  /** The narrowest that the gap between two columns of tables can be. */
  const MIN_GAP = 48;
  /** The distance between the vertical lanes that links run along. */
  const LANE_SPACING = 12;
  const CORNER_RADIUS = 5;

  /**
   * Where each table goes: which column of the grid, and which row its heading sits in. A row is the
   * height of one line of a table, so a table with n columns takes up n + 1 rows.
   *
   * Links can only join tables in the same column or in neighboring columns. A link between tables
   * in the same column goes around the right of the column. A foreign key in the same row as the
   * column it refers to gets a straight line.
   */
  const PLACEMENT: Record<string, { column: number; row: number }> = {
    issue_labels: { column: 0, row: 1 },
    labels: { column: 0, row: 6 },
    projects: { column: 0, row: 11 },
    products: { column: 0, row: 18 },
    blocks: { column: 1, row: 2 },
    issues: { column: 1, row: 8 },
    clients: { column: 1, row: 20 },
    assignments: { column: 2, row: 0 },
    comments: { column: 2, row: 5 },
    users: { column: 2, row: 12 },
    teams: { column: 2, row: 20 },
  };

  interface Box {
    name: string;
    column: number;
    y: number;
    height: number;
    columns: { name: string; type: string; y: number; links: boolean }[];
  }

  const linked = new Set(schema.links.map((link) => `${link.from.table}.${link.from.column}`));

  const boxes: Box[] = schema.tables.map((table) => {
    const placement = PLACEMENT[table.name];
    if (!placement) {
      throw new Error(
        `The sample schema has a table (${table.name}) that the ER diagram doesn't place. ` +
          'Add it to PLACEMENT in ErDiagram.svelte.',
      );
    }
    const y = MARGIN + placement.row * ROW_HEIGHT;
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

  /**
   * All the links that point at the same column, in the same gap. They share one vertical lane,
   * branching off it to each foreign key, since it doesn't matter which branch a reader follows.
   */
  interface Net {
    gap: number;
    target: End;
    sources: { key: string; label: string; end: End }[];
    /** Every point where the net meets a table: the target and each source. */
    ends: End[];
    top: number;
    bottom: number;
    lane: number;
  }

  function end(table: string, column: string, gap: number): End {
    const box = boxes.find((b) => b.name === table);
    const row = box?.columns.find((c) => c.name === column);
    if (!box || !row)
      throw new Error(`The sample schema links to ${table}.${column}, which is missing.`);
    return { box, y: row.y, fromLeft: box.column < gap };
  }

  const nets = new Map<string, Net>();
  for (const link of schema.links) {
    const key = `${link.from.table}.${link.from.column}`;
    const fromColumn = PLACEMENT[link.from.table].column;
    const toColumn = PLACEMENT[link.to.table].column;
    if (Math.abs(fromColumn - toColumn) > 1) {
      throw new Error(
        `In the ER diagram, ${key} links to a table that's more than one column away. ` +
          'Change PLACEMENT in ErDiagram.svelte so that the two tables are closer.',
      );
    }
    const gap = fromColumn === toColumn ? fromColumn + 1 : Math.max(fromColumn, toColumn);
    const netKey = `${gap}:${link.to.table}.${link.to.column}`;
    let net = nets.get(netKey);
    if (!net) {
      const target = end(link.to.table, link.to.column, gap);
      net = { gap, target, sources: [], ends: [target], top: 0, bottom: 0, lane: 0 };
      nets.set(netKey, net);
    }
    const source = end(link.from.table, link.from.column, gap);
    net.sources.push({
      key,
      label: `${key} refers to ${link.to.table}.${link.to.column}`,
      end: source,
    });
    net.ends.push(source);
  }
  for (const net of nets.values()) {
    net.top = Math.min(...net.ends.map((e) => e.y));
    net.bottom = Math.max(...net.ends.map((e) => e.y));
  }

  /**
   * How bad an ordering of the lanes in a gap is, from left to right. Two links running along the
   * same line are worst. After that come links crossing each other, and then the length of the
   * horizontal parts of the links.
   */
  function cost(order: Net[]): number {
    // Lanes are numbered from 0. The gap's left and right edges are at -1 and order.length.
    const span = (e: End, lane: number) => (e.fromLeft ? [-1, lane] : [lane, order.length]);
    let total = 0;
    order.forEach((net, lane) => {
      for (const e of net.ends) {
        const [lo, hi] = span(e, lane);
        total += hi - lo;
        order.forEach((other, otherLane) => {
          if (other === net) return;
          if (otherLane > lo && otherLane < hi && e.y > other.top && e.y < other.bottom) {
            total += 100;
          }
          for (const o of other.ends) {
            if (o.y !== e.y) continue;
            const [otherLo, otherHi] = span(o, otherLane);
            if (Math.max(lo, otherLo) < Math.min(hi, otherHi)) total += 10_000;
          }
        });
      }
    });
    return total;
  }

  function* permutations<T>(items: T[]): Generator<T[]> {
    if (items.length <= 1) {
      yield items;
      return;
    }
    for (let i = 0; i < items.length; i++) {
      const rest = [...items.slice(0, i), ...items.slice(i + 1)];
      for (const permutation of permutations(rest)) yield [items[i], ...permutation];
    }
  }

  function overlapError(gap: number): Error {
    return new Error(
      `In the ER diagram, two links between columns ${gap - 1} and ${gap} would run along the ` +
        'same line. Change PLACEMENT in ErDiagram.svelte to move one of the tables up or down.',
    );
  }

  /**
   * The nets in each gap that need a lane, in the order of their lanes from left to right. A net
   * whose ends are all in one row is a straight line across the gap, and doesn't need one.
   */
  const lanes = Array.from({ length: columnCount + 1 }, (_, gap) => {
    const inGap = [...nets.values()].filter((net) => net.gap === gap);
    const straight = inGap.filter((net) => net.top === net.bottom);
    for (const net of straight) {
      if (inGap.some((other) => other !== net && other.ends.some((e) => e.y === net.top))) {
        throw overlapError(gap);
      }
    }
    const laned = inGap.filter((net) => net.top !== net.bottom);
    if (laned.length > 8) {
      throw new Error(
        `In the ER diagram, too many links run between columns ${gap - 1} and ${gap} to arrange ` +
          'them all. Change PLACEMENT in ErDiagram.svelte to spread the tables out.',
      );
    }
    let best = laned;
    let bestCost = Infinity;
    for (const order of permutations(laned)) {
      const c = cost(order);
      if (c < bestCost) [best, bestCost] = [order, c];
    }
    if (bestCost >= 10_000) throw overlapError(gap);
    return best;
  });

  const gapWidths = lanes.map((nets, gap) => {
    const outer = gap === 0 || gap === columnCount;
    return Math.max(outer ? MARGIN : MIN_GAP, (nets.length + 1) * LANE_SPACING);
  });
  const gapStarts = gapWidths.map((_, gap) => {
    let x = 0;
    for (let g = 0; g < gap; g++) x += gapWidths[g] + COLUMN_WIDTH;
    return x;
  });

  lanes.forEach((nets, gap) => {
    const start = gapStarts[gap];
    const width = gapWidths[gap];
    nets.forEach((net, i) => {
      if (gap === 0) {
        // Keep the lanes outside the first column close to it.
        net.lane = start + width - (nets.length - i) * LANE_SPACING;
      } else if (gap === columnCount) {
        // And likewise for the lanes outside the last column.
        net.lane = start + (i + 1) * LANE_SPACING;
      } else {
        net.lane = start + (width * (i + 1)) / (nets.length + 1);
      }
    });
  });

  const boxX = (box: Box) => gapStarts[box.column] + gapWidths[box.column];
  /** Where a link touches a table's edge. */
  const edgeX = (e: End) => boxX(e.box) + (e.fromLeft ? COLUMN_WIDTH : 0);

  const width = gapStarts[columnCount] + gapWidths[columnCount];
  const height = MARGIN * 2 + Math.max(...boxes.map((box) => box.y - MARGIN + box.height));

  /** A line from (x1, y1) across to the lane, along it, and across to (x2, y2), with round corners. */
  function route(x1: number, y1: number, lane: number, x2: number, y2: number): string {
    if (y1 === y2) return `M${x1},${y1} H${x2}`;
    const dy = Math.sign(y2 - y1);
    const r = Math.min(CORNER_RADIUS, Math.abs(y2 - y1) / 2);
    const out = Math.sign(lane - x1);
    const back = Math.sign(x2 - lane);
    return (
      `M${x1},${y1} H${lane - out * r} Q${lane},${y1} ${lane},${y1 + dy * r} ` +
      `V${y2 - dy * r} Q${lane},${y2} ${lane + back * r},${y2} H${x2}`
    );
  }

  const edges = [...nets.values()].flatMap((net) =>
    net.sources.map((source) => {
      const x1 = edgeX(source.end);
      return {
        key: source.key,
        label: source.label,
        path: route(x1, source.end.y, net.lane, edgeX(net.target), net.target.y),
        start: { x: x1, y: source.end.y },
      };
    }),
  );

  /** Dots where a net branches, to tell a branch apart from two links crossing. */
  const junctions = [...nets.values()].flatMap((net) => {
    const ys = [...new Set(net.ends.map((e) => e.y))];
    return ys
      .filter((y) => {
        const branches = net.ends.filter((e) => e.y === y).length;
        return branches + (y > net.top ? 1 : 0) + (y < net.bottom ? 1 : 0) >= 3;
      })
      .map((y) => ({ key: `${net.gap}:${net.lane}:${y}`, x: net.lane, y }));
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
        <circle class="dot" cx={edge.start.x} cy={edge.start.y} r="2.5" />
      {/each}
      {#each junctions as junction (junction.key)}
        <circle class="dot" cx={junction.x} cy={junction.y} r="2.5" />
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
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg-subtle);
  }

  svg {
    display: block;
    max-width: 100%;
    height: auto;
    min-width: 34rem;
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
