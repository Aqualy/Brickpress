//! Editing state and every document mutation.
//!
//! Each change goes through [`EditorCore::commit`], which clones the document,
//! applies the change, skips no-ops, pushes undo history and marks the document
//! dirty. Placement strokes and drags are transient and commit exactly once at
//! the end, so pointer movement never fills the history.
//!
//! User-facing feedback is left in [`EditorCore::notice`] for the UI to show.

use crate::catalog::{catalog, get_piece};
use crate::document::*;
use crate::geometry::{bounds, inside_board, next_rotation, symmetry_placements};
use crate::history::History;
use crate::persistence::document::{
    create_document, make_pass, parse_document, serialize_document, DocumentError, MAX_PASSES, MAX_PIECES,
};
use crate::persistence::preferences::{GridAppearance, Preferences, SavedPrintPreset};
use crate::persistence::presets::{
    capture_preset, parse_presets, preset_rows, CompositionPreset, PresetRow, MAX_PRESETS,
};
use crate::spatial::{OccupancyIndex, PlaceError};
use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Select,
    Place,
    Hand,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Design,
    Print,
}

#[derive(Clone, Debug)]
struct ClipboardPiece {
    piece: PlacedPiece,
    color: String,
}

struct PlacementStroke {
    before: PressDocument,
    before_dirty: bool,
    latest_generation: u64,
    visited: HashSet<(i32, i32)>,
    index: OccupancyIndex,
    count: usize,
}

pub const PLACE_BLOCKED: &str = "That placement overlaps a piece or leaves the artboard.";
pub const PASS_UNAVAILABLE: &str = "Choose a visible, unlocked ink pass to place pieces.";

pub struct EditorCore {
    doc: PressDocument,
    /// Increments whenever `doc` changes; renderers cache on it.
    generation: u64,
    pub selected: Vec<Id>,
    pub active_pass_id: Id,
    pub active_piece_id: String,
    pub placement_rotation: i32,
    pub placement_mirror_x: bool,
    pub placement_mirror_y: bool,
    pub tool: Tool,
    pub mode: Mode,
    pub presets: Vec<CompositionPreset>,
    /// Increments whenever `presets` changes, so the app knows to persist them.
    pub presets_generation: u64,
    pub active_preset_id: Option<String>,
    pub preset_rotation: i32,
    pub preferences: Preferences,
    pub preferences_generation: u64,
    pub dirty: bool,
    pub notice: Option<String>,
    history: History<PressDocument>,
    stroke: Option<PlacementStroke>,
    live: Option<PressDocument>,
    clipboard: Vec<ClipboardPiece>,
    index_cache: RefCell<Option<(u64, OccupancyIndex)>>,
}

impl Default for EditorCore {
    fn default() -> Self {
        Self::new(create_document(true))
    }
}

impl EditorCore {
    pub fn new(doc: PressDocument) -> Self {
        let active_pass_id = doc.passes[0].id.clone();
        Self {
            doc,
            generation: 1,
            selected: Vec::new(),
            active_pass_id,
            active_piece_id: "25269".into(),
            placement_rotation: 0,
            placement_mirror_x: false,
            placement_mirror_y: false,
            tool: Tool::Select,
            mode: Mode::Design,
            presets: Vec::new(),
            presets_generation: 0,
            active_preset_id: None,
            preset_rotation: 0,
            preferences: Preferences::default(),
            preferences_generation: 0,
            dirty: false,
            notice: None,
            history: History::default(),
            stroke: None,
            live: None,
            clipboard: Vec::new(),
            index_cache: RefCell::new(None),
        }
    }

    // ---- read access -------------------------------------------------------

    pub fn doc(&self) -> &PressDocument {
        &self.doc
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
    pub fn active_pass(&self) -> &InkPass {
        self.doc.pass(&self.active_pass_id).unwrap_or(&self.doc.passes[0])
    }
    pub fn active_preset(&self) -> Option<&CompositionPreset> {
        let id = self.active_preset_id.as_deref()?;
        self.presets.iter().find(|p| p.id == id)
    }
    pub fn is_selected(&self, uid: &str) -> bool {
        self.selected.iter().any(|s| &**s == uid)
    }
    /// Selected pieces on unlocked passes, with their pass.
    pub fn selected_pieces(&self) -> Vec<(&InkPass, &PlacedPiece)> {
        let ids: HashSet<&str> = self.selected.iter().map(|s| &**s).collect();
        self.doc.all_pieces().filter(|(pass, p)| ids.contains(&*p.uid) && !pass.locked).collect()
    }
    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }
    pub fn notify(&mut self, message: impl Into<String>) {
        self.notice = Some(message.into());
    }
    pub fn project_text(&self) -> String {
        serialize_document(&self.doc)
    }

    /// Occupancy of the current document, cached per generation.
    fn with_index<R>(&self, f: impl FnOnce(&OccupancyIndex) -> R) -> R {
        let mut cache = self.index_cache.borrow_mut();
        if cache.as_ref().is_none_or(|(g, _)| *g != self.generation) {
            *cache = Some((self.generation, OccupancyIndex::build(&self.doc, &HashSet::new())));
        }
        f(&cache.as_ref().expect("just built").1)
    }

    pub fn check_placement(&self, candidates: &[PlacedPiece]) -> Result<(), PlaceError> {
        self.with_index(|index| index.check(candidates).map_err(|(_, e)| e))
    }

    fn set_doc(&mut self, doc: PressDocument) {
        self.doc = doc;
        self.generation += 1;
    }

    // ---- history -----------------------------------------------------------

    /// Applies `change` to a copy of the document as one undoable step.
    /// Returns false when the change made no difference.
    pub fn commit(&mut self, change: impl FnOnce(&mut PressDocument)) -> bool {
        self.finish_placement_stroke(false);
        self.end_live();
        let mut draft = self.doc.clone();
        change(&mut draft);
        if draft == self.doc {
            return false;
        }
        let previous = std::mem::replace(&mut self.doc, draft);
        self.generation += 1;
        self.history.push(previous);
        self.dirty = true;
        self.prune_selection();
        true
    }

    /// Applies a continuous change (a slider drag, a colour being picked)
    /// without history. The whole gesture becomes one undo step when
    /// [`end_live`](Self::end_live) is called.
    pub fn edit_live(&mut self, change: impl FnOnce(&mut PressDocument)) {
        self.finish_placement_stroke(false);
        let mut draft = self.doc.clone();
        change(&mut draft);
        if draft == self.doc {
            return;
        }
        let previous = std::mem::replace(&mut self.doc, draft);
        if self.live.is_none() {
            self.live = Some(previous);
        }
        self.generation += 1;
        self.dirty = true;
    }

    pub fn live_active(&self) -> bool {
        self.live.is_some()
    }

    /// Ends a live gesture, recording it as one step if anything changed.
    pub fn end_live(&mut self) {
        if let Some(before) = self.live.take() {
            if before != self.doc {
                self.history.push(before);
                self.prune_selection();
            }
        }
    }

    /// Merges the most recent step into the one before it (for operations
    /// that are naturally one action but are built from two commits).
    pub fn squash_last_step(&mut self) {
        self.history.discard_last();
    }

    pub fn undo(&mut self) -> bool {
        self.finish_placement_stroke(false);
        self.end_live();
        let current = self.doc.clone();
        match self.history.undo(current) {
            Some(previous) => {
                self.set_doc(previous);
                self.dirty = true;
                self.prune_selection();
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        self.finish_placement_stroke(false);
        self.end_live();
        let current = self.doc.clone();
        match self.history.redo(current) {
            Some(next) => {
                self.set_doc(next);
                self.dirty = true;
                self.prune_selection();
                true
            }
            None => false,
        }
    }

    pub fn prune_selection(&mut self) {
        let valid: HashSet<&str> =
            self.doc.all_pieces().filter(|(pass, _)| pass.editable()).map(|(_, p)| &*p.uid).collect();
        let kept: Vec<Id> = self.selected.iter().filter(|id| valid.contains(&***id)).cloned().collect();
        self.selected = kept;
        if self.doc.pass(&self.active_pass_id).is_none() {
            self.active_pass_id = self.doc.passes[0].id.clone();
        }
    }

    // ---- documents ---------------------------------------------------------

    pub fn new_document(&mut self) {
        self.replace_document(create_document(false), false);
        self.tool = Tool::Select;
    }

    /// Validates and opens a project. On error nothing changes.
    pub fn load(&mut self, text: &str, recovered: bool) -> Result<(), DocumentError> {
        let doc = parse_document(text)?;
        self.replace_document(doc, recovered);
        self.notify(if recovered { "Recovered your last session." } else { "Project opened." });
        Ok(())
    }

    pub fn replace_document(&mut self, doc: PressDocument, dirty: bool) {
        self.finish_placement_stroke(true);
        self.live = None;
        self.active_preset_id = None;
        self.active_pass_id = doc.passes[0].id.clone();
        self.set_doc(doc);
        self.history.clear();
        self.selected.clear();
        self.dirty = dirty;
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Takes over the state that belongs to the editor window rather than to
    /// one document (the preset library, preferences, clipboard and the piece
    /// being placed) from `other`, for windows that edit several documents.
    pub fn adopt_shared(&mut self, other: &mut EditorCore) {
        self.presets = std::mem::take(&mut other.presets);
        self.presets_generation = other.presets_generation;
        self.preferences = std::mem::take(&mut other.preferences);
        self.preferences_generation = other.preferences_generation;
        self.clipboard = std::mem::take(&mut other.clipboard);
        self.active_piece_id = other.active_piece_id.clone();
        self.placement_rotation = other.placement_rotation;
        self.placement_mirror_x = other.placement_mirror_x;
        self.placement_mirror_y = other.placement_mirror_y;
        if self.active_preset().is_none() {
            self.active_preset_id = None;
            if self.tool == Tool::Place && get_piece(&self.active_piece_id).is_none() {
                self.tool = Tool::Select;
            }
        }
    }

    // ---- selection and tools -----------------------------------------------

    pub fn select(&mut self, uid: &str, additive: bool) {
        let Some((pass, p)) = self.doc.find_piece(uid) else { return };
        if !pass.editable() {
            return;
        }
        let (pass_id, uid) = (pass.id.clone(), p.uid.clone());
        self.active_pass_id = pass_id;
        if additive {
            if let Some(i) = self.selected.iter().position(|s| *s == uid) {
                self.selected.remove(i);
            } else {
                self.selected.push(uid);
            }
        } else {
            self.selected = vec![uid];
        }
    }

    /// Replaces (or extends) the selection with editable pieces.
    pub fn select_many(&mut self, uids: impl IntoIterator<Item = Id>, additive: bool) {
        let editable: HashSet<&str> =
            self.doc.all_pieces().filter(|(pass, _)| pass.editable()).map(|(_, p)| &*p.uid).collect();
        let mut next = if additive { self.selected.clone() } else { Vec::new() };
        for uid in uids {
            if editable.contains(&*uid) && !next.contains(&uid) {
                next.push(uid);
            }
        }
        self.selected = next;
    }

    pub fn select_all(&mut self) {
        let all: Vec<Id> = self.doc.all_pieces().map(|(_, p)| p.uid.clone()).collect();
        self.select_many(all, false);
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn choose_piece(&mut self, id: &str) {
        let Some(p) = get_piece(id) else { return };
        if self.doc.options.physical && !p.is_physical() {
            return;
        }
        self.active_piece_id = id.into();
        self.active_preset_id = None;
        self.placement_rotation = p.allowed_rotations[0];
        self.placement_mirror_x = false;
        self.placement_mirror_y = false;
        self.tool = Tool::Place;
        self.mode = Mode::Design;
        self.selected.clear();
    }

    /// Samples a placed piece's shape and orientation into the Place tool.
    pub fn pick_piece(&mut self, uid: &str) {
        let Some((pass, p)) = self.doc.find_piece(uid) else { return };
        if !pass.visible {
            return;
        }
        let Some(piece) = get_piece(&p.piece_id) else { return };
        if self.doc.options.physical && !piece.is_physical() {
            return;
        }
        let (rotation, mx, my) = (p.rotation, p.mx(), p.my());
        self.choose_piece(&piece.id);
        self.placement_rotation = rotation;
        self.placement_mirror_x = mx;
        self.placement_mirror_y = my;
        self.notify(format!("Picked {}.", piece.name));
    }

    pub fn set_tool(&mut self, tool: Tool) {
        if tool != Tool::Place {
            self.finish_placement_stroke(false);
        }
        self.tool = tool;
    }

    pub fn set_mode(&mut self, mode: Mode) {
        self.finish_placement_stroke(false);
        self.mode = mode;
    }

    // ---- placement ---------------------------------------------------------

    /// Placement previews for the active piece at (x, y), including symmetry.
    pub fn ghosts(&self, x: i32, y: i32) -> Vec<PlacedPiece> {
        self.ghosts_for(x, y, &self.active_piece_id)
    }

    pub fn ghosts_for(&self, x: i32, y: i32, piece_id: &str) -> Vec<PlacedPiece> {
        let Some(p) = get_piece(piece_id) else { return vec![] };
        let rotation = if p.allows(self.placement_rotation) { self.placement_rotation } else { p.allowed_rotations[0] };
        let ghost = PlacedPiece::new("ghost", piece_id, x, y, rotation, 1)
            .with_mirror(self.placement_mirror_x, self.placement_mirror_y);
        symmetry_placements(&ghost, &self.doc)
    }

    /// Why a placement at (x, y) would fail, without side effects.
    pub fn placement_problem(&self, x: i32, y: i32) -> Option<String> {
        if let Some(preset) = self.active_preset() {
            return self.preset_problem(preset, x, y);
        }
        let piece = get_piece(&self.active_piece_id)?;
        if self.doc.options.physical && !piece.is_physical() {
            return Some("This piece is not standard height. Disable physical mode to use it.".into());
        }
        if !self.active_pass().editable() {
            return Some(PASS_UNAVAILABLE.into());
        }
        let ghosts = self.ghosts(x, y);
        if self.doc.piece_count() + ghosts.len() > MAX_PIECES {
            return Some("Placement would exceed the project limit of 10,000 pieces.".into());
        }
        match self.check_placement(&ghosts) {
            Ok(()) => None,
            Err(PlaceError::OutOfBounds) => Some("That placement leaves the artboard.".into()),
            Err(_) => Some("That placement overlaps a piece.".into()),
        }
    }

    fn placement_candidates(&mut self, x: i32, y: i32, quiet: bool, count: usize) -> Option<Vec<PlacedPiece>> {
        let piece = get_piece(&self.active_piece_id)?;
        if self.doc.options.physical && !piece.is_physical() {
            return None;
        }
        if !self.active_pass().editable() {
            if !quiet {
                self.notify(PASS_UNAVAILABLE);
            }
            return None;
        }
        let candidates: Vec<PlacedPiece> =
            self.ghosts(x, y).into_iter().map(|p| PlacedPiece { uid: new_id(), seed: new_seed(), ..p }).collect();
        if count + candidates.len() > MAX_PIECES {
            let message = "Placement would exceed the project limit of 10,000 pieces.";
            if self.notice.as_deref() != Some(message) {
                self.notify(message);
            }
            return None;
        }
        Some(candidates)
    }

    /// Places the active piece (or preset) once, as its own undo step.
    pub fn place(&mut self, x: i32, y: i32) -> bool {
        self.finish_placement_stroke(false);
        if self.active_preset_id.is_some() {
            return self.place_preset(x, y);
        }
        let count = self.doc.piece_count();
        let Some(candidates) = self.placement_candidates(x, y, false, count) else { return false };
        if self.check_placement(&candidates).is_err() {
            self.notify(PLACE_BLOCKED);
            return false;
        }
        let pass_id = self.active_pass().id.clone();
        self.commit(|doc| doc.pass_mut(&pass_id).expect("active pass").pieces_mut().extend(candidates))
    }

    pub fn begin_placement_stroke(&mut self) -> bool {
        self.finish_placement_stroke(false);
        if self.active_preset_id.is_some() || self.mode != Mode::Design || self.tool != Tool::Place {
            return false;
        }
        let index = self.with_index(|i| i.clone());
        self.stroke = Some(PlacementStroke {
            before: self.doc.clone(),
            before_dirty: self.dirty,
            latest_generation: self.generation,
            visited: HashSet::new(),
            index,
            count: self.doc.piece_count(),
        });
        true
    }

    pub fn placement_stroke_active(&self) -> bool {
        self.stroke.is_some()
    }

    /// Adds pieces at an anchor during a stroke. Revisited anchors are skipped.
    pub fn paint_placement(&mut self, x: i32, y: i32) -> bool {
        let Some(stroke) = self.stroke.as_mut() else { return false };
        if stroke.latest_generation != self.generation
            || self.tool != Tool::Place
            || self.mode != Mode::Design
            || self.active_preset_id.is_some()
        {
            return false;
        }
        if !stroke.visited.insert((x, y)) {
            return false;
        }
        let quiet = stroke.visited.len() > 1;
        let count = stroke.count;
        let Some(candidates) = self.placement_candidates(x, y, quiet, count) else { return false };
        let stroke = self.stroke.as_mut().expect("still active");
        if !stroke.index.can_place(&candidates) {
            if !quiet {
                self.notify(PLACE_BLOCKED);
            }
            return false;
        }
        stroke.index.add(&candidates);
        stroke.count += candidates.len();
        let pass_id = self.active_pass().id.clone();
        if let Some(pass) = self.doc.pass_mut(&pass_id) {
            pass.pieces_mut().extend(candidates);
        }
        self.generation += 1;
        let stroke = self.stroke.as_mut().expect("still active");
        stroke.latest_generation = self.generation;
        self.dirty = true;
        true
    }

    /// Ends a stroke: one undo step, or a full revert when `cancelled`.
    pub fn finish_placement_stroke(&mut self, cancelled: bool) {
        let Some(stroke) = self.stroke.take() else { return };
        if stroke.latest_generation != self.generation || stroke.before == self.doc {
            return;
        }
        if cancelled {
            self.set_doc(stroke.before);
            self.dirty = stroke.before_dirty;
            self.prune_selection();
        } else {
            self.history.push(stroke.before);
        }
    }

    // ---- transforms --------------------------------------------------------

    pub fn transform_selected(&mut self, transform: impl Fn(&PlacedPiece) -> PlacedPiece) -> bool {
        let candidates: Vec<PlacedPiece> = self.selected_pieces().into_iter().map(|(_, p)| transform(p)).collect();
        if candidates.is_empty() {
            return false;
        }
        let ignored: HashSet<&str> = self.selected.iter().map(|s| &**s).collect();
        if OccupancyIndex::build(&self.doc, &ignored).check(&candidates).is_err() {
            self.notify("The transform needs more space on the grid.");
            return false;
        }
        self.replace_pieces(candidates)
    }

    /// Replaces pieces by uid, as one step. The caller validates placement.
    pub fn replace_pieces(&mut self, updated: Vec<PlacedPiece>) -> bool {
        let updates: std::collections::HashMap<Id, PlacedPiece> =
            updated.into_iter().map(|p| (p.uid.clone(), p)).collect();
        self.commit(|doc| {
            for pass in &mut doc.passes {
                if pass.pieces.iter().any(|p| updates.contains_key(&p.uid)) {
                    for p in pass.pieces_mut() {
                        if let Some(next) = updates.get(&p.uid) {
                            *p = next.clone();
                        }
                    }
                }
            }
        })
    }

    /// Checks whether the selection fits when offset by (dx, dy); used during drags.
    pub fn can_move_selected(&self, dx: i32, dy: i32) -> bool {
        let moved: Vec<PlacedPiece> = self
            .selected_pieces()
            .into_iter()
            .map(|(_, p)| PlacedPiece { x: p.x + dx, y: p.y + dy, ..p.clone() })
            .collect();
        let ignored: HashSet<&str> = self.selected.iter().map(|s| &**s).collect();
        !moved.is_empty() && OccupancyIndex::build(&self.doc, &ignored).can_place(&moved)
    }

    pub fn move_selected(&mut self, dx: i32, dy: i32) -> bool {
        self.transform_selected(|p| PlacedPiece { x: p.x + dx, y: p.y + dy, ..p.clone() })
    }

    pub fn rotate(&mut self) {
        if self.tool == Tool::Place && self.selected.is_empty() {
            if self.active_preset_id.is_some() {
                self.preset_rotation = (self.preset_rotation + 90) % 360;
                return;
            }
            if let Some(piece) = get_piece(&self.active_piece_id) {
                let r = &piece.allowed_rotations;
                let i = r.iter().position(|&v| v == self.placement_rotation).map_or(0, |i| i + 1);
                self.placement_rotation = r[i % r.len()];
            }
        } else {
            self.transform_selected(next_rotation);
        }
    }

    /// Mirrors the selection (or the placement tool) across the vertical or horizontal axis.
    pub fn mirror(&mut self, horizontal: bool) {
        if self.tool == Tool::Place && self.selected.is_empty() {
            if horizontal {
                self.placement_mirror_x = !self.placement_mirror_x;
            } else {
                self.placement_mirror_y = !self.placement_mirror_y;
            }
            return;
        }
        self.transform_selected(|p| {
            let (mx, my) = if horizontal { (!p.mx(), p.my()) } else { (p.mx(), !p.my()) };
            p.clone().with_mirror(mx, my)
        });
    }

    pub fn remove_selected(&mut self) -> bool {
        let ids: HashSet<Id> = self.selected_pieces().into_iter().map(|(_, p)| p.uid.clone()).collect();
        let changed = self.remove_pieces(&ids);
        self.selected.clear();
        changed
    }

    pub fn remove_pieces(&mut self, ids: &HashSet<Id>) -> bool {
        self.commit(|doc| {
            for pass in &mut doc.passes {
                if pass.pieces.iter().any(|p| ids.contains(&p.uid)) {
                    pass.pieces_mut().retain(|p| !ids.contains(&p.uid));
                }
            }
        })
    }

    pub fn copy(&mut self) {
        self.clipboard = self
            .selected_pieces()
            .into_iter()
            .map(|(pass, p)| ClipboardPiece { piece: p.clone(), color: pass.color.clone() })
            .collect();
        let n = self.clipboard.len();
        if n > 0 {
            self.notify(format!("Copied {n} piece{}.", if n == 1 { "" } else { "s" }));
        }
    }

    pub fn cut(&mut self) {
        self.copy();
        self.remove_selected();
    }

    pub fn has_clipboard(&self) -> bool {
        !self.clipboard.is_empty()
    }

    pub fn duplicate(&mut self) {
        let source: Vec<ClipboardPiece> = self
            .selected_pieces()
            .into_iter()
            .map(|(pass, p)| ClipboardPiece { piece: p.clone(), color: pass.color.clone() })
            .collect();
        self.insert_copy(source);
    }

    pub fn paste(&mut self) {
        self.insert_copy(self.clipboard.clone());
    }

    fn insert_copy(&mut self, source: Vec<ClipboardPiece>) {
        if source.is_empty() {
            return;
        }
        if self.doc.options.physical
            && source.iter().any(|c| get_piece(&c.piece.piece_id).is_none_or(|p| !p.is_physical()))
        {
            self.notify("The clipboard contains a non-standard-height piece. Disable physical mode to paste it.");
            return;
        }
        if self.doc.piece_count() + source.len() > MAX_PIECES {
            self.notify("Pasting would exceed the project limit of 10,000 pieces.");
            return;
        }
        // Search outward in rings, keeping relative positions intact.
        let limit = self.doc.board.width.max(self.doc.board.height);
        let mut candidates = Vec::new();
        'outer: for distance in 1..=limit {
            for dx in -distance..=distance {
                for dy in -distance..=distance {
                    if dx.abs().max(dy.abs()) != distance {
                        continue;
                    }
                    let group: Vec<PlacedPiece> = source
                        .iter()
                        .map(|c| PlacedPiece {
                            uid: new_id(),
                            seed: new_seed(),
                            x: c.piece.x + dx,
                            y: c.piece.y + dy,
                            ..c.piece.clone()
                        })
                        .collect();
                    if self.check_placement(&group).is_ok() {
                        candidates = group;
                        break 'outer;
                    }
                }
            }
        }
        if candidates.is_empty() {
            self.notify("There is no room for this group on the artboard.");
            return;
        }
        let placed: Vec<Id> = candidates.iter().map(|p| p.uid.clone()).collect();
        self.commit(|doc| {
            for (piece, c) in candidates.into_iter().zip(&source) {
                let index = doc.passes.iter().position(|p| p.color == c.color && p.editable());
                let index = index.unwrap_or_else(|| {
                    let name = format!("Ink Pass {}", doc.passes.len() + 1);
                    doc.passes.push(make_pass(&c.color, &name));
                    doc.passes.len() - 1
                });
                doc.passes[index].pieces_mut().push(piece);
            }
        });
        self.selected = placed;
        self.tool = Tool::Select;
    }

    /// Moves the selection to the pass with this ink (creating it), or makes
    /// that pass active when nothing is selected.
    pub fn set_color(&mut self, color: &str) -> bool {
        if !valid_color(color) {
            self.notify("Use a six-digit HEX ink color, such as #315EAE.");
            return false;
        }
        let color = color.to_ascii_lowercase();
        let target = self.doc.passes.iter().find(|p| p.color.eq_ignore_ascii_case(&color) && p.editable());
        let target_id = target.map(|p| p.id.clone());
        if self.selected_pieces().is_empty() {
            match target_id {
                Some(id) => self.active_pass_id = id,
                None => {
                    self.add_pass(&color);
                }
            }
            return true;
        }
        let target_id = target_id.unwrap_or_else(new_id);
        let ids: HashSet<Id> = self.selected_pieces().into_iter().map(|(_, p)| p.uid.clone()).collect();
        let id = target_id.clone();
        self.commit(move |doc| {
            let mut moved = Vec::new();
            for pass in &mut doc.passes {
                if pass.pieces.iter().any(|p| ids.contains(&p.uid)) {
                    let pieces = pass.pieces_mut();
                    moved.extend(pieces.iter().filter(|p| ids.contains(&p.uid)).cloned());
                    pieces.retain(|p| !ids.contains(&p.uid));
                }
            }
            if doc.pass(&id).is_none() {
                let name = format!("Ink Pass {}", doc.passes.len() + 1);
                doc.passes.push(InkPass { id: id.clone(), ..make_pass(&color, &name) });
            }
            doc.pass_mut(&id).expect("exists").pieces_mut().extend(moved);
        });
        self.active_pass_id = target_id;
        true
    }

    // ---- passes ------------------------------------------------------------

    pub fn add_pass(&mut self, color: &str) -> Option<Id> {
        if self.doc.passes.len() >= MAX_PASSES {
            self.notify("Use up to 100 ink passes.");
            return None;
        }
        let pass = make_pass(color, &format!("Ink Pass {}", self.doc.passes.len() + 1));
        let id = pass.id.clone();
        self.commit(|doc| doc.passes.push(pass));
        self.active_pass_id = id.clone();
        Some(id)
    }

    pub fn update_pass(&mut self, id: &str, update: impl FnOnce(&mut InkPass)) -> bool {
        self.commit(|doc| {
            if let Some(pass) = doc.pass_mut(id) {
                update(pass);
            }
        })
    }

    pub fn rename_pass(&mut self, id: &str, name: &str) -> bool {
        let name: String = name.trim().chars().take(160).collect();
        if name.is_empty() {
            return false;
        }
        self.update_pass(id, |p| p.name = name)
    }

    pub fn move_to_pass(&mut self, id: &str) -> bool {
        match self.doc.pass(id) {
            Some(target) if target.editable() => {}
            _ => {
                self.notify("Choose a visible, unlocked pass.");
                return false;
            }
        }
        let ids: HashSet<Id> = self.selected_pieces().into_iter().map(|(_, p)| p.uid.clone()).collect();
        let changed = self.commit(|doc| {
            let mut moved = Vec::new();
            for pass in &mut doc.passes {
                if pass.pieces.iter().any(|p| ids.contains(&p.uid)) {
                    let pieces = pass.pieces_mut();
                    moved.extend(pieces.iter().filter(|p| ids.contains(&p.uid)).cloned());
                    pieces.retain(|p| !ids.contains(&p.uid));
                }
            }
            doc.pass_mut(id).expect("checked").pieces_mut().extend(moved);
        });
        self.active_pass_id = id.into();
        changed
    }

    pub fn reorder_pass(&mut self, id: &str, direction: i32) -> bool {
        let Some(index) = self.doc.pass_index(id) else { return false };
        self.move_pass(id, index as i32 + direction)
    }

    pub fn move_pass(&mut self, id: &str, next: i32) -> bool {
        let Some(index) = self.doc.pass_index(id) else { return false };
        if next < 0 || next as usize >= self.doc.passes.len() || index == next as usize {
            return false;
        }
        self.commit(|doc| {
            let pass = doc.passes.remove(index);
            doc.passes.insert(next as usize, pass);
        })
    }

    /// Removes a pass. Callers confirm first when it still holds pieces.
    pub fn remove_pass(&mut self, id: &str) -> bool {
        if self.doc.passes.len() == 1 {
            self.notify("Keep at least one ink pass.");
            return false;
        }
        self.commit(|doc| doc.passes.retain(|p| &*p.id != id))
    }

    pub fn reorder_selected(&mut self, direction: i32) -> bool {
        let ids: HashSet<Id> = self.selected.iter().cloned().collect();
        self.commit(|doc| {
            for pass in &mut doc.passes {
                if !pass.pieces.iter().any(|p| ids.contains(&p.uid)) {
                    continue;
                }
                let pieces = pass.pieces_mut();
                if direction > 0 {
                    for i in (0..pieces.len().saturating_sub(1)).rev() {
                        if ids.contains(&pieces[i].uid) && !ids.contains(&pieces[i + 1].uid) {
                            pieces.swap(i, i + 1);
                        }
                    }
                } else {
                    for i in 1..pieces.len() {
                        if ids.contains(&pieces[i].uid) && !ids.contains(&pieces[i - 1].uid) {
                            pieces.swap(i, i - 1);
                        }
                    }
                }
            }
        })
    }

    // ---- document settings -------------------------------------------------

    pub fn resize(&mut self, width: i32, height: i32) -> bool {
        if !(1..=128).contains(&width) || !(1..=128).contains(&height) {
            self.notify("Use whole stud dimensions from 1 to 128.");
            return false;
        }
        let board = Board { width, height };
        if self.doc.all_pieces().any(|(_, p)| !inside_board(&bounds(p), board)) {
            self.notify("Move or clear the pieces beyond the new board size first.");
            return false;
        }
        self.commit(|doc| doc.board = board)
    }

    pub fn set_physical(&mut self, physical: bool) -> bool {
        if physical && self.doc.all_pieces().any(|(_, p)| get_piece(&p.piece_id).is_none_or(|x| !x.is_physical())) {
            self.notify("Remove the non-standard-height pieces before restricting printing heights.");
            return false;
        }
        let changed = self.commit(|doc| doc.options.physical = physical);
        if physical && get_piece(&self.active_piece_id).is_none_or(|p| !p.is_physical()) {
            self.choose_piece("3070");
        }
        changed
    }

    pub fn set_allow_overlap(&mut self, allow: bool) -> bool {
        if !allow {
            let all: Vec<PlacedPiece> = self.doc.all_pieces().map(|(_, p)| p.clone()).collect();
            let mut strict = self.doc.clone();
            strict.options.allow_overlap = false;
            if !OccupancyIndex::empty(&strict).can_place(&all) {
                self.notify("Separate the overlapping pieces before turning overlap off.");
                return false;
            }
        }
        self.commit(|doc| doc.options.allow_overlap = allow)
    }

    pub fn clear(&mut self) -> bool {
        let changed = self.commit(|doc| {
            for pass in &mut doc.passes {
                if !pass.pieces.is_empty() {
                    pass.pieces = Arc::new(Vec::new());
                }
            }
        });
        self.selected.clear();
        changed
    }

    pub fn reseed(&mut self) -> bool {
        self.commit(|doc| {
            for pass in &mut doc.passes {
                pass.registration_seed = new_seed();
                for p in pass.pieces_mut() {
                    p.seed = new_seed();
                }
            }
            doc.paper.seed = new_seed();
        })
    }

    pub fn rename(&mut self, name: &str) -> bool {
        let name: String = name.trim().chars().take(160).collect();
        let name = if name.is_empty() { "Untitled impression".to_string() } else { name };
        self.commit(|doc| doc.name = name)
    }

    // ---- composition presets -----------------------------------------------

    fn presets_changed(&mut self) {
        self.presets_generation += 1;
    }

    pub fn restore_presets(&mut self, text: &str) -> Result<(), DocumentError> {
        self.presets = parse_presets(text)?;
        if self.active_preset().is_none() {
            self.active_preset_id = None;
        }
        Ok(())
    }

    pub fn save_preset(&mut self, name: &str, selection_only: bool) -> bool {
        if self.presets.len() >= MAX_PRESETS {
            self.notify("The library can hold up to 100 presets.");
            return false;
        }
        let selected: HashSet<&str> = self.selected.iter().map(|s| &**s).collect();
        match capture_preset(&self.doc, name, selection_only.then_some(&selected)) {
            Ok(preset) => {
                self.notify(format!("Saved “{}” to your presets.", preset.name));
                self.presets.push(preset);
                self.presets_changed();
                true
            }
            Err(e) => {
                self.notify(e.0);
                false
            }
        }
    }

    pub fn rename_preset(&mut self, id: &str, name: &str) -> bool {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            self.notify("Use a preset name of 1–80 characters.");
            return false;
        }
        let Some(preset) = self.presets.iter_mut().find(|p| p.id == id) else { return false };
        preset.name = name.into();
        self.presets_changed();
        true
    }

    pub fn delete_preset(&mut self, id: &str) -> bool {
        let before = self.presets.len();
        self.presets.retain(|p| p.id != id);
        if self.presets.len() == before {
            return false;
        }
        if self.active_preset_id.as_deref() == Some(id) {
            self.active_preset_id = None;
            self.tool = Tool::Select;
        }
        self.presets_changed();
        true
    }

    pub fn import_presets(&mut self, text: &str) -> Result<usize, DocumentError> {
        let imported = parse_presets(text)?;
        let mut next = self.presets.clone();
        let mut added = 0;
        for p in imported {
            if next.contains(&p) {
                continue;
            }
            let id = if next.iter().any(|s| s.id == p.id) { new_id().to_string() } else { p.id.clone() };
            next.push(CompositionPreset { id, ..p });
            added += 1;
        }
        if next.len() > MAX_PRESETS {
            return Err(DocumentError("The library can hold up to 100 presets.".into()));
        }
        self.presets = next;
        self.presets_changed();
        self.notify(format!("Imported {added} composition presets."));
        Ok(added)
    }

    /// Stops placing a preset without placing it, back to the Select tool.
    pub fn cancel_preset(&mut self) {
        if self.active_preset_id.take().is_some() {
            self.preset_rotation = 0;
            self.tool = Tool::Select;
        }
    }

    pub fn choose_preset(&mut self, id: &str) {
        if !self.presets.iter().any(|p| p.id == id) {
            return;
        }
        self.active_preset_id = Some(id.into());
        self.preset_rotation = 0;
        self.tool = Tool::Place;
        self.mode = Mode::Design;
        self.selected.clear();
    }

    pub fn preset_preview(&self, x: i32, y: i32) -> Vec<PresetRow> {
        self.active_preset().map(|p| preset_rows(p, x, y, self.preset_rotation)).unwrap_or_default()
    }

    fn preset_problem(&self, preset: &CompositionPreset, x: i32, y: i32) -> Option<String> {
        let rows = preset_rows(preset, x, y, self.preset_rotation);
        if self.doc.options.physical
            && rows.iter().any(|r| get_piece(&r.piece.piece_id).is_none_or(|p| !p.is_physical()))
        {
            return Some("This preset contains non-standard-height pieces. Disable physical mode to place it.".into());
        }
        let pieces: Vec<PlacedPiece> = rows.iter().map(|r| r.piece.clone()).collect();
        if rows.is_empty() || self.check_placement(&pieces).is_err() {
            return Some(
                "This preset overlaps pieces or does not fit the canvas. Try another position or a larger canvas."
                    .into(),
            );
        }
        if self.doc.passes.len() + preset.passes.len() > MAX_PASSES || self.doc.piece_count() + rows.len() > MAX_PIECES
        {
            return Some("Placement would exceed the project limit of 100 ink passes or 10,000 pieces.".into());
        }
        None
    }

    pub fn place_preset(&mut self, x: i32, y: i32) -> bool {
        let Some(preset) = self.active_preset().cloned() else { return false };
        if let Some(problem) = self.preset_problem(&preset, x, y) {
            self.notify(problem);
            return false;
        }
        let rows = preset_rows(&preset, x, y, self.preset_rotation);
        let mut passes: Vec<InkPass> =
            preset.passes.iter().map(|p| make_pass(&p.color, &format!("{} · {}", preset.name, p.name))).collect();
        for row in rows {
            passes[row.pass_index].pieces_mut().push(PlacedPiece { uid: new_id(), seed: new_seed(), ..row.piece });
        }
        let selected: Vec<Id> = passes.iter().flat_map(|p| p.pieces.iter().map(|x| x.uid.clone())).collect();
        let first = passes[0].id.clone();
        self.commit(|doc| doc.passes.extend(passes));
        self.selected = selected;
        self.active_pass_id = first;
        self.active_preset_id = None;
        self.tool = Tool::Select;
        self.notify(format!("Placed “{}”.", preset.name));
        true
    }

    // ---- preferences -------------------------------------------------------

    pub fn set_grid_appearance(&mut self, appearance: GridAppearance) {
        if self.preferences.grid_appearance != appearance {
            self.preferences.grid_appearance = appearance;
            self.preferences_generation += 1;
        }
    }

    pub fn save_print_preset(&mut self, name: &str) -> bool {
        let name: String = name.trim().chars().take(80).collect();
        if name.is_empty() {
            return false;
        }
        let existing = self.preferences.print_presets.iter().find(|r| r.name.to_lowercase() == name.to_lowercase());
        if existing.is_none() && self.preferences.print_presets.len() >= 100 {
            self.notify("Save up to 100 print presets.");
            return false;
        }
        let id = existing.map_or_else(|| new_id().to_string(), |r| r.id.clone());
        let row = SavedPrintPreset { id: id.clone(), name: name.clone(), settings: self.doc.print_settings };
        self.preferences.print_presets.retain(|r| r.id != id);
        self.preferences.print_presets.push(row);
        self.preferences_generation += 1;
        self.notify(format!("Print preset “{name}” saved on this device."));
        true
    }

    pub fn delete_print_preset(&mut self, id: &str) {
        self.preferences.print_presets.retain(|r| r.id != id);
        self.preferences_generation += 1;
    }

    /// Board size in studs and the catalog's default, for UI hints.
    pub fn default_board() -> Board {
        let b = &catalog().rendering.recommended_default_board;
        Board { width: b.width_studs, height: b.height_studs }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor() -> EditorCore {
        let mut e = EditorCore::default();
        e.new_document();
        e.choose_piece("3070");
        e
    }

    #[test]
    fn stroke_shows_pieces_creates_one_undo_step_and_keeps_seeds() {
        let mut e = editor();
        let before = e.project_text();
        e.begin_placement_stroke();
        for x in 2..7 {
            assert!(e.paint_placement(x, 3));
        }
        assert_eq!(e.doc().piece_count(), 5);
        e.finish_placement_stroke(false);
        let after = e.project_text();
        e.undo();
        assert_eq!(e.project_text(), before);
        assert!(!e.can_undo());
        e.redo();
        assert_eq!(e.project_text(), after);
    }

    #[test]
    fn shared_state_moves_between_documents_and_documents_stay_apart() {
        let mut first = editor();
        first.place(2, 2);
        first.select_all();
        first.copy();
        first.save_preset("Dot", false);
        first.placement_rotation = 90;
        let mut second = EditorCore::new(create_document(false));
        second.adopt_shared(&mut first);
        assert_eq!(second.presets.len(), 1);
        assert_eq!(second.presets_generation, first.presets_generation);
        assert!(second.has_clipboard() && !first.has_clipboard());
        assert_eq!((second.active_piece_id.as_str(), second.placement_rotation), ("3070", 90));
        second.paste();
        assert_eq!(second.doc().piece_count(), 1, "pastes between documents");
        assert_eq!(first.doc().piece_count(), 1);
        assert!(second.can_undo() && first.can_undo(), "each keeps its own history");
        first.adopt_shared(&mut second);
        assert_eq!(first.presets.len(), 1);
        assert!(second.presets.is_empty());
    }

    #[test]
    fn never_stacks_duplicates_on_revisited_anchor() {
        let mut e = editor();
        e.commit(|d| d.options.allow_overlap = true);
        e.begin_placement_stroke();
        e.paint_placement(2, 3);
        e.paint_placement(3, 3);
        e.paint_placement(2, 3);
        e.finish_placement_stroke(false);
        assert_eq!(e.doc().piece_count(), 2);
    }

    #[test]
    fn skips_blocked_anchors_and_uses_rotated_footprint() {
        let mut e = editor();
        e.place(2, 3);
        e.choose_piece("3069");
        e.placement_rotation = 90;
        e.begin_placement_stroke();
        assert!(!e.paint_placement(2, 3));
        assert!(e.paint_placement(3, 3));
        assert!(!e.paint_placement(3, 4));
        assert!(e.paint_placement(3, 5));
        assert!(!e.paint_placement(3, 15));
        e.finish_placement_stroke(false);
        assert_eq!(e.doc().piece_count(), 3);
        assert_eq!(e.doc().passes[0].pieces[1].rotation, 90);
        e.undo();
        assert_eq!(e.doc().piece_count(), 1);
    }

    #[test]
    fn cancel_preset_returns_to_select() {
        let mut core = EditorCore::default();
        core.replace_document(crate::persistence::document::create_document(true), false);
        let preset = capture_preset(core.doc(), "Motif", None).unwrap();
        let id = preset.id.clone();
        core.presets.push(preset);
        core.choose_preset(&id);
        assert_eq!(core.tool, Tool::Place);
        core.cancel_preset();
        assert!(core.active_preset_id.is_none());
        assert_eq!(core.tool, Tool::Select);
        assert!(!core.can_undo(), "nothing was placed");
    }

    #[test]
    fn cancel_keeps_the_redo_branch() {
        let mut e = editor();
        e.place(0, 0);
        let saved = e.project_text();
        e.undo();
        let (before, dirty) = (e.project_text(), e.dirty);
        e.begin_placement_stroke();
        e.paint_placement(5, 5);
        e.finish_placement_stroke(true);
        assert_eq!(e.project_text(), before);
        assert_eq!(e.dirty, dirty);
        assert!(e.can_redo());
        e.redo();
        assert_eq!(e.project_text(), saved);
    }

    #[test]
    fn other_operations_finish_a_live_stroke_and_replacement_documents_end_it() {
        let mut e = editor();
        e.begin_placement_stroke();
        e.paint_placement(2, 2);
        e.commit(|d| d.name = "Changed".into());
        e.finish_placement_stroke(true);
        assert_eq!(e.doc().piece_count(), 1);
        e.undo();
        assert_eq!(e.doc().name, "Untitled impression");
        e.undo();
        assert_eq!(e.doc().piece_count(), 0);
        e.begin_placement_stroke();
        e.paint_placement(4, 4);
        e.load(&serialize_document(&create_document(false)), false).unwrap();
        assert!(!e.paint_placement(5, 5));
        assert_eq!(e.doc().piece_count(), 0);
    }

    #[test]
    fn respects_locks_visibility_physical_symmetry_and_limits() {
        let mut e = editor();
        let id = e.doc().passes[0].id.clone();
        e.update_pass(&id, |p| p.locked = true);
        e.begin_placement_stroke();
        assert!(!e.paint_placement(1, 1));
        e.finish_placement_stroke(false);
        e.update_pass(&id, |p| {
            p.locked = false;
            p.visible = false;
        });
        e.begin_placement_stroke();
        assert!(!e.paint_placement(1, 1));
        e.finish_placement_stroke(false);
        e.update_pass(&id, |p| p.visible = true);
        e.choose_piece("68869");
        e.commit(|d| d.options.physical = true);
        e.begin_placement_stroke();
        assert!(!e.paint_placement(1, 1));
        e.finish_placement_stroke(false);
        e.commit(|d| d.options.physical = false);
        e.choose_piece("3070");
        e.commit(|d| d.options.symmetry = Symmetry::Both);
        e.begin_placement_stroke();
        assert!(e.paint_placement(1, 1));
        e.finish_placement_stroke(false);
        assert_eq!(e.doc().piece_count(), 4);
        e.commit(|d| {
            d.options.allow_overlap = true;
            d.passes[0].pieces =
                Arc::new((0..9999).map(|i| PlacedPiece::new(format!("limit-{i}"), "3070", 0, 0, 0, 1)).collect());
        });
        e.begin_placement_stroke();
        assert!(!e.paint_placement(3, 3));
        e.finish_placement_stroke(false);
        assert_eq!(e.doc().piece_count(), 9999);
    }

    #[test]
    fn moving_a_pass_keeps_identity_and_undoes_as_one_step() {
        let mut e = EditorCore::default();
        let original: Vec<Id> = e.doc().passes.iter().map(|p| p.id.clone()).collect();
        e.active_pass_id = original[0].clone();
        e.move_pass(&original[0], original.len() as i32 - 1);
        assert_eq!(e.doc().passes.last().unwrap().id, original[0]);
        assert_eq!(e.active_pass_id, original[0]);
        e.undo();
        assert_eq!(e.doc().passes.iter().map(|p| p.id.clone()).collect::<Vec<_>>(), original);
        let before = e.project_text();
        e.move_pass("missing", 0);
        e.move_pass(&original[0], -1);
        assert_eq!(e.project_text(), before);
    }

    #[test]
    fn saved_print_presets_do_not_touch_the_project() {
        let mut e = EditorCore::default();
        let before = e.project_text();
        e.save_print_preset("My press");
        assert_eq!(e.preferences.print_presets[0].settings, e.doc().print_settings);
        e.commit(|d| d.print_settings.pressure = 0.21);
        e.save_print_preset("MY PRESS");
        assert_eq!(e.preferences.print_presets.len(), 1);
        assert_eq!(e.preferences.print_presets[0].settings.pressure, 0.21);
        e.undo();
        assert_eq!(e.project_text(), before);
    }

    #[test]
    fn copy_paste_duplicate_and_color_moves() {
        let mut e = editor();
        e.place(2, 2);
        let uid = e.doc().passes[0].pieces[0].uid.clone();
        e.select(&uid, false);
        e.duplicate();
        assert_eq!(e.doc().piece_count(), 2);
        assert_eq!(e.selected.len(), 1);
        assert_ne!(e.selected[0], uid);
        e.copy();
        e.paste();
        assert_eq!(e.doc().piece_count(), 3);
        let passes = e.doc().passes.len();
        e.set_color("#123456");
        assert_eq!(e.doc().passes.len(), passes + 1);
        assert_eq!(e.doc().passes.last().unwrap().pieces.len(), 1);
        assert_eq!(e.active_pass().color, "#123456");
        assert!(!e.set_color("blue"));
    }

    #[test]
    fn move_rotate_mirror_remove_resize_and_physical() {
        let mut e = editor();
        e.choose_piece("3069");
        e.place(0, 0);
        let uid = e.doc().passes[0].pieces[0].uid.clone();
        e.set_tool(Tool::Select);
        e.select(&uid, false);
        assert!(!e.move_selected(-1, 0), "cannot leave the board");
        assert!(e.move_selected(1, 1));
        e.rotate();
        assert_eq!(e.doc().passes[0].pieces[0].rotation, 90);
        e.mirror(true);
        assert_eq!(e.doc().passes[0].pieces[0].mirror_x, Some(true));
        assert!(!e.resize(1, 1));
        assert!(e.resize(20, 12));
        assert!(e.remove_selected());
        assert_eq!(e.doc().piece_count(), 0);
        e.choose_piece("68869");
        e.place(0, 0);
        assert!(!e.set_physical(true));
        e.clear();
        assert!(e.set_physical(true));
        assert_eq!(e.active_piece_id, "3070");
    }

    #[test]
    fn live_edits_become_one_undo_step() {
        let mut e = editor();
        let before = e.project_text();
        for v in [0.1, 0.2, 0.3] {
            e.edit_live(|d| d.print_settings.pressure = v);
        }
        assert!(e.live_active());
        e.end_live();
        assert_eq!(e.doc().print_settings.pressure, 0.3);
        e.undo();
        assert_eq!(e.project_text(), before);
        assert!(!e.can_undo());
    }

    #[test]
    fn presets_place_as_new_passes() {
        let mut e = editor();
        e.place(1, 1);
        e.place(2, 1);
        assert!(e.save_preset("Pair", false));
        let id = e.presets[0].id.clone();
        e.choose_preset(&id);
        assert!(e.placement_problem(15, 0).is_some());
        assert!(e.placement_problem(5, 5).is_none());
        let passes = e.doc().passes.len();
        assert!(e.place(5, 5));
        assert_eq!(e.doc().passes.len(), passes + 1);
        assert_eq!(e.selected.len(), 2);
        assert_eq!(e.tool, Tool::Select);
        let text = crate::persistence::presets::serialize_presets(&e.presets);
        assert_eq!(e.import_presets(&text).unwrap(), 0, "identical presets are skipped");
    }
}
