//! SVG export.
//!
//! Design mode is pure vector: every catalog surface with the printing gap
//! cut in by a mask, as in 1.x. Print mode keeps each piece as a vector with
//! its seeded jitter, rotation and registration, and masks every ink pass with
//! the coverage texture rendered by the GPU print engine, embedded as PNG.

use crate::export::{export_dimensions, ExportError, ExportOptions};
use crate::print::{impression, jitter_matrix, paper_color, pass_offset};
use base64::Engine;
use brickpress_core::catalog::{catalog, Piece};
use brickpress_core::geometry::piece_matrix;
use brickpress_core::{PlacedPiece, PressDocument};
use std::collections::HashSet;
use std::fmt::Write;
use std::sync::Arc;

pub fn escape_xml(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

fn n(v: f64) -> String {
    let s = format!("{:.5}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

fn matrix(m: [f64; 6]) -> String {
    format!("matrix({} {} {} {} {} {})", n(m[0]), n(m[1]), n(m[2]), n(m[3]), n(m[4]), n(m[5]))
}

fn mask_id(piece: &Piece) -> String {
    format!("design-gap-{}", piece.id.replace(|c: char| !c.is_ascii_alphanumeric(), "_"))
}

/// The transparent inward offset of a canonical silhouette.
fn design_mask(piece: &Piece) -> String {
    format!(
        r#"<mask id="{}" maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" x="0" y="0" width="{}" height="{}" mask-type="luminance"><path d="{}" fill="white" fill-rule="{}" stroke="black" stroke-width="{}" stroke-linejoin="round"/></mask>"#,
        mask_id(piece),
        piece.footprint.width_studs,
        piece.footprint.height_studs,
        escape_xml(&piece.geometry.path),
        piece.geometry.fill_rule.svg(),
        n(catalog().design_gap_studs()),
    )
}

/// A copy of the document with only one pass, inked white: its coverage
/// render is the pass's print mask.
pub fn pass_coverage_document(doc: &PressDocument, pass_index: usize) -> PressDocument {
    let mut copy = doc.clone();
    for (i, pass) in copy.passes.iter_mut().enumerate() {
        pass.visible = i == pass_index && pass.visible;
        pass.color = "#ffffff".into();
    }
    copy
}

/// Renders each visible pass's print coverage (white ink, no paper) as a PNG,
/// for [`render_svg`] in print mode.
pub fn print_pass_masks(
    device: &crate::wgpu::Device,
    queue: &crate::wgpu::Queue,
    renderer: &crate::board2d::Renderer2D,
    doc: &PressDocument,
    options: &ExportOptions,
) -> Result<Vec<Option<Vec<u8>>>, ExportError> {
    let mask_options = ExportOptions { paper: false, grid: false, ..*options };
    doc.passes
        .iter()
        .enumerate()
        .map(|(i, pass)| {
            if !pass.visible || pass.pieces.is_empty() {
                return Ok(None);
            }
            let single = pass_coverage_document(doc, i);
            let image = crate::export::render_rgba(
                device,
                queue,
                renderer,
                &single,
                brickpress_core::persistence::preferences::GridAppearance::Flat,
                &mask_options,
            )?;
            crate::export::encode_png(&image).map(Some)
        })
        .collect()
}

/// Renders the SVG. In print mode, `pass_masks[i]` is the PNG coverage mask
/// of pass `i` (or `None` for hidden or empty passes).
pub fn render_svg(
    doc: &PressDocument,
    options: &ExportOptions,
    pass_masks: &[Option<Vec<u8>>],
) -> Result<String, ExportError> {
    let (out_w, out_h) = export_dimensions(doc, options.width)?;
    let (w, h) = (doc.board.width, doc.board.height);
    let printed = options.mode == crate::export::ExportMode::Print;
    let scale = catalog().surface_scale();
    let mut defs = String::new();
    let mut art = String::new();
    let mut masked = HashSet::new();
    for (index, pass) in doc.passes.iter().enumerate() {
        if !pass.visible {
            continue;
        }
        let offset = if printed { pass_offset(pass, &doc.print_settings) } else { (0.0, 0.0) };
        let mut paths = String::new();
        for p in pass.pieces.iter() {
            let Some(piece) = catalog().piece(&p.piece_id) else { continue };
            paths.push_str(&piece_path(doc, piece, p, &pass.color, printed, scale, offset, &mut defs, &mut masked));
        }
        let mask = match (printed, pass_masks.get(index).and_then(|m| m.as_ref())) {
            (true, Some(png)) => {
                let id = format!("print-coverage-{index}");
                let _ = write!(
                    defs,
                    r#"<mask id="{id}" maskUnits="userSpaceOnUse" x="0" y="0" width="{w}" height="{h}" mask-type="alpha"><image href="data:image/png;base64,{}" x="0" y="0" width="{w}" height="{h}" preserveAspectRatio="none"/></mask>"#,
                    base64::engine::general_purpose::STANDARD.encode(png)
                );
                format!(r#" mask="url(#{id})""#)
            }
            _ => String::new(),
        };
        let _ = write!(art, r#"<g data-ink-pass="{}"{mask}>{paths}</g>"#, escape_xml(&pass.name));
    }
    let background = if options.paper {
        let [r, g, b] = paper_color(&doc.paper);
        format!(r##"<rect width="{w}" height="{h}" fill="#{r:02x}{g:02x}{b:02x}"/>"##)
    } else {
        String::new()
    };
    let grid = if options.grid {
        format!(
            r##"<defs><pattern id="export-grid" width="1" height="1" patternUnits="userSpaceOnUse"><path d="M1 0H0V1" fill="none" stroke="#7d8177" stroke-width=".01"/></pattern></defs><rect width="{w}" height="{h}" fill="url(#export-grid)"/>"##
        )
    } else {
        String::new()
    };
    let desc = if printed {
        "Individual catalog tile impressions. 1 stud = 8 mm. Seeded print simulation; coverage masks rendered by Brickpress."
    } else {
        "Individual catalog tile impressions. 1 stud = 8 mm. Vector design."
    };
    Ok(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{out_w}" height="{out_h}" viewBox="0 0 {w} {h}"><title>{}</title><desc>{desc}</desc><defs>{defs}</defs>{background}{art}{grid}</svg>"#,
        escape_xml(&doc.name)
    ))
}

#[allow(clippy::too_many_arguments)]
fn piece_path(
    doc: &PressDocument,
    piece: &Piece,
    p: &PlacedPiece,
    color: &str,
    printed: bool,
    scale: f64,
    offset: (f64, f64),
    defs: &mut String,
    masked: &mut HashSet<Arc<str>>,
) -> String {
    let path = escape_xml(&piece.geometry.path);
    let rule = piece.geometry.fill_rule.svg();
    if printed {
        let imp = impression(p, &doc.print_settings);
        let m = jitter_matrix(piece_matrix(piece, p, scale), &imp, p, offset);
        format!(
            r#"<path data-piece-id="{}" d="{path}" transform="{}" fill="{color}" fill-rule="{rule}"/>"#,
            piece.id,
            matrix(m)
        )
    } else {
        if masked.insert(piece.id.as_str().into()) {
            defs.push_str(&design_mask(piece));
        }
        format!(
            r#"<path data-piece-id="{}" d="{path}" transform="{}" fill="{color}" fill-rule="{rule}" mask="url(#{})"/>"#,
            piece.id,
            matrix(piece_matrix(piece, p, 1.0)),
            mask_id(piece)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::ExportMode;
    use brickpress_core::persistence::document::create_document;

    fn options(mode: ExportMode, paper: bool) -> ExportOptions {
        ExportOptions { mode, paper, grid: false, width: 2048 }
    }

    #[test]
    fn exports_individual_surfaces_and_honors_transparency() {
        let mut doc = create_document(false);
        doc.passes[0].pieces =
            Arc::new(vec![PlacedPiece::new("a", "3070", 0, 0, 0, 1), PlacedPiece::new("b", "2431", 2, 0, 0, 1)]);
        let svg = render_svg(&doc, &options(ExportMode::Design, false), &[]).unwrap();
        assert_eq!(svg.matches("<path data-piece-id=").count(), 2);
        assert!(!svg.contains("<rect"));
        assert!(svg.contains(r#"width="2048""#));
        assert_eq!(svg.matches("<mask ").count(), 2, "one gap mask per catalog shape");
    }

    #[test]
    fn print_mode_masks_passes_and_skips_hidden_ones() {
        let mut doc = create_document(true);
        doc.passes[1].visible = false;
        let masks = vec![Some(vec![1, 2, 3]), None, Some(vec![4])];
        let svg = render_svg(&doc, &options(ExportMode::Print, true), &masks).unwrap();
        assert_eq!(svg.matches("data-ink-pass=").count(), 2);
        assert_eq!(svg.matches("print-coverage-").count(), 4, "two masks, each defined and used");
        assert!(svg.contains("<rect"));
    }

    #[test]
    fn numbers_are_compact() {
        assert_eq!(n(1.0), "1");
        assert_eq!(n(-0.000001), "0");
        assert_eq!(n(0.975), "0.975");
    }
}
