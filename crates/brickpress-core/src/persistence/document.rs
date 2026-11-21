//! Creating, serializing and validating projects. Validation messages match 1.x.

use crate::catalog::{catalog, get_piece};
use crate::document::*;
use crate::settings::{paper_preset, PRINT_CONTROLS, PRINT_DEFAULTS, STARTER_SWATCHES};
use crate::spatial::OccupancyIndex;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

pub const MAX_PROJECT_BYTES: usize = 20_000_000;
pub const MAX_PASSES: usize = 100;
pub const MAX_PIECES: usize = 10_000;
pub const MAX_BOARD: i32 = 128;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct DocumentError(pub String);

fn err<T>(message: impl Into<String>) -> Result<T, DocumentError> {
    Err(DocumentError(message.into()))
}

pub fn make_pass(color: &str, name: &str) -> InkPass {
    InkPass {
        id: new_id(),
        name: name.into(),
        color: color.into(),
        visible: true,
        locked: false,
        registration_seed: new_seed(),
        pieces: Arc::new(Vec::new()),
    }
}

/// A new document, or the starter composition ("A study in bloom") when `sample`.
pub fn create_document(sample: bool) -> PressDocument {
    let size = &catalog().rendering.recommended_default_board;
    let mut doc = PressDocument {
        version: 1,
        name: if sample { "A study in bloom" } else { "Untitled impression" }.into(),
        board: Board { width: size.width_studs, height: size.height_studs },
        paper: paper_preset("Natural White", 52718).expect("built-in paper"),
        print_settings: PRINT_DEFAULTS,
        passes: vec![
            make_pass(STARTER_SWATCHES[1], "01 · Vermilion"),
            make_pass(STARTER_SWATCHES[3], "02 · Botanical"),
            make_pass(STARTER_SWATCHES[4], "03 · Ochre"),
        ],
        swatches: STARTER_SWATCHES.iter().map(|s| s.to_string()).collect(),
        options: DocOptions {
            grid: GridStyle::Points,
            major_grid: 4,
            allow_overlap: false,
            physical: false,
            symmetry: Symmetry::Off,
        },
    };
    if sample {
        // Stable IDs and seeds make the starter a repeatable first impression.
        for (i, pass) in doc.passes.iter_mut().enumerate() {
            pass.id = format!("starter-pass-{i}").into();
            pass.registration_seed = 3719 + i as i64 * 8171;
        }
        let add = |doc: &mut PressDocument, pass: usize, id: &str, x: i32, y: i32, rotation: i32| {
            let index = doc.passes[pass].pieces.len() as i64;
            let p = PlacedPiece::new(
                format!("starter-piece-{pass}-{index}"),
                id,
                x,
                y,
                rotation,
                (pass as i64 + 1) * 100_003 + index * 8191 + x as i64 * 127 + y as i64 * 17 + rotation as i64,
            );
            if OccupancyIndex::build(doc, &HashSet::new()).can_place(std::slice::from_ref(&p)) {
                doc.passes[pass].pieces_mut().push(p);
            }
        };
        // An original modular garden, assembled entirely from catalog surfaces.
        for (x, y) in [(4, 3), (8, 2), (11, 5)] {
            add(&mut doc, 0, "25269", x, y, 0);
            add(&mut doc, 0, "25269", x + 1, y, 90);
            add(&mut doc, 0, "25269", x, y + 1, 270);
            add(&mut doc, 0, "25269", x + 1, y + 1, 180);
            add(&mut doc, 2, "98138", x, y + 2, 0);
            add(&mut doc, 2, "98138", x + 1, y + 2, 0);
            for sy in y + 3..12 {
                add(&mut doc, 1, "3069", x, sy, 0);
            }
            add(&mut doc, 1, "25269", x - 1, y + 4, 180);
            add(&mut doc, 1, "25269", x + 2, y + 5, 270);
        }
        for x in 3..=12 {
            add(&mut doc, 1, "24246", x, 12, if x % 2 == 1 { 0 } else { 180 });
        }
        add(&mut doc, 2, "1748", 3, 6, 180);
        add(&mut doc, 2, "1748", 11, 3, 0);
        add(&mut doc, 2, "98138", 6, 2, 0);
        add(&mut doc, 2, "98138", 3, 9, 0);
    }
    doc
}

pub fn serialize_document(doc: &PressDocument) -> String {
    serde_json::to_string_pretty(doc).expect("documents always serialize")
}

// --- lenient readers, so malformed files get 1.x's specific messages --------

fn int(v: Option<&Value>) -> Option<i64> {
    let v = v?;
    if let Some(i) = v.as_i64() {
        return Some(i);
    }
    let f = v.as_f64()?;
    (f.fract() == 0.0 && f.abs() < 9.0e15).then_some(f as i64)
}
fn num(v: Option<&Value>) -> Option<f64> {
    v?.as_f64().filter(|f| f.is_finite())
}
fn string(v: Option<&Value>) -> Option<&str> {
    v?.as_str()
}
fn boolean(v: Option<&Value>) -> Option<bool> {
    v?.as_bool()
}
fn optional_bool(v: Option<&Value>) -> Result<Option<bool>, ()> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        _ => Err(()),
    }
}

pub fn parse_document(text: &str) -> Result<PressDocument, DocumentError> {
    if text.len() > MAX_PROJECT_BYTES {
        return err("This project is too large (maximum 20 MB).");
    }
    let value: Value = serde_json::from_str(text).map_err(|_| DocumentError("This file is not valid JSON.".into()))?;
    parse_document_value(&value)
}

pub fn parse_document_value(doc: &Value) -> Result<PressDocument, DocumentError> {
    let unsupported = || DocumentError("This is not a supported Brickpress project.".into());
    if int(doc.get("version")) != Some(1) || !doc.get("board").is_some_and(Value::is_object) {
        return Err(unsupported());
    }
    let passes = doc.get("passes").and_then(Value::as_array).ok_or_else(unsupported)?;
    if passes.is_empty() || passes.len() > MAX_PASSES {
        return Err(unsupported());
    }
    let board = &doc["board"];
    let dims: Vec<Option<i64>> = vec![int(board.get("width")), int(board.get("height"))];
    if !dims.iter().all(|d| d.is_some_and(|n| (1..=MAX_BOARD as i64).contains(&n))) {
        return err("Board dimensions must be whole numbers between 1 and 128.");
    }
    let board = Board { width: dims[0].unwrap() as i32, height: dims[1].unwrap() as i32 };
    let name = string(doc.get("name"));
    let paper = doc.get("paper").filter(|v| v.is_object());
    let print = doc.get("printSettings").filter(|v| v.is_object());
    let options = doc.get("options").filter(|v| v.is_object());
    let paper_color = paper.and_then(|p| string(p.get("color")));
    let (Some(name), Some(paper), Some(print), Some(options), Some(paper_color)) =
        (name, paper, print, options, paper_color)
    else {
        return err("Project settings are incomplete.");
    };
    if name.chars().count() > 160 || !valid_color(paper_color) {
        return err("Project settings are incomplete.");
    }
    let mut print_settings = PRINT_DEFAULTS;
    for c in &PRINT_CONTROLS {
        match num(print.get(c.id)) {
            Some(v) if v >= c.min && v <= c.max => *print_settings.get_mut(c.key) = v,
            _ => return err(format!("Invalid {}.", c.label.to_lowercase())),
        }
    }
    let mut paper_values = [0.0; 4];
    for (i, key) in ["grain", "tooth", "fibers", "brightness"].iter().enumerate() {
        match num(paper.get(*key)) {
            Some(v) if (0.0..=1.2).contains(&v) => paper_values[i] = v,
            _ => return err("Invalid paper settings."),
        }
    }
    let invalid_options = || DocumentError("Invalid document options.".into());
    let paper_seed = int(paper.get("seed")).ok_or_else(invalid_options)?;
    let paper_preset = string(paper.get("preset")).ok_or_else(invalid_options)?;
    let grid: GridStyle =
        serde_json::from_value(options.get("grid").cloned().unwrap_or(Value::Null)).map_err(|_| invalid_options())?;
    let symmetry: Symmetry = serde_json::from_value(options.get("symmetry").cloned().unwrap_or(Value::Null))
        .map_err(|_| invalid_options())?;
    let major_grid = int(options.get("majorGrid")).filter(|m| *m == 4 || *m == 8).ok_or_else(invalid_options)?;
    let allow_overlap = boolean(options.get("allowOverlap")).ok_or_else(invalid_options)?;
    let physical = boolean(options.get("physical")).ok_or_else(invalid_options)?;

    let mut ids: HashSet<String> = HashSet::new();
    let mut count = 0usize;
    let mut out_passes = Vec::with_capacity(passes.len());
    for pass in passes {
        let invalid = || DocumentError("Invalid ink pass.".into());
        let id = string(pass.get("id")).ok_or_else(invalid)?;
        let color = string(pass.get("color")).filter(|c| valid_color(c)).ok_or_else(invalid)?;
        let pass_name = string(pass.get("name")).ok_or_else(invalid)?;
        let visible = boolean(pass.get("visible")).ok_or_else(invalid)?;
        let locked = boolean(pass.get("locked")).ok_or_else(invalid)?;
        let registration_seed = int(pass.get("registrationSeed")).ok_or_else(invalid)?;
        let pieces = pass.get("pieces").and_then(Value::as_array).ok_or_else(invalid)?;
        if ids.contains(id) {
            return Err(invalid());
        }
        ids.insert(id.to_string());
        let mut out_pieces = Vec::with_capacity(pieces.len());
        for p in pieces {
            let piece_id = string(p.get("pieceId")).unwrap_or("");
            let Some(piece) = get_piece(piece_id) else {
                return err(format!("Catalog piece {piece_id} is unavailable."));
            };
            let bad = || DocumentError(format!("Invalid placement for {}.", piece.name));
            let uid = string(p.get("uid")).ok_or_else(bad)?;
            let x = int(p.get("x")).and_then(|v| i32::try_from(v).ok()).ok_or_else(bad)?;
            let y = int(p.get("y")).and_then(|v| i32::try_from(v).ok()).ok_or_else(bad)?;
            let seed = int(p.get("seed")).ok_or_else(bad)?;
            let rotation = int(p.get("rotation")).and_then(|v| i32::try_from(v).ok()).ok_or_else(bad)?;
            let mirror_x = optional_bool(p.get("mirrorX")).map_err(|_| bad())?;
            let mirror_y = optional_bool(p.get("mirrorY")).map_err(|_| bad())?;
            if ids.contains(uid) || !piece.allows(rotation) {
                return Err(bad());
            }
            if physical && !piece.is_physical() {
                return err(format!("{} is incompatible with physical print mode.", piece.name));
            }
            ids.insert(uid.to_string());
            count += 1;
            out_pieces.push(PlacedPiece {
                uid: uid.into(),
                piece_id: piece.id.as_str().into(),
                x,
                y,
                rotation,
                seed,
                mirror_x,
                mirror_y,
            });
        }
        out_passes.push(InkPass {
            id: id.into(),
            name: pass_name.into(),
            color: color.into(),
            visible,
            locked,
            registration_seed,
            pieces: Arc::new(out_pieces),
        });
    }
    if count > MAX_PIECES {
        return err("Maximum 10,000 pieces per project.");
    }
    let swatches = doc.get("swatches").and_then(Value::as_array);
    let swatches: Option<Vec<String>> = swatches
        .filter(|s| s.len() <= 200)
        .and_then(|s| s.iter().map(|c| c.as_str().filter(|c| valid_color(c)).map(String::from)).collect());
    let result = PressDocument {
        version: 1,
        name: name.into(),
        board,
        paper: PaperSettings {
            preset: paper_preset.into(),
            color: paper_color.into(),
            grain: paper_values[0],
            tooth: paper_values[1],
            fibers: paper_values[2],
            brightness: paper_values[3],
            seed: paper_seed,
        },
        print_settings,
        passes: out_passes,
        swatches: Vec::new(),
        options: DocOptions { grid, major_grid: major_grid as i32, allow_overlap, physical, symmetry },
    };
    let all: Vec<PlacedPiece> = result.all_pieces().map(|(_, p)| p.clone()).collect();
    if !OccupancyIndex::empty(&result).can_place(&all) {
        return err("The project has pieces outside the board or overlaps while overlap is disabled.");
    }
    let Some(swatches) = swatches else { return err("Invalid swatches.") };
    Ok(PressDocument { swatches, ..result })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(id: &str, x: i32, y: i32) -> PlacedPiece {
        PlacedPiece::new("test", id, x, y, 0, 171)
    }

    #[test]
    fn round_trips_every_piece_pass_seed_and_setting() {
        let doc = create_document(true);
        assert_eq!(parse_document(&serialize_document(&doc)).unwrap(), doc);
        assert!(doc.piece_count() > 30);
    }

    #[test]
    fn round_trips_fitted_curves_with_overlap_disabled() {
        let mut doc = create_document(false);
        let mut ring = placed("27925", 4, 4);
        ring.uid = "ring".into();
        let mut quarter = placed("25269", 4, 4);
        quarter.uid = "quarter".into();
        doc.passes[0].pieces = Arc::new(vec![ring, quarter]);
        assert_eq!(parse_document(&serialize_document(&doc)).unwrap(), doc);
    }

    #[test]
    fn starter_document_is_identical_every_time() {
        assert_eq!(create_document(true), create_document(true));
    }

    #[test]
    fn rejects_unsupported_corrupt_out_of_bounds_and_missing_pieces() {
        let mut doc = create_document(false);
        assert!(parse_document("{}").is_err());
        doc.passes[0].pieces_mut().push(placed("missing", 0, 0));
        assert!(parse_document(&serialize_document(&doc)).unwrap_err().0.contains("unavailable"));
        doc.passes[0].pieces = Arc::new(vec![placed("3070", -1, 0)]);
        assert!(parse_document(&serialize_document(&doc)).unwrap_err().0.contains("outside"));
    }

    #[test]
    fn rejects_duplicate_uids_and_non_finite_settings() {
        let mut doc = create_document(false);
        doc.passes[0].pieces = Arc::new(vec![placed("3070", 0, 0), placed("3070", 2, 2)]);
        assert!(parse_document(&serialize_document(&doc)).unwrap_err().0.contains("placement"));
        doc.passes[0].pieces = Arc::new(vec![]);
        let mut value = serde_json::to_value(&doc).unwrap();
        value["printSettings"]["pressure"] = Value::Null;
        assert!(parse_document_value(&value).unwrap_err().0.contains("pressure"));
    }

    #[test]
    fn writes_integral_numbers_like_javascript() {
        let text = serialize_document(&create_document(false));
        assert!(text.contains("\"textureScale\": 1,"), "{text}");
        assert!(text.contains("\"brightness\": 1,"));
        assert!(!text.contains("mirrorX"));
    }

    #[test]
    fn reads_a_1x_project_written_by_javascript() {
        // Shape and spelling produced by Brickpress 1.1 (`JSON.stringify(doc, null, 2)`).
        let text = r##"{"version":1,"name":"Legacy","board":{"width":8,"height":4},
          "paper":{"preset":"Recycled","color":"#d9d2bf","grain":0.5,"tooth":0.55,"fibers":0.7,"brightness":1,"seed":7},
          "printSettings":{"inkAmount":1,"pressure":0.8,"inkCoverage":1,"rollerConsistency":1,"pieceWear":0,
            "edgeWear":0,"paperTooth":0,"textureScale":1,"misalignment":0,"registrationError":0,"printRoughness":0},
          "passes":[{"id":"a","name":"Ink","color":"#252A26","visible":true,"locked":false,"registrationSeed":12,
            "pieces":[{"uid":"p1","pieceId":"3069","x":1,"y":1,"rotation":90,"seed":5,"mirrorX":true}]}],
          "swatches":["#252a26"],"options":{"grid":"lines","majorGrid":8,"allowOverlap":false,"physical":false,"symmetry":"x"}}"##;
        let doc = parse_document(text).unwrap();
        assert_eq!(doc.passes[0].pieces[0].mirror_x, Some(true));
        assert_eq!(doc.options.symmetry, Symmetry::X);
        assert_eq!(parse_document(&serialize_document(&doc)).unwrap(), doc);
    }
}
