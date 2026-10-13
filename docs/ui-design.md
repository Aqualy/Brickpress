# Brickpress UI design

## Design review: Brickpress

### Summary

Overall rating: **Good** for the tested browser and Windows webview interfaces;
platform-specific macOS and Linux manual checks remain outstanding.

Brickpress is a desktop composition studio: choose modular shapes, arrange them on a
studded plate, and explore a simulated letterpress impression. Its distinctive element
is the relationship between precise tile geometry and textured printed artwork. The
surrounding controls should recede, while making document state and editing actions clear.

This pass applies the [Apple design skill](https://github.com/dickwu/apple-design-skill)
to the existing Svelte/Tauri interface. It preserves the compact editor and translates
HIG principles into webview controls rather than copying an Apple app's appearance.

### Improvements addressed

- **High — appearance support:** fixed white surfaces could clash with native dark controls.
  Semantic colors now cover the toolbar, panels, inputs, popovers, drawers, notices,
  notifications, and workspace. Artwork retains its actual ink and paper colors.
  `color.md › Best practices`: “Make sure all your app’s colors work well in light, dark, and increased contrast contexts.”
  [HIG Color](https://developer.apple.com/design/human-interface-guidelines/color).
- **Medium — command discovery:** Select and Place were absent from the overflow menu.
  The menu now groups Document and Tools and view commands, exposes editing commands at
  every width, and displays shortcuts. Frequent actions retain their toolbar positions.
  `toolbars.md › Best practices`: “Choose items deliberately to avoid overcrowding.”
  [HIG Toolbars](https://developer.apple.com/design/human-interface-guidelines/toolbars).
- **Medium — feedback:** short-lived notifications could disappear while being read.
  Hover and keyboard focus pause dismissal; an explicit close control restores the
  previous keyboard target. Drawer feedback occupies its own row inside the dialog,
  leaving controls unobscured and keeping its dismiss button in the focus trap.
  Notifications never take focus on arrival.
  `focus-and-selection.md › Best practices`: “Avoid changing focus without people’s interaction.”
  [HIG Focus and selection](https://developer.apple.com/design/human-interface-guidelines/focus-and-selection).
- **Low — consistency:** shared controls use semantic surface colors and relative text sizes.
  Segmented choices share inset selected states; navigational inspector tabs retain their
  underline. Floating overlays have restrained elevation while editor panels stay flat.
  The desktop toolbar shows the document name and saved/edited state when space permits.

### Token system and contrast

These are custom semantic colors, not hard-coded replicas of Apple system colors.
Ratios use WCAG sRGB relative luminance. Small text meets 4.5:1 and control boundaries
meet 3:1 against the listed surface. Decorative panel dividers are intentionally quieter.

| Role                | Light     | Dark      | Contrast, light / dark          |
| ------------------- | --------- | --------- | ------------------------------- |
| Panel surface       | `#F8F8FA` | `#232428` | —                               |
| Content             | `#202124` | `#F1F2F5` | 15.18 / 13.85 on panel          |
| Secondary content   | `#60646C` | `#B5BAC5` | 5.60 / 7.97 on panel            |
| Selection label     | `#0057D8` | `#9BBCFF` | 5.31 / 5.83 on selected surface |
| Primary action fill | `#0057D8` | `#2365D8` | 6.26 / 5.35 with white text     |
| Control boundary    | `#858B96` | `#858B98` | 3.43 / 3.97 on control surface  |

Supporting tokens include control surfaces `#FFFFFF` / `#2C2E34`, workspace
`#E9EBEF` / `#191B20`, and selected surfaces `#E4EDFC` / `#293C5B`.
Dark artwork thumbnails have a neutral light mat so black ink remains visible.
Appearance follows the OS; no additional theme switch is needed.
`dark-mode.md › Best practices`: “Test your content to make sure that it remains comfortably legible in both appearance modes.”
[HIG Dark Mode](https://developer.apple.com/design/human-interface-guidelines/dark-mode).

### Typography and layout

The macOS system font comes first, with self-hosted Inter elsewhere. Body text is 13px,
compact labels 12px, utility text 11px, and headings 13–14px at the default browser size.
IBM Plex Mono distinguishes coordinates and measurements. Shared text uses rem units
to support larger default text. Controls remain 32px with 4–6px corners, and can grow
when text wraps. Native OS window decoration remains intact.

The desktop layout keeps pieces to the left, the artwork in the center, and contextual
properties to the right; below 1000px both sidebars become focus-managed drawers.

```text
Regular desktop
+----------------------------------------------------------------+
| Menu  Document  History  Tools  Name / state  Design Print 3D   |
+-------------+----------------------------------+---------------+
| Library     |                                  | Inspector tabs|
| Search      |          Studded artboard        | Inks / passes |
| Categories  |        and printed artwork       | Context       |
| Shapes      |                                  | Paper / size  |
| Presets     |       Size / zoom / camera       | Export        |
+-------------+----------------------------------+---------------+
| Save status / placement feedback                               |
+----------------------------------------------------------------+

Compact
+---------------------------+
| Menu Pieces Tools Inspect |
| Design | Print | 3D       |
+---------------------------+
|                           |
|       Artboard             |
|                           |
|      Size / zoom          |
+---------------------------+
| Status                    |
+---------------------------+
  Pieces and Inspector open
  as dismissible drawers.
```

### Craft notes and what works

The palette is deliberately subdued because the red, yellow, blue, green, and black
belong to the inks. The paper, modular silhouettes, and optional embossed studs are
the product's identity. A generic glass dashboard would compete with them; solid panels
and minimal shadows suit precise editing better. This is a product-specific design
judgment, not a requirement to mimic Apple's current materials.

Existing geometry, print parameters, document formats, exports, undo, and keyboard
placement remain authoritative. No decorative motion was added; existing transitions
respect reduced motion. Increased contrast strengthens boundaries, reduced transparency
removes translucent overlays, and forced colors retain selection outlines.

### Platform notes and validation

Bits UI supplies tab, popover, and drawer interaction; Tauri retains native window
decoration and document dialogs. Browser/Windows/Linux controls use familiar web
keyboard patterns. This is not a rewrite into AppKit or a claim of complete HIG compliance.
Actual macOS VoiceOver, native text settings, and Linux desktop theme behavior still
require platform-specific manual verification.

Automated coverage is in `tests/appearance.spec.ts`: light/dark views and inspector tabs,
menus, focus restoration, 320px drawers, notification reading, enlarged text, and
serialization preservation. Screenshots are generated under ignored `artifacts/appearance/`.
See the current implementation report for executed test results.

The completed browser run passed all 49 original scenarios, and all seven appearance
scenarios passed after the final drawer adjustment: 50 unique scenarios verified.
Svelte analysis reported no errors
or warnings, all 90 unit tests passed, and catalog validation accepted all 41 shapes.

The updated Windows desktop test build passed all 18 automated native scenarios;
one interactive real-OS-dialog check was skipped. The production frontend build
succeeded and was checked to exclude the desktop test bridge. No installers were
published by this UI pass.
