import { getPiece, isPhysical } from '../catalog/catalog';
import { bounds, canPlace, nextRotation, symmetryPlacements } from '../geometry/geometry';
import { History } from '../history/history';
import {
  createDocument,
  makePass,
  parseDocument,
  serializeDocument
} from '../persistence/document';
import { newId, newSeed } from '../printing/noise';
import type { InkPass, PlacedPiece, PressDocument } from '../types/document';

type ClipboardPiece = { piece: PlacedPiece; color: string; name: string };
export class Editor {
  doc = $state.raw<PressDocument>(createDocument(true));
  selected = $state.raw<string[]>([]);
  activePassId = $state(this.doc.passes[0].id);
  activePieceId = $state('25269');
  placementRotation = $state(0);
  tool = $state<'select' | 'place' | 'hand'>('select');
  mode = $state<'design' | 'print'>('design');
  inspectorTab = $state<'properties' | 'layers' | 'export'>('properties');
  panelResetToken = $state(0);
  gridAppearance = $state<'flat' | 'embossed'>('flat');
  narrow = $state(false);
  drawer = $state<'palette' | 'inspector' | null>(null);
  canvasFocusToken = $state(0);
  focusCanvas() {
    this.drawer = null;
    this.canvasFocusToken++;
  }
  openExport() {
    this.inspectorTab = 'export';
    if (this.narrow) this.drawer = 'inspector';
  }
  toast = $state('');
  dirty = $state(false);
  autosaveStatus = $state('Saved on this device');
  historyVersion = $state(0);
  zoomCommand = $state.raw<{ value: number | 'fit'; token: number }>({ value: 'fit', token: 0 });
  zoomPercent = $state(100);
  private history = new History<PressDocument>();
  private clipboard: ClipboardPiece[] = [];
  private toastTimer?: ReturnType<typeof setTimeout>;
  get activePass() {
    return this.doc.passes.find((p) => p.id === this.activePassId) ?? this.doc.passes[0];
  }
  get allPieces() {
    return this.doc.passes.flatMap((pass) => pass.pieces.map((piece) => ({ piece, pass })));
  }
  get selectedPieces() {
    const ids = new Set(this.selected);
    return this.allPieces.filter(({ piece, pass }) => ids.has(piece.uid) && !pass.locked);
  }
  get canUndo() {
    void this.historyVersion;
    return this.history.canUndo;
  }
  get canRedo() {
    void this.historyVersion;
    return this.history.canRedo;
  }
  notify(message: string) {
    this.toast = message;
    clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => (this.toast = ''), 3200);
  }
  commit(change: (draft: PressDocument) => void) {
    const draft = structuredClone(this.doc);
    change(draft);
    if (JSON.stringify(draft) === JSON.stringify(this.doc)) return;
    this.history.push(this.doc);
    this.doc = draft;
    this.historyVersion++;
    this.dirty = true;
    this.pruneSelection();
  }
  pruneSelection() {
    const valid = new Set(
      this.allPieces.filter((p) => p.pass.visible && !p.pass.locked).map((p) => p.piece.uid)
    );
    this.selected = this.selected.filter((id) => valid.has(id));
    if (!this.doc.passes.some((p) => p.id === this.activePassId))
      this.activePassId = this.doc.passes[0].id;
  }
  select(id: string, additive = false) {
    const row = this.allPieces.find((p) => p.piece.uid === id);
    if (!row || row.pass.locked || !row.pass.visible) return;
    this.activePassId = row.pass.id;
    this.selected = additive
      ? this.selected.includes(id)
        ? this.selected.filter((p) => p !== id)
        : [...this.selected, id]
      : [id];
  }
  choosePiece(id: string) {
    const p = getPiece(id);
    if (!p || (this.doc.options.physical && !isPhysical(p))) return;
    this.activePieceId = id;
    this.placementRotation = p.allowedRotations[0];
    this.tool = 'place';
    this.mode = 'design';
    this.selected = [];
  }
  ghosts(x: number, y: number, pieceId = this.activePieceId) {
    const p = getPiece(pieceId);
    if (!p) return [];
    return symmetryPlacements(
      {
        uid: 'ghost',
        pieceId,
        x,
        y,
        rotation: p.allowedRotations.includes(this.placementRotation)
          ? this.placementRotation
          : p.allowedRotations[0],
        seed: 1
      },
      this.doc
    );
  }
  place(x: number, y: number, pieceId = this.activePieceId) {
    const catalogPiece = getPiece(pieceId);
    if (!catalogPiece || (this.doc.options.physical && !isPhysical(catalogPiece))) return;
    if (this.activePass.locked || !this.activePass.visible) {
      this.notify('Choose a visible, unlocked ink pass to place pieces.');
      return;
    }
    const candidates = this.ghosts(x, y, pieceId).map((p) => ({
      ...p,
      uid: newId(),
      seed: newSeed()
    }));
    if (!canPlace(this.doc, candidates)) {
      this.notify('That placement overlaps a piece or leaves the artboard.');
      return;
    }
    const passId = this.activePass.id;
    this.commit((doc) => doc.passes.find((p) => p.id === passId)!.pieces.push(...candidates));
  }
  transformSelected(transform: (piece: PlacedPiece) => PlacedPiece) {
    const candidates = this.selectedPieces.map(({ piece }) => transform(piece));
    if (!candidates.length) return;
    if (!canPlace(this.doc, candidates, new Set(this.selected))) {
      this.notify('The transform needs more space on the grid.');
      return;
    }
    const updates = new Map(candidates.map((p) => [p.uid, p]));
    this.commit((doc) => {
      for (const pass of doc.passes) pass.pieces = pass.pieces.map((p) => updates.get(p.uid) ?? p);
    });
  }
  move(dx: number, dy: number) {
    this.transformSelected((p) => ({ ...p, x: p.x + dx, y: p.y + dy }));
  }
  rotate() {
    if (this.tool === 'place' && !this.selected.length) {
      const rotations = getPiece(this.activePieceId)!.allowedRotations;
      this.placementRotation =
        rotations[(rotations.indexOf(this.placementRotation) + 1) % rotations.length];
    } else this.transformSelected(nextRotation);
  }
  remove() {
    const ids = new Set(this.selectedPieces.map((p) => p.piece.uid));
    this.commit((doc) => {
      for (const pass of doc.passes) pass.pieces = pass.pieces.filter((p) => !ids.has(p.uid));
    });
    this.selected = [];
  }
  copy() {
    this.clipboard = this.selectedPieces.map(({ piece, pass }) => ({
      piece: { ...piece },
      color: pass.color,
      name: pass.name
    }));
    if (this.clipboard.length)
      this.notify(
        `Copied ${this.clipboard.length} piece${this.clipboard.length === 1 ? '' : 's'}.`
      );
  }
  duplicate() {
    const selected = this.selectedPieces.map(({ piece, pass }) => ({
      piece,
      color: pass.color,
      name: pass.name
    }));
    this.insertCopy(selected);
  }
  paste() {
    this.insertCopy(this.clipboard);
  }
  private insertCopy(source: ClipboardPiece[]) {
    if (!source.length) return;
    if (
      this.doc.options.physical &&
      source.some(({ piece }) => !isPhysical(getPiece(piece.pieceId)!))
    ) {
      this.notify(
        'The clipboard contains a non-standard-height piece. Disable physical mode to paste it.'
      );
      return;
    }
    let candidates: PlacedPiece[] = [];
    // Search in rings: keep relative positions and ink passes intact.
    outer: for (
      let distance = 1;
      distance <= Math.max(this.doc.board.width, this.doc.board.height);
      distance++
    ) {
      for (let dx = -distance; dx <= distance; dx++)
        for (let dy = -distance; dy <= distance; dy++) {
          if (Math.max(Math.abs(dx), Math.abs(dy)) !== distance) continue;
          const group = source.map(({ piece }) => ({
            ...piece,
            uid: newId(),
            seed: newSeed(),
            x: piece.x + dx,
            y: piece.y + dy
          }));
          if (canPlace(this.doc, group)) {
            candidates = group;
            break outer;
          }
        }
    }
    if (!candidates.length) {
      this.notify('There is no room for this group on the artboard.');
      return;
    }
    this.commit((doc) => {
      candidates.forEach((piece, i) => {
        let pass = doc.passes.find((p) => p.color === source[i].color && !p.locked && p.visible);
        if (!pass) {
          pass = makePass(source[i].color, `Ink Pass ${doc.passes.length + 1}`);
          doc.passes.push(pass);
        }
        pass.pieces.push(piece);
      });
    });
    this.selected = candidates.map((p) => p.uid);
    this.tool = 'select';
  }
  setColor(color: string) {
    if (!/^#[\da-f]{6}$/i.test(color)) {
      this.notify('Use a six-digit HEX ink color, such as #315EAE.');
      return;
    }
    color = color.toLowerCase();
    if (this.selectedPieces.length) {
      let target = this.doc.passes.find(
        (p) => p.color.toLowerCase() === color && !p.locked && p.visible
      );
      const targetId = target?.id ?? newId();
      const ids = new Set(this.selectedPieces.map((p) => p.piece.uid));
      this.commit((doc) => {
        const moved: PlacedPiece[] = [];
        for (const pass of doc.passes) {
          moved.push(...pass.pieces.filter((p) => ids.has(p.uid)));
          pass.pieces = pass.pieces.filter((p) => !ids.has(p.uid));
        }
        let pass = doc.passes.find((p) => p.id === targetId);
        if (!pass) {
          pass = { ...makePass(color, `Ink Pass ${doc.passes.length + 1}`), id: targetId };
          doc.passes.push(pass);
        }
        pass.pieces.push(...moved);
      });
      this.activePassId = targetId;
    } else {
      const matching = this.doc.passes.find(
        (p) => p.color.toLowerCase() === color && !p.locked && p.visible
      );
      if (matching) this.activePassId = matching.id;
      else this.addPass(color);
    }
  }
  addPass(color = '#315eae') {
    const pass = makePass(color, `Ink Pass ${this.doc.passes.length + 1}`);
    this.commit((doc) => doc.passes.push(pass));
    this.activePassId = pass.id;
  }
  updatePass(id: string, update: Partial<Omit<InkPass, 'id' | 'pieces'>>) {
    this.commit((doc) =>
      Object.assign(
        doc.passes.find((p) => p.id === id)!,
        update
      )
    );
  }
  moveToPass(id: string) {
    const target = this.doc.passes.find((p) => p.id === id);
    if (!target || target.locked || !target.visible) {
      this.notify('Choose a visible, unlocked pass.');
      return;
    }
    const ids = new Set(this.selectedPieces.map((p) => p.piece.uid));
    this.commit((doc) => {
      const moved: PlacedPiece[] = [];
      for (const pass of doc.passes) {
        moved.push(...pass.pieces.filter((p) => ids.has(p.uid)));
        pass.pieces = pass.pieces.filter((p) => !ids.has(p.uid));
      }
      doc.passes.find((p) => p.id === id)!.pieces.push(...moved);
    });
    this.activePassId = id;
  }
  reorderPass(id: string, direction: number) {
    this.commit((doc) => {
      const index = doc.passes.findIndex((p) => p.id === id),
        next = index + direction;
      if (next >= 0 && next < doc.passes.length)
        [doc.passes[index], doc.passes[next]] = [doc.passes[next], doc.passes[index]];
    });
  }
  removePass(id: string) {
    if (this.doc.passes.length === 1) {
      this.notify('Keep at least one ink pass.');
      return;
    }
    const pass = this.doc.passes.find((p) => p.id === id)!;
    if (
      pass.pieces.length &&
      !window.confirm(`Delete “${pass.name}” and its ${pass.pieces.length} pieces?`)
    )
      return;
    this.commit((doc) => (doc.passes = doc.passes.filter((p) => p.id !== id)));
  }
  reorderSelected(direction: number) {
    const ids = new Set(this.selected);
    this.commit((doc) => {
      for (const pass of doc.passes) {
        if (direction > 0) {
          for (let i = pass.pieces.length - 2; i >= 0; i--)
            if (ids.has(pass.pieces[i].uid) && !ids.has(pass.pieces[i + 1].uid))
              [pass.pieces[i], pass.pieces[i + 1]] = [pass.pieces[i + 1], pass.pieces[i]];
        } else {
          for (let i = 1; i < pass.pieces.length; i++)
            if (ids.has(pass.pieces[i].uid) && !ids.has(pass.pieces[i - 1].uid))
              [pass.pieces[i], pass.pieces[i - 1]] = [pass.pieces[i - 1], pass.pieces[i]];
        }
      }
    });
  }
  resize(width: number, height: number) {
    if (![width, height].every((n) => Number.isInteger(n) && n >= 1 && n <= 128)) {
      this.notify('Use whole stud dimensions from 1 to 128.');
      return;
    }
    if (
      this.allPieces.some(({ piece }) => {
        const b = bounds(piece);
        return b.x + b.width > width || b.y + b.height > height;
      })
    ) {
      this.notify('Move or clear the pieces beyond the new board size first.');
      return;
    }
    this.commit((doc) => (doc.board = { width, height }));
    this.setZoom('fit');
  }
  setPhysical(physical: boolean) {
    if (physical && this.allPieces.some(({ piece }) => !isPhysical(getPiece(piece.pieceId)!))) {
      this.notify('Remove the non-standard-height pieces before enabling physical print mode.');
      return;
    }
    this.commit((doc) => (doc.options.physical = physical));
    if (physical && !isPhysical(getPiece(this.activePieceId)!)) this.choosePiece('3070');
  }
  clear() {
    if (
      this.allPieces.length &&
      !window.confirm('Clear every piece from the canvas? You can undo this.')
    )
      return;
    this.commit((doc) => {
      for (const pass of doc.passes) pass.pieces = [];
    });
    this.selected = [];
  }
  newDocument() {
    if (
      this.dirty &&
      !window.confirm(
        'Start a new document? Save the current project first if you want to keep it.'
      )
    )
      return;
    this.doc = createDocument();
    this.history.clear();
    this.historyVersion++;
    this.selected = [];
    this.activePassId = this.doc.passes[0].id;
    this.dirty = false;
    this.setZoom('fit');
    this.tool = 'select';
  }
  load(text: string, recovered = false) {
    const doc = parseDocument(text);
    this.doc = doc;
    this.history.clear();
    this.historyVersion++;
    this.selected = [];
    this.activePassId = doc.passes[0].id;
    this.dirty = recovered;
    this.setZoom('fit');
    this.notify(recovered ? 'Recovered your last session.' : 'Project opened.');
  }
  saved() {
    this.dirty = false;
    this.notify('Editable project saved.');
  }
  projectText() {
    return serializeDocument(this.doc);
  }
  undo() {
    const previous = this.history.undo(this.doc);
    if (previous) {
      this.doc = previous;
      this.historyVersion++;
      this.dirty = true;
      this.pruneSelection();
    }
  }
  redo() {
    const next = this.history.redo(this.doc);
    if (next) {
      this.doc = next;
      this.historyVersion++;
      this.dirty = true;
      this.pruneSelection();
    }
  }
  reseed() {
    this.commit((doc) => {
      for (const pass of doc.passes) {
        pass.registrationSeed = newSeed();
        for (const p of pass.pieces) p.seed = newSeed();
      }
      doc.paper.seed = newSeed();
    });
  }
  setZoom(value: number | 'fit') {
    this.zoomCommand = { value, token: this.zoomCommand.token + 1 };
  }
  destroy() {
    clearTimeout(this.toastTimer);
  }
}
