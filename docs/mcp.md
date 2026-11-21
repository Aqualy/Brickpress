# Brickpress for agents (MCP)

Brickpress 2 includes a [Model Context Protocol](https://modelcontextprotocol.io)
server, so an agent such as Claude can compose boards, look at its work, and
save or export it. It is the same `brickpress` executable:

```sh
brickpress mcp                 # headless: edit projects under the current folder
brickpress mcp --root designs  # headless, with files kept inside ./designs
brickpress mcp --live          # drive the Brickpress window that is open now
```

## Setup

Claude Code:

```sh
claude mcp add brickpress -- brickpress mcp --root /path/to/designs
claude mcp add brickpress-live -- brickpress mcp --live
```

Any client that runs stdio servers works the same way. Use an absolute path to
`brickpress` (`brickpress.exe` on Windows) if it is not on `PATH`.

## Modes

**Headless** (default). The server keeps its own document in memory. It starts
with an empty 16×16 board. `document_open`, `document_save` and `export_image`
read and write files, but only inside `--root`: a path that resolves outside it
is refused. A GPU is used for `render_preview` and exports when present. On
machines without one, the platform's software rasterizer is used instead
(WARP on Windows, llvmpipe on Linux).

**Live** (`--live`). The server forwards every tool call to the open Brickpress
window over a per-user local socket: a named pipe on Windows, or a Unix socket
elsewhere. Live mode needs Help ▸ **Allow Agent Connections** to be switched on;
it is off by default.

While it is on, the status bar shows whether an agent is connected and when it
is making changes. Each agent tool call is one undo step, so the person can
undo an agent's edit with a single Ctrl/⌘ Z. In live mode, agents cannot open,
save or overwrite the person's project; those stay in the window. Exports
still go inside `--root`. Edits wait while a dialog is open in the window.
Calls act on the document in the window's active tab (one undo step in that
tab's history) and fail while no document is open.

## Tools

| Tool | What it does |
| --- | --- |
| `document_new`, `document_open`, `document_save` | Headless file handling (within `--root`) |
| `document_info` | Board, ink passes (ids, names, colours), counts, settings |
| `document_get` | The full project JSON (version-1 format) |
| `document_rename` | Rename the project |
| `catalog_list`, `catalog_piece` | Pieces, sizes, allowed rotations, occupancy masks |
| `pieces_place` | Place pieces; returns per-piece results with reasons |
| `pieces_place_grid` | Draw with 1×1 pieces from rows of characters |
| `pieces_query`, `pieces_move`, `pieces_remove` | Inspect and edit placed pieces |
| `pass_add`, `pass_update`, `pass_remove` | Ink passes: colour, name, order, visibility, lock |
| `board_resize`, `options_set` | Board size, grid, symmetry, overlap, physical mode |
| `print_settings_set`, `paper_set`, `print_reseed` | The simulated press and paper |
| `board_ascii` | A text picture of the board |
| `render_preview` | A PNG image of the design or print, returned to the agent |
| `export_image` | Write a PNG or SVG (headless: within `--root`) |
| `undo`, `redo` | History |

### Coordinates and rules

- Coordinates are whole studs. x grows to the right and y grows downwards.
- A piece's `x`/`y` is the top-left corner of its **rotated** footprint.
- Rotations are clockwise: 0, 90, 180 or 270.
- Pieces may not overlap unless `allowOverlap` is on. Curved pieces may share a
  stud when their surfaces fit together, for example a quarter round inside a
  quarter ring.
- Ink passes print in order, and later passes overprint earlier ones.

### Grid notation (`pieces_place_grid`)

One character per stud:

| Character | Piece |
| --- | --- |
| `#` | 1×1 tile |
| `0` | 1×1 quarter round, curve at bottom-right |
| `1` | 1×1 quarter round, curve at bottom-left |
| `2` | 1×1 quarter round, curve at top-left |
| `3` | 1×1 quarter round, curve at top-right |
| `.` or space | empty |

Any other character needs a `legend` entry, such as `{"o": {"piece": "98138"}}`
for round 1×1 tiles. For example, an "A":

```json
{ "x": 2, "y": 2, "rows": ["1###0", "#...#", "#####", "#...#", "#...#"] }
```

### A typical session

1. `document_info`, then `catalog_list`.
2. `pass_add` for each colour, or use the starter passes.
3. `pieces_place` or `pieces_place_grid`. Read the per-piece `reason` for
   anything that was not placed.
4. `render_preview` with `mode: "design"` to check placement, and with
   `mode: "print"` to see the letterpress result.
5. `document_save` and/or `export_image`.
