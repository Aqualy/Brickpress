//! Serializable editing operations for agents (MCP, headless or live).
//!
//! Each [`Op`] is applied as **one** undoable step through [`EditorCore`], so
//! a person can undo an agent's change in the GUI with a single Ctrl+Z, and
//! every placement obeys the same collision and catalog rules as the mouse.

use crate::catalog::{catalog, get_piece, normalize_rotation};
use crate::document::*;
use crate::editor::EditorCore;
use crate::geometry::{bounds, Rect};
use crate::persistence::document::{serialize_document, MAX_PIECES};
use crate::settings::{paper_preset, print_control_by_id, print_presets, PRINT_CONTROLS};
use crate::spatial::OccupancyIndex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};

/// A piece to place. `piece` is a catalog id ("3070") or a name ("Tile 1×1").
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PieceSpec {
    pub piece: String,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub rotation: i32,
    #[serde(default)]
    pub mirror_x: bool,
    #[serde(default)]
    pub mirror_y: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LegendEntry {
    pub piece: String,
    #[serde(default)]
    pub rotation: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Op {
    Info,
    GetDocument,
    BoardAscii,
    QueryPieces {
        #[serde(default)]
        rect: Option<[i32; 4]>,
        #[serde(default)]
        pass: Option<String>,
    },
    PlacePieces {
        pieces: Vec<PieceSpec>,
        #[serde(default)]
        pass: Option<String>,
        /// All or nothing.
        #[serde(default)]
        atomic: bool,
    },
    PlaceGrid {
        rows: Vec<String>,
        #[serde(default)]
        x: i32,
        #[serde(default)]
        y: i32,
        #[serde(default)]
        pass: Option<String>,
        #[serde(default)]
        legend: BTreeMap<String, LegendEntry>,
    },
    RemovePieces {
        #[serde(default)]
        uids: Vec<String>,
        #[serde(default)]
        rect: Option<[i32; 4]>,
    },
    MovePieces {
        uids: Vec<String>,
        dx: i32,
        dy: i32,
    },
    AddPass {
        color: String,
        #[serde(default)]
        name: Option<String>,
    },
    UpdatePass {
        pass: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        visible: Option<bool>,
        #[serde(default)]
        locked: Option<bool>,
        #[serde(default)]
        index: Option<usize>,
    },
    RemovePass {
        pass: String,
    },
    ResizeBoard {
        width: i32,
        height: i32,
    },
    SetPrint {
        #[serde(default)]
        preset: Option<String>,
        #[serde(default)]
        settings: BTreeMap<String, f64>,
    },
    SetPaper {
        #[serde(default)]
        preset: Option<String>,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        grain: Option<f64>,
        #[serde(default)]
        tooth: Option<f64>,
        #[serde(default)]
        fibers: Option<f64>,
    },
    SetOptions {
        #[serde(default)]
        grid: Option<GridStyle>,
        #[serde(default)]
        major_grid: Option<i32>,
        #[serde(default)]
        symmetry: Option<Symmetry>,
        #[serde(default)]
        allow_overlap: Option<bool>,
        #[serde(default)]
        physical: Option<bool>,
    },
    Rename {
        name: String,
    },
    Clear,
    Reseed,
    Undo,
    Redo,
}

impl Op {
    /// Whether the op only reads the document.
    pub fn is_read_only(&self) -> bool {
        matches!(self, Op::Info | Op::GetDocument | Op::BoardAscii | Op::QueryPieces { .. })
    }
}

pub type OpResult = Result<Value, String>;

/// Finds a catalog piece by id or (case-insensitive) name.
pub fn resolve_piece(name: &str) -> Option<&'static crate::catalog::Piece> {
    get_piece(name).or_else(|| {
        let lower = name.trim().to_lowercase().replace('x', "×");
        catalog()
            .pieces
            .iter()
            .find(|p| p.name.to_lowercase() == lower || p.name.to_lowercase() == name.trim().to_lowercase())
    })
}

/// Finds a pass by id, name (case-insensitive) or zero-based index.
pub fn resolve_pass(doc: &PressDocument, reference: &str) -> Option<usize> {
    doc.pass_index(reference)
        .or_else(|| doc.passes.iter().position(|p| p.name.eq_ignore_ascii_case(reference.trim())))
        .or_else(|| reference.trim().parse::<usize>().ok().filter(|i| *i < doc.passes.len()))
}

fn pass_or_active(core: &EditorCore, reference: Option<&str>) -> Result<usize, String> {
    match reference {
        Some(r) => resolve_pass(core.doc(), r)
            .ok_or_else(|| format!("No ink pass called “{r}”. Use document_info to list passes.")),
        None => Ok(core.doc().pass_index(&core.active_pass().id).unwrap_or(0)),
    }
}

fn in_rect(p: &PlacedPiece, rect: [i32; 4]) -> bool {
    bounds(p).intersects(&Rect { x: rect[0], y: rect[1], width: rect[2], height: rect[3] })
}

pub fn piece_json(pass: &InkPass, p: &PlacedPiece) -> Value {
    let piece = get_piece(&p.piece_id);
    let b = bounds(p);
    json!({
        "uid": &*p.uid,
        "piece": &*p.piece_id,
        "name": piece.map(|x| x.name.as_str()),
        "x": p.x, "y": p.y, "width": b.width, "height": b.height,
        "rotation": p.rotation,
        "mirrorX": p.mx(), "mirrorY": p.my(),
        "pass": &*pass.id, "passName": pass.name,
    })
}

pub fn info(core: &EditorCore) -> Value {
    let doc = core.doc();
    json!({
        "name": doc.name,
        "board": { "width": doc.board.width, "height": doc.board.height },
        "pieces": doc.piece_count(),
        "activePass": &*core.active_pass().id,
        "passes": doc.passes.iter().enumerate().map(|(i, p)| json!({
            "index": i, "id": &*p.id, "name": p.name, "color": p.color,
            "visible": p.visible, "locked": p.locked, "pieces": p.pieces.len(),
        })).collect::<Vec<_>>(),
        "options": doc.options,
        "paper": doc.paper,
        "printSettings": doc.print_settings,
        "canUndo": core.can_undo(),
        "canRedo": core.can_redo(),
        "dirty": core.dirty,
    })
}

/// Occupancy as text: one character per stud, a letter per ink pass
/// (A = first pass), `.` for empty. Quarter rounds show their rotation digit.
pub fn board_ascii(doc: &PressDocument) -> String {
    let (w, h) = (doc.board.width as usize, doc.board.height as usize);
    let mut grid = vec![vec!['.'; w]; h];
    for (index, pass) in doc.passes.iter().enumerate() {
        let letter = (b'A' + (index % 26) as u8) as char;
        for p in pass.pieces.iter() {
            for (x, y) in crate::geometry::occupied_cells(p) {
                if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                    grid[y as usize][x as usize] = if &*p.piece_id == "25269" {
                        char::from_digit((p.rotation / 90) as u32, 10).unwrap_or(letter)
                    } else {
                        letter
                    };
                }
            }
        }
    }
    let mut out = String::new();
    for row in grid {
        out.extend(row);
        out.push('\n');
    }
    out.push_str("\nLegend: . empty, 0-3 quarter round (rotation/90), ");
    let passes: Vec<String> = doc
        .passes
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{} = {} ({})", (b'A' + (i % 26) as u8) as char, p.name, p.color))
        .collect();
    out.push_str(&passes.join(", "));
    out
}

/// Applies one operation as one undoable step.
pub fn apply(core: &mut EditorCore, op: Op) -> OpResult {
    match op {
        Op::Info => Ok(info(core)),
        Op::GetDocument => Ok(serde_json::from_str(&serialize_document(core.doc())).expect("valid json")),
        Op::BoardAscii => Ok(Value::String(board_ascii(core.doc()))),
        Op::QueryPieces { rect, pass } => {
            let pass =
                pass.map(|p| resolve_pass(core.doc(), &p).ok_or(format!("No ink pass called “{p}”."))).transpose()?;
            let doc = core.doc();
            let pieces: Vec<Value> = doc
                .passes
                .iter()
                .enumerate()
                .filter(|(i, _)| pass.is_none_or(|p| p == *i))
                .flat_map(|(_, pass)| pass.pieces.iter().map(move |p| (pass, p)))
                .filter(|(_, p)| rect.is_none_or(|r| in_rect(p, r)))
                .take(2000)
                .map(|(pass, p)| piece_json(pass, p))
                .collect();
            Ok(json!({ "count": pieces.len(), "pieces": pieces }))
        }
        Op::PlacePieces { pieces, pass, atomic } => place(core, pieces, pass.as_deref(), atomic),
        Op::PlaceGrid { rows, x, y, pass, legend } => {
            let mut specs = Vec::new();
            for (row_index, row) in rows.iter().enumerate() {
                for (col, c) in row.chars().enumerate() {
                    let (px, py) = (x + col as i32, y + row_index as i32);
                    let entry = match c {
                        '.' | ' ' | '_' => continue,
                        '#' => LegendEntry { piece: "3070".into(), rotation: 0 },
                        '0'..='3' => LegendEntry { piece: "25269".into(), rotation: c.to_digit(10).unwrap() as i32 * 90 },
                        other => match legend.get(&other.to_string()) {
                            Some(e) => e.clone(),
                            None => return Err(format!("Unknown grid character “{other}” at row {row_index}, column {col}. Add it to `legend`.")),
                        },
                    };
                    specs.push(PieceSpec {
                        piece: entry.piece,
                        x: px,
                        y: py,
                        rotation: entry.rotation,
                        mirror_x: false,
                        mirror_y: false,
                    });
                }
            }
            place(core, specs, pass.as_deref(), false)
        }
        Op::RemovePieces { uids, rect } => {
            let mut ids: HashSet<Id> = uids.iter().map(|u| Id::from(u.as_str())).collect();
            if let Some(r) = rect {
                ids.extend(
                    core.doc()
                        .all_pieces()
                        .filter(|(pass, p)| !pass.locked && in_rect(p, r))
                        .map(|(_, p)| p.uid.clone()),
                );
            }
            let locked: HashSet<Id> =
                core.doc().all_pieces().filter(|(pass, _)| pass.locked).map(|(_, p)| p.uid.clone()).collect();
            ids.retain(|id| !locked.contains(id));
            let before = core.doc().piece_count();
            core.remove_pieces(&ids);
            Ok(json!({ "removed": before - core.doc().piece_count() }))
        }
        Op::MovePieces { uids, dx, dy } => {
            let wanted: HashSet<&str> = uids.iter().map(String::as_str).collect();
            let moved: Vec<PlacedPiece> = core
                .doc()
                .all_pieces()
                .filter(|(pass, p)| !pass.locked && wanted.contains(&*p.uid))
                .map(|(_, p)| PlacedPiece { x: p.x + dx, y: p.y + dy, ..p.clone() })
                .collect();
            if moved.is_empty() {
                return Err("None of those pieces exist on an unlocked pass.".into());
            }
            OccupancyIndex::build(core.doc(), &wanted)
                .check(&moved)
                .map_err(|(i, e)| format!("{} would {e}.", moved[i].uid))?;
            core.replace_pieces(moved.clone());
            Ok(json!({ "moved": moved.len() }))
        }
        Op::AddPass { color, name } => {
            if !valid_color(&color) {
                return Err("Use a six-digit hex colour such as #315eae.".into());
            }
            let id = core.add_pass(&color.to_lowercase()).ok_or("Use up to 100 ink passes.")?;
            if let Some(name) = name {
                core.end_live();
                rename_pass_without_history(core, &id, &name);
            }
            Ok(json!({ "pass": &*id }))
        }
        Op::UpdatePass { pass, name, color, visible, locked, index } => {
            let i = resolve_pass(core.doc(), &pass).ok_or(format!("No ink pass called “{pass}”."))?;
            if let Some(c) = &color {
                if !valid_color(c) {
                    return Err("Use a six-digit hex colour such as #315eae.".into());
                }
            }
            let id = core.doc().passes[i].id.clone();
            core.commit(|doc| {
                let p = doc.pass_mut(&id).expect("exists");
                if let Some(name) = name {
                    p.name = name.chars().take(160).collect();
                }
                if let Some(c) = color {
                    p.color = c.to_lowercase();
                }
                if let Some(v) = visible {
                    p.visible = v;
                }
                if let Some(l) = locked {
                    p.locked = l;
                }
                if let Some(to) = index {
                    let from = doc.pass_index(&id).expect("exists");
                    let pass = doc.passes.remove(from);
                    doc.passes.insert(to.min(doc.passes.len()), pass);
                }
            });
            Ok(json!({ "pass": &*id }))
        }
        Op::RemovePass { pass } => {
            let i = resolve_pass(core.doc(), &pass).ok_or(format!("No ink pass called “{pass}”."))?;
            let id = core.doc().passes[i].id.clone();
            if !core.remove_pass(&id) {
                return Err(core.take_notice().unwrap_or_else(|| "The pass could not be removed.".into()));
            }
            Ok(json!({ "removed": &*id }))
        }
        Op::ResizeBoard { width, height } => {
            if core.resize(width, height) || core.doc().board == (Board { width, height }) {
                Ok(json!({ "board": core.doc().board }))
            } else {
                Err(core.take_notice().unwrap_or_else(|| "The board could not be resized.".into()))
            }
        }
        Op::SetPrint { preset, settings } => {
            let mut next = core.doc().print_settings;
            if let Some(name) = preset {
                next = print_presets()
                    .into_iter()
                    .find(|(n, _)| n.eq_ignore_ascii_case(&name))
                    .map(|(_, s)| s)
                    .or_else(|| {
                        core.preferences
                            .print_presets
                            .iter()
                            .find(|p| p.name.eq_ignore_ascii_case(&name))
                            .map(|p| p.settings)
                    })
                    .ok_or_else(|| {
                        let names: Vec<&str> = print_presets().iter().map(|(n, _)| *n).collect();
                        format!("Unknown print preset. Built-in presets: {}.", names.join(", "))
                    })?;
            }
            for (key, value) in settings {
                let control = print_control_by_id(&key).ok_or_else(|| {
                    let ids: Vec<&str> = PRINT_CONTROLS.iter().map(|c| c.id).collect();
                    format!("Unknown print setting “{key}”. Use one of: {}.", ids.join(", "))
                })?;
                if !value.is_finite() || value < control.min || value > control.max {
                    return Err(format!("{} must be between {} and {}.", control.label, control.min, control.max));
                }
                *next.get_mut(control.key) = value;
            }
            core.commit(|doc| doc.print_settings = next);
            Ok(json!({ "printSettings": core.doc().print_settings }))
        }
        Op::SetPaper { preset, color, grain, tooth, fibers } => {
            let mut paper = core.doc().paper.clone();
            if let Some(name) = preset {
                paper = paper_preset(&name, paper.seed).ok_or(
                    "Unknown paper. Use Bright White, Natural White, Warm Ivory, Recycled, Newsprint or Heavy Cotton.",
                )?;
            }
            if let Some(c) = color {
                if !valid_color(&c) {
                    return Err("Use a six-digit hex colour such as #f6f4ec.".into());
                }
                paper.color = c.to_lowercase();
            }
            for (value, target) in [(grain, &mut paper.grain), (tooth, &mut paper.tooth), (fibers, &mut paper.fibers)] {
                if let Some(v) = value {
                    if !(0.0..=1.2).contains(&v) {
                        return Err("Paper grain, tooth and fibres range from 0 to 1.2.".into());
                    }
                    *target = v;
                }
            }
            core.commit(|doc| doc.paper = paper);
            Ok(json!({ "paper": core.doc().paper }))
        }
        Op::SetOptions { grid, major_grid, symmetry, allow_overlap, physical } => {
            if let Some(m) = major_grid {
                if m != 4 && m != 8 {
                    return Err("majorGrid is 4 or 8.".into());
                }
            }
            if let Some(p) = physical {
                if !core.set_physical(p) && core.doc().options.physical != p {
                    return Err(core.take_notice().unwrap_or_default());
                }
            }
            if let Some(a) = allow_overlap {
                if !core.set_allow_overlap(a) && core.doc().options.allow_overlap != a {
                    return Err(core.take_notice().unwrap_or_default());
                }
            }
            core.commit(|doc| {
                if let Some(g) = grid {
                    doc.options.grid = g;
                }
                if let Some(m) = major_grid {
                    doc.options.major_grid = m;
                }
                if let Some(s) = symmetry {
                    doc.options.symmetry = s;
                }
            });
            Ok(json!({ "options": core.doc().options }))
        }
        Op::Rename { name } => {
            core.rename(&name);
            Ok(json!({ "name": core.doc().name }))
        }
        Op::Clear => {
            let removed = core.doc().piece_count();
            core.clear();
            Ok(json!({ "removed": removed }))
        }
        Op::Reseed => {
            core.reseed();
            Ok(json!({ "reseeded": true }))
        }
        Op::Undo => Ok(json!({ "undone": core.undo() })),
        Op::Redo => Ok(json!({ "redone": core.redo() })),
    }
}

fn rename_pass_without_history(core: &mut EditorCore, id: &str, name: &str) {
    // Folds the name into the add-pass step instead of a second undo entry.
    let name: String = name.trim().chars().take(160).collect();
    if !name.is_empty() {
        core.edit_live(|doc| {
            if let Some(p) = doc.pass_mut(id) {
                p.name = name;
            }
        });
        core.end_live();
        core.squash_last_step();
    }
}

fn place(core: &mut EditorCore, specs: Vec<PieceSpec>, pass: Option<&str>, atomic: bool) -> OpResult {
    let pass_index = pass_or_active(core, pass)?;
    let doc = core.doc();
    let target = &doc.passes[pass_index];
    if target.locked || !target.visible {
        return Err(format!("“{}” is hidden or locked. Choose a visible, unlocked pass.", target.name));
    }
    let mut index = OccupancyIndex::build(doc, &HashSet::new());
    let mut accepted = Vec::new();
    let mut results = Vec::with_capacity(specs.len());
    let mut count = doc.piece_count();
    for (i, spec) in specs.iter().enumerate() {
        let reject = |reason: String| json!({ "index": i, "placed": false, "reason": reason });
        let Some(piece) = resolve_piece(&spec.piece) else {
            results.push(reject(format!("Unknown piece “{}”. Use catalog_list.", spec.piece)));
            continue;
        };
        let rotation = normalize_rotation(spec.rotation);
        if !piece.allows(rotation) {
            results.push(reject(format!(
                "{} cannot be rotated {}°; allowed: {:?}.",
                piece.name, rotation, piece.allowed_rotations
            )));
            continue;
        }
        if doc.options.physical && !piece.is_physical() {
            results.push(reject(format!("{} is not standard height; physical mode is on.", piece.name)));
            continue;
        }
        if count >= MAX_PIECES {
            results.push(reject("The project limit of 10,000 pieces is reached.".into()));
            continue;
        }
        let candidate = PlacedPiece::new(new_id(), piece.id.as_str(), spec.x, spec.y, rotation, new_seed())
            .with_mirror(spec.mirror_x, spec.mirror_y);
        match index.check(std::slice::from_ref(&candidate)) {
            Ok(()) => {
                index.add(std::slice::from_ref(&candidate));
                results.push(json!({ "index": i, "placed": true, "uid": &*candidate.uid }));
                accepted.push(candidate);
                count += 1;
            }
            Err((_, e)) => results.push(reject(format!("{} at ({}, {}) {e}.", piece.name, spec.x, spec.y))),
        }
    }
    let rejected = specs.len() - accepted.len();
    if atomic && rejected > 0 {
        return Ok(
            json!({ "placed": 0, "rejected": specs.len(), "results": results, "note": "atomic: nothing was placed" }),
        );
    }
    let placed = accepted.len();
    if placed > 0 {
        let pass_id = core.doc().passes[pass_index].id.clone();
        core.commit(|doc| doc.pass_mut(&pass_id).expect("exists").pieces_mut().extend(accepted));
    }
    Ok(json!({ "placed": placed, "rejected": rejected, "results": results }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn core() -> EditorCore {
        let mut c = EditorCore::default();
        c.new_document();
        c
    }

    fn run(c: &mut EditorCore, value: Value) -> OpResult {
        apply(c, serde_json::from_value(value).map_err(|e| e.to_string())?)
    }

    #[test]
    fn places_with_per_item_reasons_as_one_undo_step() {
        let mut c = core();
        let r = run(
            &mut c,
            json!({"op":"placePieces","pieces":[
                {"piece":"3070","x":0,"y":0},
                {"piece":"3070","x":0,"y":0},
                {"piece":"Tile 1×2","x":4,"y":4,"rotation":90},
                {"piece":"nope","x":1,"y":1},
                {"piece":"3070","x":99,"y":0}
            ]}),
        )
        .unwrap();
        assert_eq!(r["placed"], 2);
        assert_eq!(r["rejected"], 3);
        assert!(r["results"][1]["reason"].as_str().unwrap().contains("overlaps"));
        assert!(r["results"][4]["reason"].as_str().unwrap().contains("leaves the board"));
        assert!(c.undo());
        assert_eq!(c.doc().piece_count(), 0);
        assert!(!c.can_undo());
    }

    #[test]
    fn atomic_placement_is_all_or_nothing() {
        let mut c = core();
        let r = run(&mut c, json!({"op":"placePieces","atomic":true,"pieces":[{"piece":"3070","x":0,"y":0},{"piece":"3070","x":0,"y":0}]})).unwrap();
        assert_eq!(r["placed"], 0);
        assert_eq!(c.doc().piece_count(), 0);
    }

    #[test]
    fn grid_notation_draws_letterforms() {
        let mut c = core();
        let r = run(&mut c, json!({"op":"placeGrid","x":2,"y":2,"rows":["1###0","#...#","2###3"]})).unwrap();
        assert_eq!(r["placed"], 12);
        let ascii = board_ascii(c.doc());
        assert!(ascii.lines().nth(2).unwrap().starts_with("..1AAA0"), "{ascii}");
        assert!(run(&mut c, json!({"op":"placeGrid","rows":["x"]})).unwrap_err().contains("legend"));
    }

    #[test]
    fn passes_settings_and_queries() {
        let mut c = core();
        let added = run(&mut c, json!({"op":"addPass","color":"#123456","name":"Blue"})).unwrap();
        let id = added["pass"].as_str().unwrap().to_string();
        assert_eq!(c.doc().pass(&id).unwrap().name, "Blue");
        c.undo();
        assert!(c.doc().pass(&id).is_none(), "naming does not add a second undo step");
        c.redo();
        run(&mut c, json!({"op":"placePieces","pass":"Blue","pieces":[{"piece":"3070","x":3,"y":3}]})).unwrap();
        let q = run(&mut c, json!({"op":"queryPieces","rect":[0,0,5,5]})).unwrap();
        assert_eq!(q["count"], 1);
        assert_eq!(q["pieces"][0]["passName"], "Blue");
        run(&mut c, json!({"op":"setPrint","preset":"Worn","settings":{"pressure":0.9}})).unwrap();
        assert_eq!(c.doc().print_settings.pressure, 0.9);
        assert!(run(&mut c, json!({"op":"setPrint","settings":{"pressure":3}})).is_err());
        run(&mut c, json!({"op":"setPaper","preset":"Recycled"})).unwrap();
        assert_eq!(c.doc().paper.preset, "Recycled");
        run(&mut c, json!({"op":"setOptions","symmetry":"x","grid":"lines"})).unwrap();
        assert_eq!(c.doc().options.symmetry, Symmetry::X);
        let uid = q["pieces"][0]["uid"].as_str().unwrap();
        run(&mut c, json!({"op":"movePieces","uids":[uid],"dx":1,"dy":0})).unwrap();
        assert_eq!(c.doc().find_piece(uid).unwrap().1.x, 4);
        run(&mut c, json!({"op":"removePieces","rect":[0,0,16,16]})).unwrap();
        assert_eq!(c.doc().piece_count(), 0);
    }

    #[test]
    fn ops_round_trip_through_json() {
        let op = Op::PlaceGrid { rows: vec!["#".into()], x: 0, y: 0, pass: None, legend: BTreeMap::new() };
        let text = serde_json::to_string(&op).unwrap();
        assert!(text.contains("\"op\":\"placeGrid\""));
        let _: Op = serde_json::from_str(&text).unwrap();
    }
}
