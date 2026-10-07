# LEGO Letterpress Digital Piece Kit

A developer-ready 2D piece library for building a grid-based editor inspired by LEGO letterpress printing.

This catalog is original work created by Tiebo (Aqualy), the Brickpress project
owner. The historical kit name does not imply LEGO Group authorship. See the
project's [catalog provenance notice](../../NOTICE.md) for ownership and licensing.

## Files

- `letterpress-pieces.json` — canonical data file, 41 pieces.
- `letterpress-pieces.ts` — typed TypeScript export.
- `render-svg.ts` — minimal SVG placement/render helper.
- `pieces-contact-sheet.svg` — visual index of all 41 silhouettes.
- `thumbnails/*.svg` — one thumbnail per Design ID.

## Coordinate model

- 1 stud = 1 logical unit.
- Nominal LEGO System stud pitch = 8.0 mm.
- A regular one-stud part is about 7.8 mm across, leaving about 0.2 mm between neighboring pieces.
- The library therefore keeps geometry in exact stud units and recommends `defaultSurfaceScale = 0.975` when drawing.
- Do not merge adjacent pieces. Their seams are part of the letterpress look.

## Core toolbar

Start the UI with these pieces exposed:

`3070`, `3069`, `98138`, `24246`, `1126`, `14769`, `25269`, `1748`, `27925`, `35787`.

That small alphabet is enough for most of the visual language in the reference prints: square cells, bars, dots, D-shapes, capsules, circles, quarter circles, semicircles, macaroni curves and triangles.

## Collision

Use `gridMask`, not the SVG path, for snapping/collision. This preserves LEGO-like placement behavior while allowing a print surface to curve or cut away inside the footprint.

## Physical-height filter

A strict physical letterpress setup should use pieces with:

```ts
piece.physicalLetterpressReady && piece.surfaceHeightPlates === 1;
```

`68869` and `74169` are included for digital completeness but are 2/3-brick-high special elements, so they do not naturally print in the same plane as standard tiles.

## Print simulation

Recommended order:

1. Snap the piece to the stud grid.
2. Apply rotation.
3. Scale its print surface around its own center to create the physical seam.
4. Generate a unique random texture seed for that piece.
5. Apply small per-piece ink dropout, edge erosion and position/rotation jitter.
6. Composite pieces in one ink pass.
7. For multi-color work, render each color as a separate pass with a tiny global registration offset.
8. Apply paper texture last.

Do not put one generic grunge bitmap over the finished composition. The references read as physical letterpress because neighboring pieces fail and pick up ink differently.

Suggested ranges are included in the JSON under `rendering.recommendedPrintSimulation`.

## SVG use

Every piece exposes a local SVG path:

```ts
const piece = getPiece("25269");

<path
  d={piece.geometry.path}
  fill="currentColor"
  fillRule={piece.geometry.fillRule}
/>
```

The local SVG coordinate system is measured in studs and starts at `(0, 0)`.

## Fidelity

The simple geometric families are represented directly as their intended top silhouettes.

A few rare pieces are marked `geometry.fidelity = "approximate"`. These are deliberately called out in the data rather than pretending to be CAD-accurate. If you want a physically exact mode, replace those paths with orthographic top projections from the corresponding LDraw files.

## Reference catalog

The Design IDs and family names were checked against Brick Architect's LEGO Parts Guide. The two main relevant families are:

- Basic tiles: https://brickarchitect.com/parts/category-18
- Rounded tiles: https://brickarchitect.com/parts/category-77
- Wedge tiles: https://brickarchitect.com/parts/category-70

Individual entries are linked under each piece's `reference` field.

For exact CAD geometry, LDraw is the better next source because its part library is intended for geometric representation.

## Trademark note

LEGO® is a trademark of the LEGO Group. This kit is an unofficial reference and is not sponsored, authorized or endorsed by the LEGO Group. No official LEGO artwork or product imagery is included.
