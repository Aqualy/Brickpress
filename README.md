# Form & Impression

A local, browser-based studio for individual LEGO tile compositions and simulated letterpress impressions. Built with SvelteKit, Svelte 5, TypeScript, Tailwind CSS, shadcn-svelte, Bits UI, Paneforge, and Lucide Svelte. The artboard is custom Svelte/SVG; Canvas is used only to rasterize PNG exports. No accounts or backend.

## Run

```sh
npm install
npm run dev
```

Open the local URL printed in the terminal. The full sidebar layout is available on desktop; below 1,000 pixels the Pieces and Inspector buttons open accessible drawers.

```sh
npm run check
npm run validate:catalog
npm test
npm run build
npm run preview
```

The production build is a static site in `build/`. Serve it with an ordinary static web server. Node 20.19 or newer is required.

## Working in the studio

- Pick an essential piece, or use **Piece filters** to browse **All pieces**. Vertical categories show the full category catalog. Thumbnails preview the active ink. Click the board to stamp a piece, or drag it from the palette. Choosing a piece focuses the artboard: arrows move the placement cursor, Enter stamps, and **R** rotates.
- **V** selects, **B** places and **H** pans. Shift-click adds to selection; dragging empty board space creates a box selection. Arrows move selected pieces one stud; Shift-arrows move four studs.
- **R** rotates a selection. Delete/Backspace deletes; Ctrl/Command-D duplicates; Ctrl/Command-C/V copies and pastes; Ctrl/Command-A selects all unlocked visible pieces.
- Ctrl/Command-Z undoes; Ctrl/Command-Shift-Z redoes. Dragging commits exactly one history entry. History keeps the last 100 document transactions.
- Space-drag pans. **H** plus arrows pans with the keyboard. The wheel zooms around the pointer. Both zoom displays share the same view; the corner icon fits the artboard, percentage buttons reset to 100%, and the main menu includes 200%.
- Drag the separators to resize desktop panels, or focus a separator and use the arrow keys. **Reset panel widths** in the main menu restores the reference proportions. Below 1,000 pixels the same panels become modal drawers, keeping inspector choices intact.
- Each color is an ink pass. Recoloring selected pieces moves them into a matching or new pass. Use pass actions to recolor, lock, reorder, or delete. Names can be edited directly; expand passes in **Layers** to select individual pieces.
- **Properties** contains inks, passes, paper, and four primary print controls. Open **Advanced print settings** for the other seven controls, presets, and reseeding. Registration error displays the maximum per-axis offset in millimeters, using the existing normalized simulation parameter.
- **Grid** contains style, intervals, symmetry, overlap, and physical mode. Optional embossed studs are a Design-mode editor guide. The appearance preference is saved separately from projects and never included in exports; Flat remains the default.
- Physical mode permits only catalog pieces with standard, one-plate printing surfaces. The two non-standard-height pieces stay visible but disabled in the palette. Existing non-standard pieces must be removed before enabling that mode.
- Optional symmetry previews mirrored placements and creates pieces only when stamped. Explicit overlap can be enabled; it cannot be disabled until overlapping pieces are separated.

## Files and recovery

**Save** downloads an editable `.legopress.json` file containing all geometry references, positions, rotations, mirror flags, piece seeds, pass seeds, swatches, paper and print settings. **Open** validates a project before replacing the current document. New documents prompt when file-unsaved changes exist.

Changes also autosave to this browser's localStorage. Reloading recovers the most recent document. Keep a project file for durable storage: clearing browser data removes device autosaves. Copy/paste uses an editor-local clipboard.

## Rendering and exports

The canonical catalog is imported directly from `lego-letterpress-kit/lego-letterpress-kit/letterpress-pieces.json`; the original kit remains intact. Its geometry, fill rules, `gridMask`, allowed rotations, height flags and surface scale are used throughout. Malformed entries are omitted with a warning. Approximate catalog geometry is identified in the palette and selection information.

Placements use the top-left of the **rotated footprint**. Geometry rotates around its logical center. When a non-square piece changes footprint parity, its origin snaps to the nearest whole stud; half-stud centers cannot always be preserved on an integer grid. Collision uses cached rotated/mirrored masks to find nearby pieces, then compares the catalog SVG surfaces, including curved cutouts. Surface checks use the renderer’s nominal piece gap and cache results for repeated placements.

Design output preserves separate vector paths with a constant nominal 0.2 mm seam. Shared SVG masks inset every straight and curved edge by 0.1 mm, keeping placement previews, the artboard and transparent design exports consistent regardless of piece size. Print output retains the catalog surface scale and gives **every piece its own seeded SVG filter**, combining procedural high-frequency tooth/dropout, low-frequency coverage variation, erosion, displacement and pressure-dependent opacity. A separate seeded offset applies to each whole color pass. Paper grain and fibers are procedural and do not distress the composition. All eleven press controls and six presets modify these rendering parameters.

**Export** opens the Inspector’s Export tab and supports clean vector SVG, procedural-filter print SVG, PNG at 1×/2×/4× or custom pixel width, and editable JSON. Backgrounds can be paper or transparent. Grid is excluded by default; selections, rulers and editor controls never export. PNG uses the same SVG impression renderer at the requested resolution. Exports are limited to 12,000 pixels per side and 40 megapixels to respect browser memory.

SVG filter rendering differs between vector applications; use PNG for a consistent print image. This is a surface/ink simulation, not a paper deformation model or a guarantee of physical print results. PDF, manufacturing instructions and native 3D editing are outside this release.

## Architecture and tests

`src/lib/catalog`, `geometry`, `history`, `persistence`, `printing` and `export` contain independently testable logic. The generated shadcn-svelte foundation lives in `src/lib/components/ui`. Its buttons, inputs, tabs, popovers, sheets, and resizable panels use custom compact styling rather than the registry theme. Bits UI directly handles palette tooltips and persistent drawer content; Paneforge owns panel resizing. `Icon.svelte` maps interface names to Lucide icons. Tailwind is compiled through its Vite plugin.

A per-page `Editor` class owns immutable document transactions and reactive interaction state. Palette, canvas, properties, paper, press settings, layers and export are separate Svelte components. Keyed piece components and transient drag transforms keep pointer movement out of document history and autosave.

Unit tests cover the supplied catalog, lookup and malformed entries, footprint/mask rotation, collisions, mirrored placements, serialization, validation, undo/redo, deterministic noise, pass offsets and SVG exports. Browser tests cover all 41 pieces, core editing, multiple selections, passes, physical mode, symmetry, save/load/recovery, PNG/SVG output, responsive layout down to 320 CSS pixels, keyboard placement, tab and popover focus, pointer and keyboard panel resizing, nested drawer popovers, synchronized zoom, independent stud guides, accessibility scans, and a 2,000-piece document.

```sh
npx playwright install chromium
npm run test:e2e
```

This is an unofficial creative tool. LEGO is a trademark of the LEGO Group; no official branding or product imagery is included.

## Reference UI boundaries

The editor follows the supplied Figma-style interface without simulated window chrome. Existing project artwork, ink colors, paper presets, dimensions, and export rendering remain compatible. A5/A-series page sizing, ink-pass drag-reordering, and browser fullscreen are deferred: current size presets, pass reorder commands, and Fit remain available.

Accessibility targets WCAG 2.2 AA with keyboard editing, Bits UI popovers and modal drawers, focus restoration, labels, contrast, and non-color state indicators. Browser tests run axe scans across both modes, all inspector tabs, menus, and narrow drawers; manual visual and keyboard checks supplement those scans.
