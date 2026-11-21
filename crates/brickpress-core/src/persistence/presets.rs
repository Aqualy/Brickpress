//! Composition presets: reusable groups of pieces, stored as a portable library
//! (`.brickpress-presets.json`).

use super::document::{create_document, make_pass, parse_document_value, DocumentError, MAX_BOARD, MAX_PASSES};
use crate::document::{new_id, Board, PlacedPiece, PressDocument};
use crate::geometry::{bounds, bounds_of};
use crate::spatial::OccupancyIndex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

pub const MAX_PRESETS: usize = 100;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PresetPass {
    pub name: String,
    pub color: String,
    pub pieces: Vec<PlacedPiece>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompositionPreset {
    pub version: u32,
    pub id: String,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub passes: Vec<PresetPass>,
}

impl CompositionPreset {
    pub fn piece_count(&self) -> usize {
        self.passes.iter().map(|p| p.pieces.len()).sum()
    }
}

fn err<T>(message: &str) -> Result<T, DocumentError> {
    Err(DocumentError(message.into()))
}

/// Captures visible pieces (optionally only `selected`) with margins trimmed.
pub fn capture_preset(
    doc: &PressDocument,
    name: &str,
    selected: Option<&HashSet<&str>>,
) -> Result<CompositionPreset, DocumentError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return err("Give the preset a name of 1–80 characters.");
    }
    let passes: Vec<PresetPass> = doc
        .passes
        .iter()
        .filter(|p| p.visible)
        .map(|pass| PresetPass {
            name: pass.name.clone(),
            color: pass.color.clone(),
            pieces: pass.pieces.iter().filter(|p| selected.is_none_or(|s| s.contains(&*p.uid))).cloned().collect(),
        })
        .filter(|p| !p.pieces.is_empty())
        .collect();
    let Some(b) = bounds_of(passes.iter().flat_map(|p| &p.pieces)) else {
        return err("Place or select some visible pieces first.");
    };
    Ok(CompositionPreset {
        version: 1,
        id: new_id().to_string(),
        name: name.into(),
        width: b.width,
        height: b.height,
        passes: passes
            .into_iter()
            .map(|pass| PresetPass {
                pieces: pass.pieces.into_iter().map(|p| PlacedPiece { x: p.x - b.x, y: p.y - b.y, ..p }).collect(),
                ..pass
            })
            .collect(),
    })
}

/// Studs of empty board kept around a preset opened as its own document.
pub const PRESET_DOCUMENT_MARGIN: i32 = 2;

/// A new document holding `preset`: named after it, one ink pass per preset
/// pass (same names, colours and piece seeds), on a board fitted to the
/// preset with a small margin. Paper and print settings are the defaults of a
/// new project. Overlap is allowed only when the preset's pieces overlap.
pub fn document_from_preset(preset: &CompositionPreset) -> PressDocument {
    let mut doc = create_document(false);
    let margin = |size: i32| ((MAX_BOARD - size) / 2).clamp(0, PRESET_DOCUMENT_MARGIN);
    let (mx, my) = (margin(preset.width), margin(preset.height));
    doc.name = preset.name.chars().take(160).collect();
    doc.board =
        Board { width: (preset.width + 2 * mx).min(MAX_BOARD), height: (preset.height + 2 * my).min(MAX_BOARD) };
    doc.passes = preset
        .passes
        .iter()
        .take(MAX_PASSES)
        .map(|pass| {
            let mut ink = make_pass(&pass.color, &pass.name);
            ink.pieces = Arc::new(
                pass.pieces
                    .iter()
                    .map(|p| PlacedPiece { uid: new_id(), x: p.x + mx, y: p.y + my, ..p.clone() })
                    .collect(),
            );
            ink
        })
        .collect();
    if doc.passes.is_empty() {
        doc.passes = create_document(false).passes;
    }
    for pass in &doc.passes {
        if doc.swatches.len() < 200 && !doc.swatches.iter().any(|s| s.eq_ignore_ascii_case(&pass.color)) {
            doc.swatches.push(pass.color.clone());
        }
    }
    let all: Vec<PlacedPiece> = doc.all_pieces().map(|(_, p)| p.clone()).collect();
    doc.options.allow_overlap = !OccupancyIndex::empty(&doc).can_place(&all);
    doc
}

fn valid_preset_id(id: &str) -> bool {
    (1..=100).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub fn parse_presets(text: &str) -> Result<Vec<CompositionPreset>, DocumentError> {
    if text.len() > 20_000_000 {
        return err("The preset library is too large.");
    }
    let input: Value = serde_json::from_str(text).map_err(|_| DocumentError("This file is not valid JSON.".into()))?;
    let Some(list) = input.as_array().filter(|l| l.len() <= MAX_PRESETS) else {
        return err("Use a library of up to 100 presets.");
    };
    let mut ids = HashSet::new();
    let mut out = Vec::with_capacity(list.len());
    for preset in list {
        let id = preset.get("id").and_then(Value::as_str);
        let name = preset.get("name").and_then(Value::as_str);
        let passes = preset.get("passes").and_then(Value::as_array);
        let (Some(id), Some(name), Some(passes)) = (id, name, passes) else {
            return err("Invalid composition preset.");
        };
        if preset.get("version").and_then(Value::as_i64) != Some(1)
            || !valid_preset_id(id)
            || ids.contains(id)
            || name.trim().is_empty()
            || name.chars().count() > 80
        {
            return err("Invalid composition preset.");
        }
        ids.insert(id.to_string());
        // Validate the pieces as a document of the preset's own size.
        let mut doc = serde_json::to_value(create_document(false)).expect("serializable");
        doc["board"] = serde_json::json!({
            "width": preset.get("width").cloned().unwrap_or(Value::Null),
            "height": preset.get("height").cloned().unwrap_or(Value::Null),
        });
        doc["options"]["allowOverlap"] = Value::Bool(true);
        doc["passes"] = Value::Array(
            passes
                .iter()
                .map(|p| {
                    let mut pass = serde_json::to_value(make_pass(
                        p.get("color").and_then(Value::as_str).unwrap_or(""),
                        p.get("name").and_then(Value::as_str).unwrap_or(""),
                    ))
                    .expect("serializable");
                    pass["color"] = p.get("color").cloned().unwrap_or(Value::Null);
                    pass["name"] = p.get("name").cloned().unwrap_or(Value::Null);
                    pass["pieces"] = p.get("pieces").cloned().unwrap_or(Value::Null);
                    pass
                })
                .collect(),
        );
        let checked = parse_document_value(&doc)?;
        if checked.piece_count() == 0 {
            return err("A preset must contain pieces.");
        }
        out.push(CompositionPreset {
            version: 1,
            id: id.into(),
            name: name.trim().into(),
            width: checked.board.width,
            height: checked.board.height,
            passes: checked
                .passes
                .into_iter()
                .filter(|p| !p.pieces.is_empty())
                .map(|p| PresetPass { name: p.name, color: p.color, pieces: Arc::unwrap_or_clone(p.pieces) })
                .collect(),
        });
    }
    Ok(out)
}

pub fn serialize_presets(presets: &[CompositionPreset]) -> String {
    serde_json::to_string(presets).expect("presets always serialize")
}

#[derive(Clone, Debug, PartialEq)]
pub struct PresetRow {
    pub color: String,
    pub pass_index: usize,
    pub piece: PlacedPiece,
}

/// Places a preset at (x, y), rotating the whole group while keeping catalog
/// geometry and world-axis mirrors intact.
pub fn preset_rows(preset: &CompositionPreset, x: i32, y: i32, rotation: i32) -> Vec<PresetRow> {
    let swapped = rotation % 180 != 0;
    let mut rows = Vec::new();
    for (pass_index, pass) in preset.passes.iter().enumerate() {
        for (i, piece) in pass.pieces.iter().enumerate() {
            let b = bounds(piece);
            let (px, py) = match rotation {
                90 => (preset.height - b.y - b.height, b.x),
                180 => (preset.width - b.x - b.width, preset.height - b.y - b.height),
                270 => (b.y, preset.width - b.x - b.width),
                _ => (piece.x, piece.y),
            };
            let (mirror_x, mirror_y) =
                if swapped { (piece.mirror_y, piece.mirror_x) } else { (piece.mirror_x, piece.mirror_y) };
            rows.push(PresetRow {
                color: pass.color.clone(),
                pass_index,
                piece: PlacedPiece {
                    uid: format!("preset-ghost-{pass_index}-{i}").into(),
                    x: px + x,
                    y: py + y,
                    rotation: (piece.rotation + rotation) % 360,
                    mirror_x,
                    mirror_y,
                    ..piece.clone()
                },
            });
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persistence::document::{parse_document, serialize_document};
    use crate::spatial::can_place;

    fn tile(uid: &str, x: i32, y: i32, id: &str) -> PlacedPiece {
        PlacedPiece::new(uid, id, x, y, 0, 171)
    }
    fn none() -> HashSet<&'static str> {
        HashSet::new()
    }

    #[test]
    fn trims_margins_and_keeps_colors_order_and_mirrors() {
        let mut doc = create_document(false);
        doc.passes[0].pieces = Arc::new(vec![tile("red", 4, 3, "3070").with_mirror(true, false)]);
        doc.passes[1].pieces = Arc::new(vec![tile("green", 6, 3, "3070")]);
        doc.passes[2].pieces = Arc::new(vec![tile("hidden", 10, 10, "3070")]);
        doc.passes[2].visible = false;
        let before = doc.clone();
        let preset = capture_preset(&doc, "  Pair  ", None).unwrap();
        assert_eq!((preset.name.as_str(), preset.width, preset.height), ("Pair", 3, 1));
        assert_eq!(
            preset.passes.iter().map(|p| &p.color).collect::<Vec<_>>(),
            vec![&doc.passes[0].color, &doc.passes[1].color]
        );
        let first = &preset.passes[0].pieces[0];
        assert_eq!((first.x, first.y, first.mirror_x, first.seed), (0, 0, Some(true), 171));
        assert_eq!(preset.passes[1].pieces[0].x, 2);
        assert_eq!(doc, before);
    }

    #[test]
    fn saves_only_selected_pieces_and_rejects_empty() {
        let mut doc = create_document(false);
        doc.passes[0].pieces = Arc::new(vec![tile("one", 3, 5, "3070"), tile("two", 7, 8, "3070")]);
        let preset = capture_preset(&doc, "Selected", Some(&HashSet::from(["two"]))).unwrap();
        assert_eq!((preset.width, preset.height), (1, 1));
        assert_eq!(&*preset.passes[0].pieces[0].uid, "two");
        assert!(capture_preset(&doc, "Empty", Some(&HashSet::new())).unwrap_err().0.contains("visible pieces"));
    }

    #[test]
    fn rotates_curved_groups_and_their_mirrors() {
        let mut doc = create_document(false);
        doc.passes[0].pieces = Arc::new(vec![
            tile("ring", 3, 3, "27925").with_mirror(true, false),
            tile("quarter", 4, 3, "25269").with_mirror(true, false),
        ]);
        let preset = capture_preset(&doc, "Curve", None).unwrap();
        let target = create_document(false);
        for rotation in [0, 90, 180, 270] {
            let rows = preset_rows(&preset, 4, 4, rotation);
            let pieces: Vec<_> = rows.iter().map(|r| r.piece.clone()).collect();
            assert!(can_place(&target, &pieces, &none()), "{rotation}");
            assert_eq!(rows[0].piece.rotation, rotation);
            let upright = rotation % 180 == 0;
            assert_eq!(rows[0].piece.mirror_x, upright.then_some(true));
            assert_eq!(rows[0].piece.mirror_y, (!upright).then_some(true));
        }
    }

    #[test]
    fn keeps_geometry_on_other_boards_and_rejects_small_boards() {
        let mut doc = create_document(false);
        doc.passes[0].pieces = Arc::new(vec![tile("bar", 4, 4, "2431")]);
        let preset = capture_preset(&doc, "Bar", None).unwrap();
        let pieces = |p: &CompositionPreset| preset_rows(p, 0, 0, 0).into_iter().map(|r| r.piece).collect::<Vec<_>>();
        for size in [4, 8, 32] {
            let mut target = create_document(false);
            target.board = crate::document::Board { width: size, height: size };
            assert!(can_place(&target, &pieces(&preset), &none()));
        }
        let mut small = create_document(false);
        small.board = crate::document::Board { width: 3, height: 3 };
        assert!(!can_place(&small, &pieces(&preset), &none()));
        small.board = crate::document::Board { width: 4, height: 4 };
        small.passes[0].pieces = Arc::new(vec![tile("existing", 0, 0, "3070")]);
        assert!(!can_place(&small, &pieces(&preset), &none()));
    }

    #[test]
    fn opens_a_preset_as_its_own_document() {
        let mut doc = create_document(false);
        doc.passes[0].pieces = Arc::new(vec![tile("red", 4, 3, "3070").with_mirror(true, false)]);
        doc.passes[1].pieces = Arc::new(vec![tile("green", 6, 5, "2431")]);
        let preset = capture_preset(&doc, "Motif", None).unwrap();
        let opened = document_from_preset(&preset);
        assert_eq!(opened.name, "Motif");
        let m = PRESET_DOCUMENT_MARGIN;
        assert_eq!((opened.board.width, opened.board.height), (preset.width + 2 * m, preset.height + 2 * m));
        assert_eq!(opened.passes.len(), 2);
        assert_eq!(
            opened.passes.iter().map(|p| (&p.name, &p.color)).collect::<Vec<_>>(),
            preset.passes.iter().map(|p| (&p.name, &p.color)).collect::<Vec<_>>()
        );
        let red = &opened.passes[0].pieces[0];
        assert_eq!((red.x, red.y, red.mirror_x, red.seed), (m, m, Some(true), 171));
        assert_ne!(&*red.uid, "red", "pieces get fresh ids");
        assert!(!opened.options.allow_overlap);
        assert!(opened.swatches.iter().any(|s| s == &preset.passes[1].color));
        assert_eq!(parse_document(&serialize_document(&opened)).unwrap(), opened, "a valid project");
        assert_eq!(opened.paper, create_document(false).paper);

        // Overlapping pieces (from a board that allowed them) keep overlap on; a
        // preset as large as the biggest board gets no margin.
        let mut big = preset.clone();
        big.width = MAX_BOARD;
        big.passes[1].pieces[0].x = 0;
        big.passes[1].pieces[0].y = 0;
        let opened = document_from_preset(&big);
        assert_eq!(opened.board.width, MAX_BOARD);
        assert!(opened.options.allow_overlap);
        assert!(parse_document(&serialize_document(&opened)).is_ok());
    }

    #[test]
    fn validates_libraries() {
        let mut doc = create_document(false);
        doc.passes[0].pieces = Arc::new(vec![tile("one", 3, 4, "3070")]);
        let mut preset = capture_preset(&doc, "One", None).unwrap();
        let one = serde_json::to_string(&[&preset]).unwrap();
        assert_eq!(parse_presets(&one).unwrap(), vec![preset.clone()]);
        let two = serde_json::to_string(&[&preset, &preset]).unwrap();
        assert!(parse_presets(&two).unwrap_err().0.contains("Invalid"));
        let wide = serde_json::to_string(&[CompositionPreset { width: 129, ..preset.clone() }]).unwrap();
        assert!(parse_presets(&wide).unwrap_err().0.contains("dimensions"));
        preset.passes[0].pieces[0].piece_id = "missing".into();
        assert!(parse_presets(&serde_json::to_string(&[&preset]).unwrap()).unwrap_err().0.contains("unavailable"));
    }
}
