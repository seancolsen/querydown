<!--
  An entity-relationship diagram of the sample issue-tracker schema.

  It's drawn from the same schema JSON that the docs' examples are compiled against, so its tables,
  columns, and links can't drift from the schema. Only the placement of the tables is decided here.
-->
<script lang="ts" module>
  import schema from '../../../../compiler/resources/test/issue_schema.json';

  const COLUMN_WIDTH = 176;
  const COLUMN_GAP = 84;
  const ROW_HEIGHT = 22;
  const MARGIN = 28;
  /** Room at the sides for the links that go around the outside of a column. */
  const SIDE_MARGIN = 60;

  /** Where each table goes: which of the diagram's three columns, and how far down. */
  const PLACEMENT: Record<string, { column: number; y: number }> = {
    teams: { column: 0, y: 0 },
    users: { column: 0, y: 104 },
    assignments: { column: 0, y: 296 },
    comments: { column: 0, y: 422 },
    projects: { column: 1, y: 0 },
    issues: { column: 1, y: 188 },
    issue_labels: { column: 1, y: 466 },
    products: { column: 2, y: 22 },
    clients: { column: 2, y: 148 },
    blocks: { column: 2, y: 296 },
    labels: { column: 2, y: 488 },
  };

  /**
   * A link between tables in the same column leaves and arrives on the same side. These are the
   * links that look better going around the right.
   */
  const AROUND_THE_RIGHT = new Set(['issues.duplicate_of', 'issues.project', 'products.client']);

  interface Box {
    name: string;
    x: number;
    y: number;
    height: number;
    column: number;
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
    const x = SIDE_MARGIN + placement.column * (COLUMN_WIDTH + COLUMN_GAP);
    const y = MARGIN + placement.y;
    return {
      name: table.name,
      x,
      y,
      column: placement.column,
      height: (table.columns.length + 1) * ROW_HEIGHT,
      columns: table.columns.map((column, index) => ({
        name: column.name,
        type: column.type,
        y: y + (index + 1.5) * ROW_HEIGHT,
        links: linked.has(`${table.name}.${column.name}`),
      })),
    };
  });

  const width = SIDE_MARGIN * 2 + COLUMN_WIDTH * 3 + COLUMN_GAP * 2;
  const height = MARGIN * 2 + Math.max(...boxes.map((box) => box.y - MARGIN + box.height));

  function anchor(table: string, column: string): { box: Box; y: number } {
    const box = boxes.find((b) => b.name === table);
    const row = box?.columns.find((c) => c.name === column);
    if (!box || !row)
      throw new Error(`The sample schema links to ${table}.${column}, which is missing.`);
    return { box, y: row.y };
  }

  /** A curve from each foreign key column to the column it refers to. */
  const edges = schema.links.map((link) => {
    const from = anchor(link.from.table, link.from.column);
    const to = anchor(link.to.table, link.to.column);
    const key = `${link.from.table}.${link.from.column}`;
    let x1: number, x2: number, c1: number, c2: number;
    if (from.box.column === to.box.column) {
      // Leave and arrive on the same side, bulging outwards.
      const right = AROUND_THE_RIGHT.has(key);
      const direction = right ? 1 : -1;
      const reach = Math.min(COLUMN_GAP * 0.62, 22 + Math.abs(from.y - to.y) * 0.16);
      x1 = from.box.x + (right ? COLUMN_WIDTH : 0);
      x2 = to.box.x + (right ? COLUMN_WIDTH : 0);
      c1 = x1 + direction * reach;
      c2 = x2 + direction * reach;
    } else {
      const rightwards = to.box.column > from.box.column;
      x1 = from.box.x + (rightwards ? COLUMN_WIDTH : 0);
      x2 = to.box.x + (rightwards ? 0 : COLUMN_WIDTH);
      const bend = (x2 - x1) * 0.55;
      c1 = x1 + bend;
      c2 = x2 - bend;
    }
    return {
      key,
      label: `${key} refers to ${link.to.table}.${link.to.column}`,
      path: `M${x1},${from.y} C${c1},${from.y} ${c2},${to.y} ${x2},${to.y}`,
      start: { x: x1, y: from.y },
    };
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

      {#each edges as edge (edge.key)}
        <path class="edge" d={edge.path} marker-end="url(#er-arrow)"
          ><title>{edge.label}</title></path
        >
        <circle class="dot" cx={edge.start.x} cy={edge.start.y} r="2.5" />
      {/each}

      {#each boxes as box (box.name)}
        <g>
          <rect class="table" x={box.x} y={box.y} width={COLUMN_WIDTH} height={box.height} rx="5" />
          <path
            class="heading"
            d="M{box.x},{box.y + ROW_HEIGHT} v{-ROW_HEIGHT + 5} a5,5 0 0 1 5,-5 h{COLUMN_WIDTH -
              10} a5,5 0 0 1 5,5 v{ROW_HEIGHT - 5} z"
          />
          <text class="name" x={box.x + 10} y={box.y + ROW_HEIGHT / 2}>{box.name}</text>
          {#each box.columns as column (column.name)}
            <text class="column" class:links={column.links} x={box.x + 10} y={column.y}>
              {column.name}
            </text>
            <text class="type" x={box.x + COLUMN_WIDTH - 10} y={column.y}>{column.type}</text>
          {/each}
          <rect
            class="outline"
            x={box.x}
            y={box.y}
            width={COLUMN_WIDTH}
            height={box.height}
            rx="5"
          />
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

  .edge {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.4;
    opacity: 0.75;
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
