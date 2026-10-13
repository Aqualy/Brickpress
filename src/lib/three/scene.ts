import * as THREE from 'three';
import { bounds } from '../geometry/geometry';
import type { PressDocument, PlacedPiece } from '../types/document';
import type { TraceOverlay } from '../persistence/tracing';
import type {
  CameraAction,
  CameraState,
  CameraProjection,
  Point,
  PreviewRow,
  ScreenRect
} from './types';
import { geometryPose, makePieceGeometry, pieceHeight, pieceMatrix } from './geometry';
import { frameBoardCamera, type BoardCamera } from './camera';

type Batch = { mesh: THREE.InstancedMesh; rows: PreviewRow[] };
export class BoardScene {
  private renderer: THREE.WebGLRenderer;
  private scene = new THREE.Scene();
  private camera: BoardCamera = new THREE.PerspectiveCamera(40, 1, 0.1, 2000);
  private ray = new THREE.Raycaster();
  private geometry = new Map<string, THREE.BufferGeometry>();
  private batches = new Map<string, Batch>();
  private ghosts = new Map<string, Batch>();
  private material = new THREE.MeshStandardMaterial({ roughness: 0.4, metalness: 0 });
  private ghostMaterial = new THREE.MeshStandardMaterial({
    roughness: 0.5,
    transparent: true,
    opacity: 0.45,
    depthWrite: false
  });
  private plateMaterial = new THREE.MeshStandardMaterial({ color: '#e9e5db', roughness: 0.6 });
  private lineMaterial = new THREE.LineDashedMaterial({
    color: '#0057e6',
    dashSize: 0.12,
    gapSize: 0.07,
    depthTest: false
  });
  private selection = new THREE.Group();
  private plate = new THREE.Group();
  private grid?: THREE.LineSegments;
  private traceMesh?: THREE.Mesh<THREE.PlaneGeometry, THREE.MeshBasicMaterial>;
  private traceUrl = '';
  private traceRequest = 0;
  private width = 1;
  private height = 1;
  private board = { width: 0, height: 0 };
  private doc?: PressDocument;
  private delta: Point = { x: 0, y: 0 };
  private selected = new Set<string>();
  private frame = 0;
  private disposed = false;
  private zoom: number | 'fit' = 'fit';
  private effectiveZoom = 1;
  private state: CameraState;
  private light: THREE.DirectionalLight;

  constructor(
    private canvas: HTMLCanvasElement,
    state: CameraState | null,
    private onCamera: (state: CameraState, zoom: number) => void,
    private onError: (message: string) => void
  ) {
    this.state = state
      ? structuredClone(state)
      : { azimuth: Math.PI / 4, elevation: Math.PI / 3, target: [0, 0, 0] };
    this.state.projection ??= 'perspective';
    if (this.state.projection === 'orthographic')
      this.camera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 2000);
    this.renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 2));
    // The editor workspace follows the system appearance; artwork colors stay fixed.
    this.renderer.setClearColor(0, 0);
    this.renderer.shadowMap.enabled = true;
    this.renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    this.renderer.shadowMap.autoUpdate = false;
    this.scene.add(new THREE.HemisphereLight('#ffffff', '#a9a2a0', 2.5));
    this.light = new THREE.DirectionalLight('#ffffff', 3);
    this.light.position.set(-12, 30, 14);
    this.light.castShadow = true;
    this.light.shadow.mapSize.set(2048, 2048);
    this.light.shadow.normalBias = 0.035;
    this.scene.add(this.light, this.light.target, this.plate, this.selection);
    canvas.addEventListener('webglcontextlost', this.contextLost);
  }
  private contextLost = (event: Event) => {
    event.preventDefault();
    this.onError(
      'The 3D graphics context was lost. Return to Design and reopen 3D to retry; your work is safe.'
    );
  };
  private invalidate() {
    if (this.frame || this.disposed) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      if (this.disposed) return;
      try {
        this.renderer.render(this.scene, this.camera);
      } catch {
        this.onError('3D rendering is unavailable. Return to Design to continue editing.');
      }
    });
  }
  resize(width: number, height: number) {
    this.width = Math.max(1, width);
    this.height = Math.max(1, height);
    this.renderer.setSize(this.width, this.height, false);
    this.updateCamera(true);
  }
  private updateCamera(refit = false) {
    // Fit only on an explicit framing change. Refitting while orbiting makes the
    // same pieces grow and shrink as the board's projected bounds change.
    this.effectiveZoom = frameBoardCamera(
      this.camera,
      this.state,
      this.board,
      this.width,
      this.height,
      this.zoom === 'fit' && !refit ? this.effectiveZoom * 100 : this.zoom
    );
    this.onCamera(structuredClone(this.state), Math.round(this.effectiveZoom * 100));
    this.invalidate();
  }
  setProjection(projection: CameraProjection) {
    if (this.state.projection === projection) return;
    this.state.projection = projection;
    this.camera =
      projection === 'perspective'
        ? new THREE.PerspectiveCamera(40, 1, 0.1, 2000)
        : new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 2000);
    this.updateCamera(true);
  }
  setZoom(value: number | 'fit', resetPan = true) {
    this.zoom = value;
    if (value === 'fit' && resetPan) this.state.target = [0, 0, 0];
    this.updateCamera(true);
  }
  orbit(dx: number, dy: number) {
    this.state.azimuth -= dx * 0.008;
    this.state.elevation = Math.max(
      0.17,
      Math.min(Math.PI / 2 - 0.0001, this.state.elevation + dy * 0.008)
    );
    this.updateCamera();
  }
  pan(dx: number, dy: number) {
    const scale = 1 / (32 * this.effectiveZoom);
    const right = new THREE.Vector3().setFromMatrixColumn(this.camera.matrixWorld, 0);
    const up = new THREE.Vector3().setFromMatrixColumn(this.camera.matrixWorld, 1);
    const offset = right.multiplyScalar(-dx * scale).addScaledVector(up, dy * scale);
    this.state.target = this.state.target.map((v, i) => v + offset.getComponent(i)) as [
      number,
      number,
      number
    ];
    this.updateCamera();
  }
  cameraAction(action: CameraAction) {
    if (action === 'top' || action === 'isometric' || action === 'reset') {
      this.state = {
        projection: this.state.projection,
        azimuth: action === 'top' ? 0 : Math.PI / 4,
        elevation: action === 'top' ? Math.PI / 2 - 0.0001 : Math.PI / 3,
        target: [0, 0, 0]
      };
      this.zoom = 'fit';
      this.updateCamera(true);
    } else if (action.startsWith('pan-'))
      this.pan(
        action === 'pan-left' ? 32 : action === 'pan-right' ? -32 : 0,
        action === 'pan-up' ? 32 : action === 'pan-down' ? -32 : 0
      );
    else
      this.orbit(
        action === 'left' ? 24 : action === 'right' ? -24 : 0,
        action === 'up' ? 16 : action === 'down' ? -16 : 0
      );
  }
  private geometryFor(p: PlacedPiece) {
    const key = geometryPose(p).key;
    if (!this.geometry.has(key)) this.geometry.set(key, makePieceGeometry(p));
    return this.geometry.get(key)!;
  }
  private syncBatches(target: Map<string, Batch>, rows: PreviewRow[], preview = false) {
    const grouped = new Map<string, PreviewRow[]>();
    for (const row of rows) {
      const key = geometryPose(row.piece).key;
      if (!grouped.has(key)) grouped.set(key, []);
      grouped.get(key)!.push(row);
    }
    for (const [key, batch] of target)
      if (!grouped.has(key)) {
        this.scene.remove(batch.mesh);
        batch.mesh.dispose();
        target.delete(key);
      }
    for (const [key, list] of grouped) {
      let batch = target.get(key);
      if (!batch || batch.mesh.instanceMatrix.count < list.length) {
        if (batch) {
          this.scene.remove(batch.mesh);
          batch.mesh.dispose();
        }
        const mesh = new THREE.InstancedMesh(
          this.geometryFor(list[0].piece),
          preview ? this.ghostMaterial : this.material,
          2 ** Math.ceil(Math.log2(Math.max(1, list.length)))
        );
        mesh.castShadow = !preview;
        mesh.receiveShadow = !preview;
        batch = { mesh, rows: list };
        target.set(key, batch);
        this.scene.add(mesh);
      }
      batch.rows = list;
      batch.mesh.count = list.length;
      for (let i = 0; i < list.length; i++)
        batch.mesh.setColorAt(i, new THREE.Color(list[i].color));
      if (batch.mesh.instanceColor) batch.mesh.instanceColor.needsUpdate = true;
    }
    this.updateMatrices(target, preview);
  }
  private updateMatrices(target: Map<string, Batch>, preview = false) {
    const order = new Map(
      this.doc?.passes.flatMap((pass) => pass.pieces).map((p, i) => [p.uid, i])
    );
    for (const batch of target.values()) {
      batch.rows.forEach(({ piece }, i) => {
        const p =
          !preview && this.selected.has(piece.uid)
            ? { ...piece, x: piece.x + this.delta.x, y: piece.y + this.delta.y }
            : piece;
        batch.mesh.setMatrixAt(
          i,
          pieceMatrix(p, this.board, preview ? 10001 : order.get(piece.uid))
        );
      });
      batch.mesh.instanceMatrix.needsUpdate = true;
      batch.mesh.computeBoundingSphere();
    }
    this.scene.updateMatrixWorld(true);
  }
  syncDocument(doc: PressDocument) {
    this.doc = doc;
    if (doc.board.width !== this.board.width || doc.board.height !== this.board.height) {
      if (this.board.width) this.state.target = [0, 0, 0];
      this.board = { ...doc.board };
      this.buildPlate();
      this.updateCamera(true);
    }
    this.syncBatches(
      this.batches,
      doc.passes
        .filter((pass) => pass.visible)
        .flatMap((pass) => pass.pieces.map((piece) => ({ piece, color: pass.color })))
    );
    this.buildGrid(doc);
    this.renderer.shadowMap.needsUpdate = true;
    this.invalidate();
  }
  private buildPlate() {
    for (const child of [...this.plate.children]) {
      if (child instanceof THREE.Mesh) {
        child.geometry.dispose();
        if (child instanceof THREE.InstancedMesh) child.dispose();
      }
      this.plate.remove(child);
    }
    const { width, height } = this.board;
    const base = new THREE.Mesh(new THREE.BoxGeometry(width, 0.18, height), this.plateMaterial);
    base.position.y = -0.09;
    base.receiveShadow = true;
    this.plate.add(base);
    const studs = new THREE.InstancedMesh(
      new THREE.CylinderGeometry(0.29, 0.3, 0.18, 20),
      this.plateMaterial,
      width * height
    );
    const matrix = new THREE.Matrix4();
    for (let y = 0; y < height; y++)
      for (let x = 0; x < width; x++)
        studs.setMatrixAt(
          y * width + x,
          matrix.makeTranslation(x + 0.5 - width / 2, 0.09, y + 0.5 - height / 2)
        );
    studs.receiveShadow = true;
    this.plate.add(studs);
    const size = Math.max(width, height);
    Object.assign(this.light.shadow.camera, {
      left: -size,
      right: size,
      top: size,
      bottom: -size,
      far: size * 8
    });
    this.light.shadow.camera.updateProjectionMatrix();
    this.light.position.set(-size, size * 2, size);
    this.light.target.position.set(0, 0, 0);
  }
  private gridKey = '';
  private buildGrid(doc: PressDocument) {
    const key = `${this.board.width}:${this.board.height}:${doc.options.grid}:${doc.options.majorGrid}:${doc.options.symmetry}`;
    if (key === this.gridKey) return;
    this.gridKey = key;
    if (this.grid) {
      this.scene.remove(this.grid);
      this.grid.geometry.dispose();
      (this.grid.material as THREE.Material).dispose();
    }
    const points: number[] = [],
      { width: w, height: h } = this.board;
    const line = (x: number, y: number, x2: number, y2: number) =>
      points.push(x - w / 2, 0.185, y - h / 2, x2 - w / 2, 0.185, y2 - h / 2);
    if (doc.options.grid === 'lines' || doc.options.grid === 'squares') {
      for (let x = 0; x <= w; x++) line(x, 0, x, h);
      for (let y = 0; y <= h; y++) line(0, y, w, y);
    }
    if (doc.options.symmetry === 'x' || doc.options.symmetry === 'both') line(w / 2, 0, w / 2, h);
    if (doc.options.symmetry === 'y' || doc.options.symmetry === 'both') line(0, h / 2, w, h / 2);
    this.grid = new THREE.LineSegments(
      new THREE.BufferGeometry().setAttribute(
        'position',
        new THREE.Float32BufferAttribute(points, 3)
      ),
      new THREE.LineBasicMaterial({ color: '#6b7789', transparent: true, opacity: 0.3 })
    );
    this.scene.add(this.grid);
  }
  syncPreview(
    selected: string[],
    delta: Point,
    valid: boolean,
    ghosts: PreviewRow[],
    ghostValid: boolean
  ) {
    this.selected = new Set(selected);
    this.delta = delta;
    this.updateMatrices(this.batches);
    for (const child of [...this.selection.children]) {
      (child as THREE.Line).geometry.dispose();
      this.selection.remove(child);
    }
    this.lineMaterial.color.set(valid ? '#0057e6' : '#b42318');
    for (const pass of this.doc?.passes ?? [])
      if (pass.visible && !pass.locked)
        for (const p of pass.pieces)
          if (this.selected.has(p.uid)) {
            const b = bounds(p),
              x = b.x + delta.x - this.board.width / 2,
              z = b.y + delta.y - this.board.height / 2;
            const y = pieceHeight(p.pieceId) + 0.035;
            const points = [
              [x, z],
              [x + b.width, z],
              [x + b.width, z + b.height],
              [x, z + b.height],
              [x, z]
            ].map(([x, z]) => new THREE.Vector3(x, y, z));
            const outline = new THREE.Line(
              new THREE.BufferGeometry().setFromPoints(points),
              this.lineMaterial
            );
            outline.computeLineDistances();
            outline.renderOrder = 5;
            this.selection.add(outline);
          }
    this.syncBatches(
      this.ghosts,
      ghosts.map((row) => ({ ...row, color: ghostValid ? row.color : '#b42318' })),
      true
    );
    this.renderer.shadowMap.needsUpdate = true;
    this.invalidate();
  }
  syncTrace(trace: TraceOverlay | null) {
    this.latestTrace = trace;
    if (!trace) {
      this.traceRequest++;
      this.traceUrl = '';
      if (this.traceMesh) {
        this.scene.remove(this.traceMesh);
        this.traceMesh.geometry.dispose();
        this.traceMesh.material.map?.dispose();
        this.traceMesh.material.dispose();
        this.traceMesh = undefined;
      }
    }
    if (!trace?.visible) {
      if (this.traceMesh) this.traceMesh.visible = false;
      return this.invalidate();
    }
    if (trace.url !== this.traceUrl) {
      this.traceUrl = trace.url;
      const request = ++this.traceRequest;
      const texture = new THREE.TextureLoader().load(
        trace.url,
        () => {
          if (this.disposed || request !== this.traceRequest) {
            texture.dispose();
            return;
          }
          texture.colorSpace = THREE.SRGBColorSpace;
          if (this.traceMesh) {
            this.scene.remove(this.traceMesh);
            this.traceMesh.geometry.dispose();
            this.traceMesh.material.map?.dispose();
            this.traceMesh.material.dispose();
          }
          this.traceMesh = new THREE.Mesh(
            new THREE.PlaneGeometry(1, 1),
            new THREE.MeshBasicMaterial({ map: texture, transparent: true, depthWrite: false })
          );
          this.traceMesh.rotation.x = -Math.PI / 2;
          this.scene.add(this.traceMesh);
          this.syncTrace(this.latestTrace);
        },
        undefined,
        () => {
          texture.dispose();
        }
      );
    }
    if (this.traceMesh) {
      this.traceMesh.visible = true;
      this.traceMesh.scale.set(trace.width, trace.height, 1);
      this.traceMesh.position.set(
        trace.x + trace.width / 2 - this.board.width / 2,
        0.19,
        trace.y + trace.height / 2 - this.board.height / 2
      );
      this.traceMesh.material.opacity = trace.opacity;
    }
    this.invalidate();
  }
  private latestTrace: TraceOverlay | null = null;
  diagnostics() {
    return {
      draws: this.renderer.info.render.calls,
      geometries: this.renderer.info.memory.geometries,
      textures: this.renderer.info.memory.textures,
      frames: this.renderer.info.render.frame
    };
  }
  private setRay(clientX: number, clientY: number) {
    const rect = this.canvas.getBoundingClientRect();
    this.ray.setFromCamera(
      new THREE.Vector2(
        ((clientX - rect.left) / rect.width) * 2 - 1,
        1 - ((clientY - rect.top) / rect.height) * 2
      ),
      this.camera
    );
  }
  point(clientX: number, clientY: number, height = 0.4): Point | null {
    this.setRay(clientX, clientY);
    const hit = this.ray.ray.intersectPlane(
      new THREE.Plane(new THREE.Vector3(0, 1, 0), -height),
      new THREE.Vector3()
    );
    return hit ? { x: hit.x + this.board.width / 2, y: hit.z + this.board.height / 2 } : null;
  }
  pick(clientX: number, clientY: number) {
    this.setRay(clientX, clientY);
    const hits = this.ray.intersectObjects(
      [...this.batches.values()].map((b) => b.mesh),
      false
    );
    for (const hit of hits)
      for (const batch of this.batches.values())
        if (batch.mesh === hit.object && hit.instanceId !== undefined)
          return batch.rows[hit.instanceId].piece.uid;
    return null;
  }
  project(x: number, y: number, height = 0.4): Point {
    const p = new THREE.Vector3(
      x - this.board.width / 2,
      height,
      y - this.board.height / 2
    ).project(this.camera);
    return { x: ((p.x + 1) * this.width) / 2, y: ((1 - p.y) * this.height) / 2 };
  }
  marquee(rect: ScreenRect) {
    return (
      this.doc?.passes
        .filter((p) => p.visible && !p.locked)
        .flatMap((pass) =>
          pass.pieces
            .filter((p) => {
              const b = bounds(p),
                points = [
                  this.project(b.x, b.y, pieceHeight(p.pieceId)),
                  this.project(b.x + b.width, b.y, pieceHeight(p.pieceId)),
                  this.project(b.x, b.y + b.height, pieceHeight(p.pieceId)),
                  this.project(b.x + b.width, b.y + b.height, pieceHeight(p.pieceId))
                ];
              return (
                Math.min(...points.map((p) => p.x)) < rect.x + rect.width &&
                Math.max(...points.map((p) => p.x)) > rect.x &&
                Math.min(...points.map((p) => p.y)) < rect.y + rect.height &&
                Math.max(...points.map((p) => p.y)) > rect.y
              );
            })
            .map((p) => p.uid)
        ) ?? []
    );
  }
  dispose() {
    this.disposed = true;
    this.traceRequest++;
    cancelAnimationFrame(this.frame);
    this.canvas.removeEventListener('webglcontextlost', this.contextLost);
    for (const batch of [...this.batches.values(), ...this.ghosts.values()]) batch.mesh.dispose();
    for (const geometry of this.geometry.values()) geometry.dispose();
    for (const child of this.plate.children)
      if (child instanceof THREE.Mesh) {
        child.geometry.dispose();
        if (child instanceof THREE.InstancedMesh) child.dispose();
      }
    for (const child of this.selection.children) (child as THREE.Line).geometry.dispose();
    this.grid?.geometry.dispose();
    (this.grid?.material as THREE.Material | undefined)?.dispose();
    this.traceMesh?.geometry.dispose();
    this.traceMesh?.material.map?.dispose();
    this.traceMesh?.material.dispose();
    this.material.dispose();
    this.ghostMaterial.dispose();
    this.plateMaterial.dispose();
    this.lineMaterial.dispose();
    this.light.shadow.dispose();
    this.renderer.dispose();
    this.renderer.forceContextLoss();
  }
}
