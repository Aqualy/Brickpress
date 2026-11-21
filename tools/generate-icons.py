# Regenerates the Lucide icon subset and crates/brickpress-app/src/icons.rs.
# Usage: python tools/generate-icons.py <lucide-static>/font/codepoints.json <lucide-static>/font/lucide.ttf \n#          crates/brickpress-app/assets/fonts/lucide.ttf crates/brickpress-app/src/icons.rs
# Glyphs are re-centred in their advance (see center_glyphs).
# Add names to the list below to make more icons available. Needs fontTools.
import json, sys
from fontTools import subset
from fontTools.ttLib import TTFont
codepoints = json.load(open(sys.argv[1], encoding='utf8'))
names = """menu ellipsis-vertical ellipsis box boxes plus minus mouse-pointer-2 hand grid-3x3 undo-2 redo-2 rotate-cw rotate-ccw copy
trash-2 save folder-open file file-plus upload download search chevron-down chevron-up chevron-right chevron-left x eye eye-off
lock-keyhole lock-keyhole-open scan info check sliders-horizontal layers refresh-cw stamp bring-to-front send-to-back grip-vertical
flip-horizontal-2 flip-vertical-2 image image-plus palette zoom-in zoom-out maximize keyboard bot plug circle-help command
camera sun moon monitor sparkles triangle-alert circle-check circle-x history clipboard-paste scissors dices move ruler
paintbrush droplet square circle panel-left panel-right pencil folder bookmark-plus package printer pen-tool frame
square-dashed-mouse-pointer arrow-up arrow-down arrow-left arrow-right shapes blend scaling crosshair focus orbit rotate-3d
view house settings log-out external-link file-image file-code file-json-2 pipette swatch-book layout-grid mountain""".split()
missing = [n for n in names if n not in codepoints]
if missing:
    print("missing:", missing, file=sys.stderr)
names = [n for n in names if n in codepoints]

def center_glyphs(font):
    # Lucide's font drops each icon's left side bearing, so the glyphs sit at
    # the left of their 1000-unit advance. Centre every glyph in its advance
    # (vertically they are already on Lucide's 24-unit grid) and make every
    # vertical metric span exactly the em, so UI code can centre the line box.
    from fontTools.pens.boundsPen import BoundsPen
    from fontTools.pens.transformPen import TransformPen
    from fontTools.pens.ttGlyphPen import TTGlyphPen
    glyf, hmtx, gs = font['glyf'], font['hmtx'], font.getGlyphSet()
    for name in font.getGlyphOrder():
        bounds = BoundsPen(gs)
        gs[name].draw(bounds)
        if bounds.bounds is None:
            continue
        x0, _, x1, _ = bounds.bounds
        advance = hmtx[name][0]
        dx = round(advance / 2 - (x0 + x1) / 2)
        if dx == 0:
            continue
        pen = TTGlyphPen(gs)
        gs[name].draw(TransformPen(pen, (1, 0, 0, 1, dx, 0)))
        glyf[name] = pen.glyph()
        glyf[name].recalcBounds(glyf)
        hmtx[name] = (advance, glyf[name].xMin)
    em = font['head'].unitsPerEm
    font['hhea'].ascent, font['hhea'].descent, font['hhea'].lineGap = em, 0, 0
    os2 = font['OS/2']
    os2.sTypoAscender, os2.sTypoDescender, os2.sTypoLineGap = em, 0, 0
    os2.usWinAscent, os2.usWinDescent = em, 0
font = TTFont(sys.argv[2])
for tag in ("GSUB", "GPOS", "GDEF"):
    if tag in font: del font[tag]
options = subset.Options()
options.layout_features = []
options.name_IDs = ['*']
options.notdef_outline = True
sub = subset.Subsetter(options)
sub.populate(unicodes=[codepoints[n] for n in names])
sub.subset(font)
center_glyphs(font)
font.save(sys.argv[3])
def ident(n):
    return n.upper().replace('-', '_')
with open(sys.argv[4], 'w', encoding='utf8', newline='\n') as out:
    out.write("//! Lucide icons (ISC licence), as code points in the bundled icon font.\n//! Generated from lucide-static codepoints; the font is subset to these glyphs.\n\n")
    out.write("#![allow(dead_code)]\n\n")
    for n in names:
        out.write("pub const " + ident(n) + ": &str = \"" + chr(92) + "u{" + format(codepoints[n], "x") + "}\";" + chr(10))
print(len(names), "icons")
