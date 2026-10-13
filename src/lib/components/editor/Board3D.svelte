<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import type { Editor } from '../../stores/editor.svelte';
  import type { BoardScene } from '../../three/scene';
  import type { Point, ScreenRect, CameraAction, CameraProjection } from '../../three/types';
  import { getPiece } from '../../catalog/catalog';
  import { canPlace } from '../../geometry/geometry';
  import { placementPath } from '../../geometry/placement-path';
  import Icon from '../ui/Icon.svelte';
  import BoardSize from '../ui/BoardSize.svelte';
  import ZoomControls from '../ui/ZoomControls.svelte';
  import Popover from '../ui/Popover.svelte';
  let { editor }: { editor: Editor } = $props();
  let viewport: HTMLDivElement;
  let canvas: HTMLCanvasElement;
  let width = $state(800),
    height = $state(700);
  let scene = $state.raw<BoardScene | null>(null);
  let error = $state('');
  let projection = $state<CameraProjection>(
    untrack(() => editor.camera3D?.projection ?? 'perspective')
  );
  let hover = $state.raw<Point | null>(null);
  let delta = $state.raw<Point>({ x: 0, y: 0 });
  let marquee = $state.raw<ScreenRect | null>(null);
  let dragging = $state<'move' | 'box' | 'place' | 'orbit' | 'pan' | null>(null);
  let space = $state(false);
  let dragPointer: number | null = null;
  let origin: Point = { x: 0, y: 0 },
    lastClient = { x: 0, y: 0 },
    startClient = { x: 0, y: 0 },
    paintFrom = { x: 0, y: 0 };
  let dragHeight = 0.4,
    additive = false;
  let selected = $derived(editor.selectedPieces);
  let validMove = $derived(
    dragging !== 'move' ||
      canPlace(
        editor.doc,
        selected.map(({ piece }) => ({ ...piece, x: piece.x + delta.x, y: piece.y + delta.y })),
        new Set(editor.selected)
      )
  );
  let ghosts = $derived(
    hover && editor.tool === 'place'
      ? editor.activePresetId
        ? editor.presetPreview(hover.x, hover.y)
        : editor
            .ghosts(hover.x, hover.y)
            .map((piece) => ({ piece, color: editor.activePass.color }))
      : []
  );
  let validGhosts = $derived(
    ghosts.length > 0 &&
      (editor.activePresetId
        ? !!hover && editor.presetFits(hover.x, hover.y)
        : !editor.activePass.locked &&
          editor.activePass.visible &&
          canPlace(
            editor.doc,
            ghosts.map((row) => row.piece)
          ))
  );
  let status = $derived(
    dragging === 'move'
      ? validMove
        ? `Move ${delta.x}, ${delta.y} studs`
        : 'Cannot move here: overlap or board boundary.'
      : hover && editor.tool === 'place'
        ? `${hover.x}, ${hover.y} studs · ${validGhosts ? 'Ready to place' : 'Placement unavailable'}`
        : `${editor.selected.length} selected · Right-drag to orbit`
  );
  onMount(() => {
    let disposed = false;
    let instance: BoardScene | null = null;
    void import('../../three/scene')
      .then(({ BoardScene }) => {
        if (disposed) return;
        instance = new BoardScene(
          canvas,
          editor.camera3D,
          (camera, zoom) => {
            editor.camera3D = camera;
            editor.zoomPercent = zoom;
            projection = camera.projection ?? 'perspective';
          },
          (message) => {
            cancelDrag();
            error = message;
          }
        );
        instance.syncDocument(editor.doc);
        scene = instance;
      })
      .catch(() => {
        if (!disposed)
          error = '3D graphics are unavailable on this device. You can keep editing in Design.';
      });
    return () => {
      disposed = true;
      cancelDrag();
      instance?.dispose();
    };
  });
  $effect(() => {
    scene?.resize(width, height);
  });
  $effect(() => {
    scene?.syncDocument(editor.doc);
  });
  let firstZoomCommand = true;
  $effect(() => {
    const command = editor.zoomCommand;
    if (scene) {
      scene.setZoom(command.value, !firstZoomCommand);
      firstZoomCommand = false;
    }
  });
  $effect(() => {
    scene?.syncPreview(editor.selected, delta, validMove, ghosts, validGhosts);
  });
  $effect(() => {
    editor.doc.board;
    scene?.syncTrace(editor.trace);
  });
  $effect(() => {
    if (editor.canvasFocusToken)
      untrack(() => {
        if (editor.tool === 'place') hover = { x: 0, y: 0 };
        void tick().then(() => viewport?.focus({ preventScroll: true }));
      });
  });
  function inside(p: Point) {
    return p.x >= 0 && p.y >= 0 && p.x < editor.doc.board.width && p.y < editor.doc.board.height;
  }
  function point(e: { clientX: number; clientY: number }) {
    return scene?.point(e.clientX, e.clientY, dragging === 'move' ? dragHeight : 0.4) ?? null;
  }
  function screenPoint(e: { clientX: number; clientY: number }) {
    const r = viewport.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }
  function setHover(p: Point | null) {
    hover = p && inside(p) ? { x: Math.floor(p.x), y: Math.floor(p.y) } : null;
  }
  function down(event: PointerEvent) {
    if (
      !scene ||
      error ||
      dragging ||
      ![0, 1, 2].includes(event.button) ||
      (event.target as Element).closest(
        'button,input,select,a,[data-camera-controls],.canvas-bottom'
      )
    )
      return;
    viewport.focus({ preventScroll: true });
    event.preventDefault();
    const p = point(event);
    if (!p) return;
    const uid = scene.pick(event.clientX, event.clientY);
    if (event.button === 1 && editor.tool === 'place' && !space && uid) {
      editor.pickPiece(uid);
      setHover(p);
      return;
    }
    origin = p;
    startClient = screenPoint(event);
    lastClient = startClient;
    if (event.button === 2) dragging = 'orbit';
    else if (space || editor.tool === 'hand' || event.button === 1) dragging = 'pan';
    else if (editor.tool === 'place') {
      if (!inside(p)) return;
      if (editor.activePresetId) {
        editor.place(Math.floor(p.x), Math.floor(p.y));
        return;
      }
      if (!editor.beginPlacementStroke()) return;
      dragging = 'place';
      paintFrom = p;
      setHover(p);
      editor.paintPlacement(Math.floor(p.x), Math.floor(p.y));
    } else if (uid) {
      const row = editor.allPieces.find(({ piece }) => piece.uid === uid);
      if (!row || row.pass.locked) return;
      if (event.shiftKey || !editor.selected.includes(uid)) editor.select(uid, event.shiftKey);
      if (!editor.selected.includes(uid)) return;
      dragHeight = (getPiece(row.piece.pieceId)?.surfaceHeightPlates ?? 1) * 0.4;
      dragging = 'move';
      origin = point(event) ?? p;
      delta = { x: 0, y: 0 };
    } else {
      additive = event.shiftKey;
      if (!additive) editor.selected = [];
      dragging = 'box';
      marquee = { ...startClient, width: 0, height: 0 };
    }
    dragPointer = event.pointerId;
    viewport.setPointerCapture(event.pointerId);
  }
  function paintTo(p: Point) {
    for (const cell of placementPath(paintFrom, p, editor.doc.board))
      editor.paintPlacement(cell.x, cell.y);
    paintFrom = p;
  }
  function move(event: PointerEvent) {
    if (!scene || error || (dragPointer !== null && dragPointer !== event.pointerId)) return;
    const p = point(event),
      client = screenPoint(event);
    if (dragging === 'orbit') scene.orbit(client.x - lastClient.x, client.y - lastClient.y);
    else if (dragging === 'pan') scene.pan(client.x - lastClient.x, client.y - lastClient.y);
    else if (dragging === 'move' && p)
      delta = { x: Math.round(p.x - origin.x), y: Math.round(p.y - origin.y) };
    else if (dragging === 'box')
      marquee = {
        x: Math.min(client.x, startClient.x),
        y: Math.min(client.y, startClient.y),
        width: Math.abs(client.x - startClient.x),
        height: Math.abs(client.y - startClient.y)
      };
    else if (dragging === 'place' && p) {
      if (!editor.placementStrokeActive || editor.tool !== 'place' || editor.activePresetId) {
        cancelDrag();
        return;
      }
      for (const sample of event.getCoalescedEvents?.() ?? []) {
        const q = point(sample);
        if (q) paintTo(q);
      }
      paintTo(p);
      setHover(p);
    } else setHover(p);
    lastClient = client;
  }
  function up(event: PointerEvent) {
    if (dragPointer !== event.pointerId) return;
    if (dragging === 'place') {
      const p = point(event);
      if (p) paintTo(p);
      editor.finishPlacementStroke();
    }
    if (dragging === 'move' && (delta.x || delta.y)) editor.move(delta.x, delta.y);
    if (dragging === 'box' && marquee)
      editor.selected = [
        ...new Set([...(additive ? editor.selected : []), ...(scene?.marquee(marquee) ?? [])])
      ];
    cancelDrag();
  }
  function cancelDrag() {
    if (dragging === 'place') editor.finishPlacementStroke(true);
    dragging = null;
    delta = { x: 0, y: 0 };
    marquee = null;
    const id = dragPointer;
    dragPointer = null;
    if (id !== null && viewport?.hasPointerCapture(id)) viewport.releasePointerCapture(id);
  }
  function wheel(event: WheelEvent) {
    if (
      !scene ||
      error ||
      (event.target as Element).closest('button,[data-camera-controls],.canvas-bottom')
    )
      return;
    event.preventDefault();
    if (Math.abs(event.deltaX) > Math.abs(event.deltaY) && !event.ctrlKey)
      scene.pan(-event.deltaX, -event.deltaY);
    else
      editor.setZoom(
        Math.max(3, Math.min(600, editor.zoomPercent * Math.exp(-event.deltaY * 0.0015)))
      );
  }
  function keyboard(event: KeyboardEvent) {
    if (event.target !== viewport || error) return;
    if (event.key === 'Escape') {
      cancelDrag();
      return;
    }
    if (event.code === 'Space') {
      space = true;
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    if (event.ctrlKey || event.metaKey) return;
    const step = event.shiftKey ? 4 : 1;
    const dx = event.key === 'ArrowLeft' ? -step : event.key === 'ArrowRight' ? step : 0;
    const dy = event.key === 'ArrowUp' ? -step : event.key === 'ArrowDown' ? step : 0;
    if ((dx || dy) && (event.altKey || editor.tool === 'hand' || editor.tool === 'place')) {
      event.preventDefault();
      event.stopPropagation();
      if (event.altKey) scene?.orbit(-dx * 16, -dy * 16);
      else if (editor.tool === 'hand') scene?.pan(-dx * 32, -dy * 32);
      else
        hover = {
          x: Math.max(0, Math.min(editor.doc.board.width - 1, (hover?.x ?? 0) + dx)),
          y: Math.max(0, Math.min(editor.doc.board.height - 1, (hover?.y ?? 0) + dy))
        };
    } else if (event.key === 'Enter' && editor.tool === 'place') {
      event.preventDefault();
      event.stopPropagation();
      editor.place(hover?.x ?? 0, hover?.y ?? 0);
    } else if (event.key.toLowerCase() === 'i' && editor.tool === 'place' && hover && scene) {
      event.preventDefault();
      event.stopPropagation();
      const p = scene.project(hover.x + 0.5, hover.y + 0.5),
        r = viewport.getBoundingClientRect();
      const uid = scene.pick(r.left + p.x, r.top + p.y);
      if (uid) editor.pickPiece(uid);
      else editor.notify('No piece beneath the placement cursor.');
    }
  }
  function camera(action: CameraAction) {
    cancelDrag();
    scene?.cameraAction(action);
    if (['top', 'isometric', 'reset'].includes(action)) editor.setZoom('fit');
  }
  function setProjection(value: CameraProjection) {
    cancelDrag();
    scene?.setProjection(value);
  }
  function drop(event: DragEvent) {
    event.preventDefault();
    const p = point(event);
    if (!p || !inside(p) || !scene || error) return;
    const preset = event.dataTransfer?.getData('application/x-brickpress-preset'),
      id = event.dataTransfer?.getData('application/x-legopress-piece');
    if (preset) {
      editor.choosePreset(preset, false);
      editor.placePreset(Math.floor(p.x), Math.floor(p.y));
    } else if (id) {
      editor.choosePiece(id);
      editor.place(Math.floor(p.x), Math.floor(p.y), id);
    }
    hover = null;
    editor.focusCanvas();
  }
</script>

<svelte:window
  onkeyup={(e) => {
    if (e.code === 'Space') space = false;
  }}
  onblur={() => {
    space = false;
    cancelDrag();
  }}
/>
<!-- A custom application viewport owns its labeled keyboard editing commands. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="canvas-workspace three-workspace"
  class:placing={editor.tool === 'place'}
  class:panning={space || editor.tool === 'hand' || dragging === 'pan'}
  bind:this={viewport}
  bind:clientWidth={width}
  bind:clientHeight={height}
  role="application"
  tabindex="0"
  aria-label="Brickpress 3D artboard. Drag pieces to move; right-drag to orbit; Space-drag to pan. Alt and arrows orbit. Arrow keys move pieces or the placement cursor; Enter places; R rotates."
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={cancelDrag}
  onlostpointercapture={cancelDrag}
  onpointerleave={() => {
    if (!dragging && document.activeElement !== viewport) hover = null;
  }}
  onauxclick={(e) => e.preventDefault()}
  oncontextmenu={(e) => {
    if (!(e.target as Element).closest('button,[data-camera-controls]')) e.preventDefault();
  }}
  onwheel={wheel}
  onkeydown={keyboard}
  ondragover={(e) => {
    e.preventDefault();
    setHover(point(e));
  }}
  ondrop={drop}
>
  <canvas
    bind:this={canvas}
    class="three-canvas"
    aria-hidden="true"
    data-ready={!!scene && !error}
    data-projection={projection}
  ></canvas>
  {#if !scene && !error}<p class="three-message" role="status">Loading 3D view…</p>{/if}
  {#if error}<div class="three-message" role="alert">
      <p>{error}</p>
      <button class="size-trigger" onclick={() => editor.setView('2d')}>Return to Design</button>
    </div>{/if}
  {#if scene && !error}
    <div class="three-heading" data-camera-controls>
      <span class="three-label"><Icon name="cube" size={16} />3D board</span>
      <Popover label="3D camera controls" className="size-trigger">
        {#snippet trigger()}<Icon name="rotate" size={16} /> Camera{/snippet}
        <div data-camera-controls>
          <h3>Camera</h3>
          <fieldset class="camera-projection">
            <legend class="field-label">Projection</legend>
            <div class="camera-presets">
              <button
                class="size-trigger"
                aria-pressed={projection === 'perspective'}
                onclick={() => setProjection('perspective')}>Linear perspective</button
              >
              <button
                class="size-trigger"
                aria-pressed={projection === 'orthographic'}
                onclick={() => setProjection('orthographic')}>Orthographic</button
              >
            </div>
          </fieldset>
          <div class="camera-presets">
            <button class="size-trigger" onclick={() => camera('top')}>Top</button><button
              class="size-trigger"
              onclick={() => camera('isometric')}>Isometric</button
            ><button class="size-trigger" onclick={() => camera('reset')}>Reset camera</button>
          </div>
          <p class="field-label">Orbit</p>
          <div class="camera-buttons">
            {#each ['left', 'right', 'up', 'down'] as direction (direction)}<button
                class="size-trigger"
                aria-label={`Orbit ${direction}`}
                onclick={() => camera(direction as CameraAction)}>{direction}</button
              >{/each}
          </div>
          <p class="field-label">Pan</p>
          <div class="camera-buttons">
            {#each ['left', 'right', 'up', 'down'] as direction (direction)}<button
                class="size-trigger"
                aria-label={`Pan ${direction}`}
                onclick={() => camera(`pan-${direction}` as CameraAction)}>{direction}</button
              >{/each}
          </div>
          <p class="fine-print">
            Right-drag or Alt + arrows to orbit. Space-drag to pan. Scroll to zoom. Touch: use these
            buttons to orbit and the Hand tool to pan.
          </p>
        </div>
      </Popover>
    </div>
    <p class="three-status" role="status" aria-live="polite">{status}</p>
  {/if}
  {#if marquee}<svg class="three-marquee" aria-hidden="true"
      ><rect
        {...marquee}
        fill="#0057e6"
        fill-opacity=".08"
        stroke="#0057e6"
        stroke-width="1"
      /></svg
    >{/if}
  <div class="canvas-bottom">
    <BoardSize {editor} compact /><ZoomControls {editor} location="Canvas" fit />
  </div>
</div>

<style>
  .three-canvas,
  .three-marquee {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
  }
  .three-marquee {
    pointer-events: none;
  }
  .three-heading {
    position: absolute;
    top: 16px;
    left: 16px;
    right: 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 8px;
    pointer-events: none;
  }
  .three-heading :global(button) {
    pointer-events: auto;
  }
  .three-label {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--muted);
    font-size: 0.6875rem;
  }
  .three-status {
    position: absolute;
    top: 56px;
    left: 16px;
    right: 16px;
    margin: 0;
    font-size: 0.6875rem;
    color: var(--muted);
    pointer-events: none;
  }
  .three-message {
    position: absolute;
    top: 35%;
    left: 24px;
    right: 24px;
    text-align: center;
    font-size: 0.8125rem;
    color: var(--ink);
  }
  .three-message button {
    margin: 12px auto;
  }
  .camera-buttons,
  .camera-presets {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin: 8px 0 12px;
  }
  .camera-projection {
    border: 0;
    padding: 0;
    margin: 0 0 12px;
    min-width: 0;
  }
  .camera-projection legend {
    margin-bottom: 6px;
  }
  .camera-projection button[aria-pressed='true'] {
    background: var(--blue-soft);
    color: var(--blue);
    border-color: var(--blue);
    box-shadow: inset 0 -2px var(--blue);
  }
  .camera-buttons button {
    text-transform: capitalize;
    flex: 1;
  }
</style>
