import { getPiece, isPhysical } from '../catalog/catalog';
import {
  bounds,
  canPlace,
  createPlacementValidator,
  nextRotation,
  symmetryPlacements
} from '../geometry/geometry';
import { History } from '../history/history';
import {
  createDocument,
  makePass,
  parseDocument,
  serializeDocument
} from '../persistence/document';
import { newId, newSeed } from '../printing/noise';
import {
  capturePreset,
  parsePresets,
  presetRows,
  type CompositionPreset
} from '../persistence/presets';
import { readTraceFile, type TraceImage, type TraceOverlay } from '../persistence/tracing';
import type { InkPass, PlacedPiece, PressDocument } from '../types/document';
import { platformError, type FileKind, type PlatformAdapter } from '../platform/types';

type ClipboardPiece = { piece: PlacedPiece; color: string; name: string };
export class Editor {
  doc = $state.raw<PressDocument>(createDocument(true));
  selected = $state.raw<string[]>([]);
  activePassId = $state(this.doc.passes[0].id);
  activePieceId = $state('25269');
  paletteTab = $state<'pieces' | 'presets'>('pieces');
  presets = $state.raw<CompositionPreset[]>([]);
  activePresetId = $state<string | null>(null);
  presetRotation = $state(0);
  trace = $state.raw<TraceOverlay | null>(null);
  traceLoading = $state(false);
  private traceRequest = 0;
  placementRotation = $state(0);
  placementMirrorX = $state(false);
  placementMirrorY = $state(false);
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
  desktop = $state(false);
  documentBusy = $state(false);
  transitioning = $state(false);
  activeFilePath = $state<string | null>(null);
  private platform?: PlatformAdapter;
  private presetWrites: Promise<unknown> = Promise.resolve();
  private assetOperations = new Set<Promise<unknown>>();
  private trackAsset<T>(operation: () => Promise<T>) {
    const pending = operation();
    this.assetOperations.add(pending);
    const finish = () => this.assetOperations.delete(pending);
    void pending.then(finish, finish);
    return pending;
  }
  async flushAssets() {
    while (this.assetOperations.size) await Promise.all([...this.assetOperations]);
  }
  setPlatform(platform: PlatformAdapter) {
    this.platform = platform;
    this.desktop = platform.desktop;
  }
  async confirmAction(message: string) {
    try {
      return this.platform ? await this.platform.confirmAction(message) : false;
    } catch (error) {
      this.notify(platformError(error).message);
      return false;
    }
  }
  async flushPresets() {
    await this.presetWrites;
  }
  async exportBlob(kind: Exclude<FileKind, 'trace'>, blob: Blob, name: string) {
    if (!this.platform) throw new Error('Storage is still loading.');
    const result = await this.platform.exportFile(kind, blob, name);
    if (result.status === 'error') throw new Error(result.error.message);
    return result.status === 'success';
  }
  chooseTracingImage() {
    return this.trackAsset(() => this.readSelectedTracingImage());
  }
  private async readSelectedTracingImage() {
    const request = ++this.traceRequest;
    this.traceLoading = true;
    try {
      if (!this.platform) return;
      const result = await this.platform.chooseFile('trace');
      if (result.status === 'error') throw result.error;
      if (result.status !== 'success') return;
      try {
        if (request !== this.traceRequest) return;
        await this.uploadTrace(result.value.file);
      } finally {
        if (result.value.selected) await this.platform.releaseFile(result.value.selected);
      }
    } catch (error) {
      if (request === this.traceRequest) this.notify(platformError(error).message);
    } finally {
      if (request === this.traceRequest) this.traceLoading = false;
    }
  }
  choosePresetFile() {
    return this.trackAsset(() => this.readSelectedPresetFile());
  }
  private async readSelectedPresetFile() {
    try {
      if (!this.platform) return;
      const result = await this.platform.chooseFile('presets');
      if (result.status === 'error') throw result.error;
      if (result.status !== 'success') return;
      try {
        await this.importPresets(await result.value.file.text());
      } finally {
        if (result.value.selected) await this.platform.releaseFile(result.value.selected);
      }
    } catch (error) {
      this.notify(platformError(error).message);
    }
  }
  autosaveStatus = $state('Saved on this device');
  historyVersion = $state(0);
  zoomCommand = $state.raw<{ value: number | 'fit'; token: number }>({ value: 'fit', token: 0 });
  zoomPercent = $state(100);
  private history = new History<PressDocument>();
  private placementStroke: {
    before: PressDocument;
    latest: PressDocument;
    dirty: boolean;
    visited: Set<string>;
    validator: ReturnType<typeof createPlacementValidator>;
    count: number;
  } | null = null;
  private clipboard: ClipboardPiece[] = [];
  private toastTimer?: ReturnType<typeof setTimeout>;
  get activePass() {
    return this.doc.passes.find((p) => p.id === this.activePassId) ?? this.doc.passes[0];
  }
  get activePreset() {
    return this.presets.find((p) => p.id === this.activePresetId);
  }
  restorePresets(text: string) {
    this.presets = parsePresets(text);
    if (!this.activePreset) this.activePresetId = null;
  }
  private writePresets(update: () => CompositionPreset[]) {
    const pending = this.presetWrites.then(() => this.persistPresets(update()));
    this.presetWrites = pending.catch(() => undefined);
    return pending;
  }
  private async persistPresets(next: CompositionPreset[]) {
    try {
      if (next.length > 100) throw new Error('The library can hold up to 100 presets.');
      if (!this.platform) throw new Error('Storage is still loading.');
      await this.platform.writeState('presets', JSON.stringify(next));
      this.presets = next;
      return true;
    } catch (e) {
      this.notify(
        e instanceof Error && next.length > 100
          ? e.message
          : 'Preset storage is full or unavailable. Export your presets to keep a backup.'
      );
      return false;
    }
  }
  async savePreset(name: string, selectionOnly = false) {
    try {
      const preset = capturePreset(
        this.doc,
        name,
        selectionOnly ? new Set(this.selected) : undefined
      );
      if (await this.writePresets(() => [...this.presets, preset])) {
        this.paletteTab = 'presets';
        this.notify(`Saved “${preset.name}” to your presets.`);
        return true;
      }
    } catch (e) {
      this.notify(e instanceof Error ? e.message : 'The preset could not be saved.');
    }
    return false;
  }
  async renamePreset(id: string, name: string) {
    name = name.trim();
    if (!name || name.length > 80) {
      this.notify('Use a preset name of 1–80 characters.');
      return;
    }
    await this.writePresets(() => this.presets.map((p) => (p.id === id ? { ...p, name } : p)));
  }
  async deletePreset(id: string) {
    if (
      (await this.writePresets(() => this.presets.filter((p) => p.id !== id))) &&
      this.activePresetId === id
    ) {
      this.activePresetId = null;
      this.tool = 'select';
    }
  }
  async importPresets(text: string) {
    const imported = parsePresets(text);
    if (
      await this.writePresets(() => {
        const next = [...this.presets];
        for (const p of imported) {
          if (next.some((saved) => JSON.stringify(saved) === JSON.stringify(p))) continue;
          next.push({ ...p, id: next.some((saved) => saved.id === p.id) ? newId() : p.id });
        }
        return next;
      })
    )
      this.notify(`Imported ${imported.length} composition presets.`);
  }
  choosePreset(id: string, focus = true) {
    if (!this.presets.some((p) => p.id === id)) return;
    this.activePresetId = id;
    this.presetRotation = 0;
    this.tool = 'place';
    this.mode = 'design';
    this.selected = [];
    if (focus) this.focusCanvas();
  }
  presetPreview(x: number, y: number) {
    return this.activePreset ? presetRows(this.activePreset, x, y, this.presetRotation) : [];
  }
  presetFits(x: number, y: number) {
    const rows = this.presetPreview(x, y);
    return (
      rows.length > 0 &&
      (!this.doc.options.physical ||
        rows.every(({ piece }) => isPhysical(getPiece(piece.pieceId)!))) &&
      canPlace(
        this.doc,
        rows.map((p) => p.piece)
      )
    );
  }
  placePreset(x: number, y: number) {
    const preset = this.activePreset;
    if (!preset) return;
    const rows = this.presetPreview(x, y);
    if (
      this.doc.options.physical &&
      rows.some(({ piece }) => !isPhysical(getPiece(piece.pieceId)!))
    ) {
      this.notify(
        'This preset contains non-standard-height pieces. Disable physical mode to place it.'
      );
      return;
    }
    if (!this.presetFits(x, y)) {
      this.notify(
        'This preset overlaps pieces or does not fit the canvas. Try another position or a larger canvas.'
      );
      return;
    }
    if (
      this.doc.passes.length + preset.passes.length > 100 ||
      this.allPieces.length + rows.length > 10_000
    ) {
      this.notify('Placement would exceed the project limit of 100 ink passes or 10,000 pieces.');
      return;
    }
    const passes = preset.passes.map((p) => makePass(p.color, `${preset.name} · ${p.name}`));
    for (const row of rows)
      passes[row.passIndex].pieces.push({ ...row.piece, uid: newId(), seed: newSeed() });
    this.commit((doc) => doc.passes.push(...passes));
    this.selected = passes.flatMap((p) => p.pieces.map((piece) => piece.uid));
    this.activePassId = passes[0].id;
    this.activePresetId = null;
    this.tool = 'select';
    this.notify(`Placed “${preset.name}”.`);
  }
  restoreTrace(record: TraceImage | null) {
    if (this.trace) URL.revokeObjectURL(this.trace.url);
    this.trace = null;
    if (!record) return;
    this.trace = { ...record, url: URL.createObjectURL(record.asset) };
  }
  uploadTrace(file: File) {
    return this.trackAsset(() => this.decodeTrace(file));
  }
  private async decodeTrace(file: File) {
    const request = ++this.traceRequest;
    this.traceLoading = true;
    try {
      const image = await readTraceFile(file);
      if (request !== this.traceRequest) return;
      if (this.trace) URL.revokeObjectURL(this.trace.url);
      this.trace = {
        ...image,
        url: URL.createObjectURL(image.asset),
        x: 0,
        y: 0,
        width: 1,
        height: 1,
        opacity: 0.4,
        visible: true,
        locked: true
      };
      this.fitTrace();
      this.mode = 'design';
      this.notify('Tracing image added beneath your pieces.');
    } catch (e) {
      if (request === this.traceRequest)
        this.notify(e instanceof Error ? e.message : 'The image could not be loaded.');
    } finally {
      if (request === this.traceRequest) this.traceLoading = false;
    }
  }
  updateTrace(
    update: Partial<Pick<TraceImage, 'x' | 'y' | 'width' | 'opacity' | 'visible' | 'locked'>>
  ) {
    if (!this.trace) return;
    const trace = { ...this.trace, ...update };
    if (update.width !== undefined)
      trace.height = (trace.width * trace.imageHeight) / trace.imageWidth;
    if (
      ![trace.x, trace.y, trace.width, trace.height, trace.opacity].every(Number.isFinite) ||
      trace.width <= 0 ||
      trace.width > 1024 ||
      trace.height > 1024 ||
      Math.abs(trace.x) > 1024 ||
      Math.abs(trace.y) > 1024 ||
      trace.opacity < 0 ||
      trace.opacity > 1
    ) {
      this.notify('Use valid tracing coordinates and a positive width up to 1,024 studs.');
      return;
    }
    this.trace = trace;
  }
  fitTrace() {
    if (!this.trace) return;
    const scale = Math.min(
      this.doc.board.width / this.trace.imageWidth,
      this.doc.board.height / this.trace.imageHeight
    );
    const width = this.trace.imageWidth * scale,
      height = this.trace.imageHeight * scale;
    this.trace = {
      ...this.trace,
      width,
      height,
      x: (this.doc.board.width - width) / 2,
      y: (this.doc.board.height - height) / 2
    };
  }
  centerTrace() {
    if (this.trace)
      this.trace = {
        ...this.trace,
        x: (this.doc.board.width - this.trace.width) / 2,
        y: (this.doc.board.height - this.trace.height) / 2
      };
  }
  removeTrace() {
    this.traceRequest++;
    this.traceLoading = false;
    if (this.trace) URL.revokeObjectURL(this.trace.url);
    this.trace = null;
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
    this.finishPlacementStroke();
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
    this.activePresetId = null;
    this.placementRotation = p.allowedRotations[0];
    this.placementMirrorX = false;
    this.placementMirrorY = false;
    this.tool = 'place';
    this.mode = 'design';
    this.selected = [];
  }
  pickPiece(uid: string) {
    const row = this.allPieces.find(({ piece }) => piece.uid === uid);
    if (!row || !row.pass.visible) return;
    const catalogPiece = getPiece(row.piece.pieceId);
    if (!catalogPiece || (this.doc.options.physical && !isPhysical(catalogPiece))) return;
    // Sampling changes the placement tool only, including reflected silhouettes.
    // Keep the current ink, document, and undo history intact, even on locked passes.
    this.choosePiece(row.piece.pieceId);
    this.placementRotation = row.piece.rotation;
    this.placementMirrorX = !!row.piece.mirrorX;
    this.placementMirrorY = !!row.piece.mirrorY;
    this.notify(`Picked ${catalogPiece.name}.`);
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
        seed: 1,
        ...(this.placementMirrorX && { mirrorX: true }),
        ...(this.placementMirrorY && { mirrorY: true })
      },
      this.doc
    );
  }
  private placementCandidates(
    x: number,
    y: number,
    pieceId: string,
    quiet = false,
    count = this.allPieces.length
  ) {
    const reject = (message: string) => {
      if (!quiet) this.notify(message);
      return null;
    };
    const catalogPiece = getPiece(pieceId);
    if (!catalogPiece || (this.doc.options.physical && !isPhysical(catalogPiece))) return null;
    if (this.activePass.locked || !this.activePass.visible) {
      return reject('Choose a visible, unlocked ink pass to place pieces.');
    }
    const candidates = this.ghosts(x, y, pieceId).map((p) => ({
      ...p,
      uid: newId(),
      seed: newSeed()
    }));
    if (count + candidates.length > 10_000) {
      const message = 'Placement would exceed the project limit of 10,000 pieces.';
      if (this.toast !== message) this.notify(message);
      return null;
    }
    return candidates;
  }
  place(x: number, y: number, pieceId = this.activePieceId) {
    this.finishPlacementStroke();
    if (this.activePresetId) {
      this.placePreset(x, y);
      return;
    }
    const candidates = this.placementCandidates(x, y, pieceId);
    if (!candidates) return;
    if (!canPlace(this.doc, candidates)) {
      this.notify('That placement overlaps a piece or leaves the artboard.');
      return;
    }
    const passId = this.activePass.id;
    this.commit((doc) => doc.passes.find((p) => p.id === passId)!.pieces.push(...candidates));
  }
  beginPlacementStroke() {
    this.finishPlacementStroke();
    if (this.activePresetId || this.mode !== 'design' || this.tool !== 'place') return false;
    this.placementStroke = {
      before: this.doc,
      latest: this.doc,
      dirty: this.dirty,
      visited: new Set(),
      validator: createPlacementValidator(this.doc),
      count: this.allPieces.length
    };
    return true;
  }
  get placementStrokeActive() {
    return this.placementStroke !== null;
  }
  paintPlacement(x: number, y: number) {
    const stroke = this.placementStroke;
    if (
      !stroke ||
      this.doc !== stroke.latest ||
      this.tool !== 'place' ||
      this.mode !== 'design' ||
      this.activePresetId
    )
      return false;
    const key = `${x},${y}`;
    if (stroke.visited.has(key)) return false;
    stroke.visited.add(key);
    const quiet = stroke.visited.size > 1;
    const candidates = this.placementCandidates(x, y, this.activePieceId, quiet, stroke.count);
    if (!candidates) return false;
    if (!stroke.validator.canPlace(candidates)) {
      if (!quiet) this.notify('That placement overlaps a piece or leaves the artboard.');
      return false;
    }
    stroke.validator.add(candidates);
    const passId = this.activePass.id;
    this.doc = {
      ...this.doc,
      passes: this.doc.passes.map((pass) =>
        pass.id === passId ? { ...pass, pieces: [...pass.pieces, ...candidates] } : pass
      )
    };
    stroke.latest = this.doc;
    stroke.count += candidates.length;
    this.dirty = true;
    return true;
  }
  finishPlacementStroke(cancelled = false) {
    const stroke = this.placementStroke;
    this.placementStroke = null;
    if (!stroke || stroke.before === stroke.latest || this.doc !== stroke.latest) return;
    if (cancelled) {
      this.doc = stroke.before;
      this.dirty = stroke.dirty;
      this.pruneSelection();
    } else {
      this.history.push(stroke.before);
      this.historyVersion++;
    }
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
      if (this.activePreset) {
        this.presetRotation = (this.presetRotation + 90) % 360;
        return;
      }
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
  async removePass(id: string) {
    if (this.doc.passes.length === 1) {
      this.notify('Keep at least one ink pass.');
      return;
    }
    const pass = this.doc.passes.find((p) => p.id === id)!;
    if (
      pass.pieces.length &&
      !(await this.confirmAction(`Delete “${pass.name}” and its ${pass.pieces.length} pieces?`))
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
  async clear() {
    if (
      this.allPieces.length &&
      !(await this.confirmAction('Clear every piece from the canvas? You can undo this.'))
    )
      return;
    this.commit((doc) => {
      for (const pass of doc.passes) pass.pieces = [];
    });
    this.selected = [];
  }
  newDocument() {
    this.finishPlacementStroke(true);
    this.removeTrace();
    this.activePresetId = null;
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
    this.finishPlacementStroke(true);
    this.removeTrace();
    this.activePresetId = null;
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
    this.finishPlacementStroke();
    const previous = this.history.undo(this.doc);
    if (previous) {
      this.doc = previous;
      this.historyVersion++;
      this.dirty = true;
      this.pruneSelection();
    }
  }
  redo() {
    this.finishPlacementStroke();
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
    this.finishPlacementStroke(true);
    clearTimeout(this.toastTimer);
    this.removeTrace();
  }
}
