<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { getPiece, pieces } from '../../catalog/catalog';
  import { bounds, canPlace, pieceTransform } from '../../geometry/geometry';
  import { designMaskId, designMaskMarkup } from '../../geometry/design-surface';
  import { paperMarkup, passTransform } from '../../printing/print-engine';
  import type { Editor } from '../../stores/editor.svelte';
  import Piece from './Piece.svelte';
  import Icon from '../ui/Icon.svelte';
  import BoardSize from '../ui/BoardSize.svelte';
  import ZoomControls from '../ui/ZoomControls.svelte';
  let { editor }: { editor: Editor } = $props();
  let viewport: HTMLDivElement;
  let svg: SVGSVGElement;
  let viewportWidth = $state(800),
    viewportHeight = $state(700);
  let panX = $state(0),
    panY = $state(0),
    space = $state(false);
  let hover = $state.raw<{ x: number; y: number } | null>(null);
  let delta = $state.raw({ x: 0, y: 0 });
  let dragKind = $state<'pan' | 'move' | 'box' | null>(null);
  let marquee = $state.raw<{ x: number; y: number; width: number; height: number } | null>(null);
  let hint = $state(true);
  let origin = { x: 0, y: 0 },
    clientOrigin = { x: 0, y: 0 },
    initialPan = { x: 0, y: 0 };
  let additive = false;
  const studPixels = 32;
  const designMasks = pieces.map((piece) => designMaskMarkup(piece)).join('');
  let board = $derived(editor.doc.board);
  let fit = $derived(
    Math.min(
      (viewportWidth - 96) / (board.width * studPixels),
      (viewportHeight - 180) / (board.height * studPixels)
    )
  );
  let zoom = $derived(
    editor.zoomCommand.value === 'fit'
      ? Math.max(0.12, Math.min(2, fit))
      : Math.max(0.12, Math.min(6, editor.zoomCommand.value / 100))
  );
  let printed = $derived(editor.mode === 'print');
  let selectedIds = $derived(new Set(editor.selected));
  let selected = $derived(editor.selectedPieces);
  let ghostRows = $derived(
    hover && editor.tool === 'place' && !printed
      ? editor.activePresetId
        ? editor.presetPreview(hover.x, hover.y)
        : editor
            .ghosts(hover.x, hover.y)
            .map((piece) => ({ piece, color: editor.activePass.color }))
      : []
  );
  let ghostPieces = $derived(ghostRows.map((row) => row.piece));
  let ghostsValid = $derived(
    ghostPieces.length > 0 &&
      (editor.activePresetId
        ? !!hover && editor.presetFits(hover.x, hover.y)
        : !editor.activePass.locked &&
          editor.activePass.visible &&
          canPlace(editor.doc, ghostPieces))
  );
  let dragValid = $derived(
    !dragKind ||
      dragKind !== 'move' ||
      canPlace(
        editor.doc,
        selected.map(({ piece }) => ({ ...piece, x: piece.x + delta.x, y: piece.y + delta.y })),
        selectedIds
      )
  );
  let paper = $derived(paperMarkup(editor.doc.paper, board.width, board.height));
  $effect(() => {
    if (editor.canvasFocusToken) {
      untrack(() => {
        if (editor.tool === 'place') hover = { x: 0, y: 0 };
        void tick().then(() => viewport?.focus({ preventScroll: true }));
      });
    }
  });
  function keyboard(event: KeyboardEvent) {
    if (
      event.target !== viewport ||
      (printed && editor.tool !== 'hand') ||
      event.ctrlKey ||
      event.metaKey ||
      event.altKey
    )
      return;
    const step = event.shiftKey ? 4 : 1;
    if (event.key.startsWith('Arrow') && (editor.tool === 'place' || editor.tool === 'hand')) {
      event.preventDefault();
      event.stopPropagation();
      const dx = event.key === 'ArrowRight' ? step : event.key === 'ArrowLeft' ? -step : 0;
      const dy = event.key === 'ArrowDown' ? step : event.key === 'ArrowUp' ? -step : 0;
      if (editor.tool === 'hand') {
        panX -= dx * studPixels * zoom;
        panY -= dy * studPixels * zoom;
      } else
        hover = {
          x: Math.max(0, Math.min(board.width - 1, (hover?.x ?? 0) + dx)),
          y: Math.max(0, Math.min(board.height - 1, (hover?.y ?? 0) + dy))
        };
    } else if (event.key === 'Enter' && editor.tool === 'place') {
      event.preventDefault();
      event.stopPropagation();
      editor.place(hover?.x ?? 0, hover?.y ?? 0);
    } else if (event.key.toLowerCase() === 'i' && editor.tool === 'place' && hover) {
      event.preventDefault();
      event.stopPropagation();
      const matrix = svg?.getScreenCTM();
      if (!matrix) return;
      const cursor = new DOMPoint(hover.x + 0.5, hover.y + 0.5).matrixTransform(matrix);
      const uid = document
        .elementFromPoint(cursor.x, cursor.y)
        ?.closest('[data-uid]')
        ?.getAttribute('data-uid');
      if (uid) editor.pickPiece(uid);
      else editor.notify('No piece beneath the placement cursor.');
    }
  }
  $effect(() => {
    const command = editor.zoomCommand;
    // Consume the external Fit command; normal zoom is purely derived.
    if (command.value === 'fit') {
      panX = 0;
      panY = 0;
    }
  });
  $effect(() => {
    editor.zoomPercent = Math.round(zoom * 100);
  });
  function point(event: { clientX: number; clientY: number }) {
    const matrix = svg?.getScreenCTM();
    if (!matrix) return { x: 0, y: 0 };
    const p = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix.inverse());
    return { x: p.x, y: p.y };
  }
  function inside(p: { x: number; y: number }) {
    return p.x >= 0 && p.y >= 0 && p.x < board.width && p.y < board.height;
  }
  function pointerdown(event: PointerEvent) {
    if (event.button !== 0 && event.button !== 1) return;
    if ((event.target as Element).closest('button, input, .onboarding, .contextual-toolbar'))
      return;
    viewport.focus({ preventScroll: true });
    const p = point(event);
    const uid = (event.target as Element).closest('[data-uid]')?.getAttribute('data-uid');
    if (event.button === 1 && editor.tool === 'place' && !printed && !space && uid) {
      event.preventDefault();
      editor.pickPiece(uid);
      hover = inside(p) ? { x: Math.floor(p.x), y: Math.floor(p.y) } : null;
      return;
    }
    origin = p;
    clientOrigin = { x: event.clientX, y: event.clientY };
    if (space || editor.tool === 'hand' || event.button === 1) {
      dragKind = 'pan';
      initialPan = { x: panX, y: panY };
      event.preventDefault();
    } else if (printed) return;
    else if (!inside(p)) {
      if (!event.shiftKey) editor.selected = [];
      return;
    } else if (editor.tool === 'place') {
      editor.place(Math.floor(p.x), Math.floor(p.y));
      event.preventDefault();
      return;
    } else {
      if (uid) {
        const row = editor.allPieces.find((item) => item.piece.uid === uid);
        if (!row || row.pass.locked) return;
        if (event.shiftKey || !selectedIds.has(uid)) editor.select(uid, event.shiftKey);
        if (editor.selected.includes(uid)) {
          dragKind = 'move';
          delta = { x: 0, y: 0 };
        }
      } else {
        additive = event.shiftKey;
        if (!additive) editor.selected = [];
        dragKind = 'box';
        marquee = { x: p.x, y: p.y, width: 0, height: 0 };
      }
      event.preventDefault();
    }
    if (dragKind) viewport.setPointerCapture(event.pointerId);
  }
  function pointermove(event: PointerEvent) {
    const p = point(event);
    if (dragKind === 'pan') {
      panX = initialPan.x + event.clientX - clientOrigin.x;
      panY = initialPan.y + event.clientY - clientOrigin.y;
    } else if (dragKind === 'move')
      delta = { x: Math.round(p.x - origin.x), y: Math.round(p.y - origin.y) };
    else if (dragKind === 'box')
      marquee = {
        x: Math.min(p.x, origin.x),
        y: Math.min(p.y, origin.y),
        width: Math.abs(p.x - origin.x),
        height: Math.abs(p.y - origin.y)
      };
    else hover = inside(p) ? { x: Math.floor(p.x), y: Math.floor(p.y) } : null;
  }
  function pointerup(event: PointerEvent) {
    if (dragKind === 'move' && (delta.x || delta.y)) editor.move(delta.x, delta.y);
    if (dragKind === 'box' && marquee) {
      const box = marquee;
      const hits = editor.allPieces
        .filter(({ piece, pass }) => {
          const b = bounds(piece);
          return (
            pass.visible &&
            !pass.locked &&
            b.x < box.x + box.width &&
            b.x + b.width > box.x &&
            b.y < box.y + box.height &&
            b.y + b.height > box.y
          );
        })
        .map((p) => p.piece.uid);
      editor.selected = [...new Set([...(additive ? editor.selected : []), ...hits])];
    }
    cancelDrag();
    if (viewport.hasPointerCapture(event.pointerId))
      viewport.releasePointerCapture(event.pointerId);
  }
  function cancelDrag() {
    dragKind = null;
    delta = { x: 0, y: 0 };
    marquee = null;
  }
  function wheel(event: WheelEvent) {
    event.preventDefault();
    if (!event.ctrlKey && !event.metaKey && Math.abs(event.deltaX) > Math.abs(event.deltaY)) {
      panX -= event.deltaX;
      panY -= event.deltaY;
      return;
    }
    const next = Math.max(0.12, Math.min(6, zoom * Math.exp(-event.deltaY * 0.0015)));
    const rect = viewport.getBoundingClientRect(),
      x = event.clientX - rect.left - rect.width / 2,
      y = event.clientY - rect.top - rect.height / 2;
    panX = x - ((x - panX) * next) / zoom;
    panY = y - ((y - panY) * next) / zoom;
    editor.setZoom(next * 100);
  }
  function dragover(event: DragEvent) {
    event.preventDefault();
    const p = point(event);
    hover = inside(p) ? { x: Math.floor(p.x), y: Math.floor(p.y) } : null;
  }
  function drop(event: DragEvent) {
    event.preventDefault();
    const id = event.dataTransfer?.getData('application/x-legopress-piece'),
      presetId = event.dataTransfer?.getData('application/x-brickpress-preset'),
      p = point(event);
    if (presetId && inside(p)) {
      editor.choosePreset(presetId, false);
      editor.placePreset(Math.floor(p.x), Math.floor(p.y));
      editor.focusCanvas();
    } else if (id && inside(p)) {
      editor.mode = 'design';
      editor.choosePiece(id);
      editor.place(Math.floor(p.x), Math.floor(p.y), id);
    }
    hover = null;
  }
</script>

<svelte:window
  onkeydown={(e) => {
    keyboard(e);
    if (e.code === 'Space' && (e.target === viewport || e.target === document.body)) {
      space = true;
      e.preventDefault();
    }
    if (e.key === 'Escape') cancelDrag();
  }}
  onkeyup={(e) => {
    if (e.code === 'Space') space = false;
  }}
  onblur={() => {
    space = false;
    cancelDrag();
  }}
/>
<!-- The application canvas needs focus for its editor keyboard controls. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="canvas-workspace"
  class:placing={editor.tool === 'place' && !printed}
  class:panning={space || editor.tool === 'hand' || dragKind === 'pan'}
  bind:this={viewport}
  bind:clientWidth={viewportWidth}
  bind:clientHeight={viewportHeight}
  role="application"
  tabindex="0"
  aria-label="Brickpress artboard. Arrow keys move selected pieces or the placement cursor. Enter places a piece. R rotates. H and arrows pan. Middle-click a piece in Place mode to pick its shape and orientation, or use arrows and I to pick beneath the cursor."
  onpointerdown={pointerdown}
  onauxclick={(event) => {
    if (event.button === 1) event.preventDefault();
  }}
  onpointermove={pointermove}
  onpointerup={pointerup}
  onpointercancel={cancelDrag}
  onpointerleave={() => {
    if (!dragKind && document.activeElement !== viewport) hover = null;
  }}
  onwheel={wheel}
  ondragover={dragover}
  ondrop={drop}
>
  {#if selected.length && !printed}<div
      class="contextual-toolbar"
      role="toolbar"
      aria-label="Selection actions"
    >
      <span class="mono">{selected.length}</span><button
        class="icon-button"
        title="Rotate selection · R"
        aria-label="Rotate selection"
        onclick={() => editor.rotate()}><Icon name="rotate" size={16} /></button
      ><button
        class="icon-button"
        title="Duplicate selection"
        aria-label="Duplicate selection"
        onclick={() => editor.duplicate()}><Icon name="duplicate" size={16} /></button
      ><button
        class="icon-button"
        title="Delete selection"
        aria-label="Delete selection"
        onclick={() => editor.remove()}><Icon name="trash" size={16} /></button
      ><span class="divider"></span><input
        type="color"
        value={editor.activePass.color}
        aria-label="Selection ink color"
        title="Selection ink color"
        onchange={(e) => editor.setColor(e.currentTarget.value)}
      /><button
        class="icon-button"
        title="Bring forward"
        aria-label="Bring selection forward"
        onclick={() => editor.reorderSelected(1)}><Icon name="forward" size={16} /></button
      ><button
        class="icon-button"
        title="Send backward"
        aria-label="Send selection backward"
        onclick={() => editor.reorderSelected(-1)}><Icon name="backward" size={16} /></button
      >
    </div>{/if}
  <div
    class="artboard-transform"
    style:width={`${board.width * studPixels}px`}
    style:height={`${board.height * studPixels}px`}
    style:--canvas-zoom={zoom}
    style:transform={`translate(calc(-50% + ${panX}px), calc(-50% + ${panY}px)) scale(${zoom})`}
  >
    <div class="board-ruler ruler-x">
      {#each Array.from({ length: Math.ceil(board.width / 4) }, (_, i) => i * 4) as value (value)}<span
          style:left={`${(value / board.width) * 100}%`}>{value}</span
        >{/each}<span style:left="100%">{board.width}</span>
    </div>
    <div class="board-ruler ruler-y">
      {#each Array.from({ length: Math.ceil(board.height / 4) }, (_, i) => i * 4) as value (value)}<span
          style:top={`${(value / board.height) * 100}%`}>{value}</span
        >{/each}
    </div>
    <svg
      class="artboard"
      bind:this={svg}
      viewBox={`0 0 ${board.width} ${board.height}`}
      role="img"
      aria-label={`${editor.doc.name}, ${board.width} by ${board.height} studs`}
    >
      {@html paper}
      {#if !printed && editor.trace?.visible}
        <image
          data-editor-guide="tracing"
          href={editor.trace.url}
          x={editor.trace.x}
          y={editor.trace.y}
          width={editor.trace.width}
          height={editor.trace.height}
          opacity={editor.trace.opacity}
          preserveAspectRatio="none"
          pointer-events="none"
          aria-hidden="true"
        />
      {/if}
      {#if !printed}<defs>{@html designMasks}</defs>{/if}
      {#if !printed && editor.doc.options.grid !== 'off' && editor.gridAppearance === 'embossed'}
        <defs>
          <radialGradient id="stud-highlight" cx="35%" cy="25%"
            ><stop offset="0" stop-color="white" stop-opacity=".72" /><stop
              offset=".8"
              stop-color={editor.doc.paper.color}
              stop-opacity=".2"
            /><stop offset="1" stop-color="#111827" stop-opacity=".1" /></radialGradient
          >
          <pattern id="embossed-stud-guide" width="1" height="1" patternUnits="userSpaceOnUse"
            ><circle cx=".51" cy=".54" r=".225" fill="#111827" opacity=".1" /><circle
              cx=".5"
              cy=".5"
              r=".22"
              fill={editor.doc.paper.color}
            /><circle
              cx=".5"
              cy=".5"
              r=".22"
              fill="url(#stud-highlight)"
              stroke="white"
              stroke-opacity=".38"
              stroke-width=".015"
            /></pattern
          >
        </defs>
        <rect
          data-editor-guide="embossed"
          width={board.width}
          height={board.height}
          fill="url(#embossed-stud-guide)"
          pointer-events="none"
        />
      {/if}
      {#if !printed && editor.doc.options.grid !== 'off'}
        <defs
          ><pattern id="stud-grid" width="1" height="1" patternUnits="userSpaceOnUse"
            >{#if editor.doc.options.grid === 'points' && editor.gridAppearance === 'flat'}<circle
                cx=".5"
                cy=".5"
                r=".018"
                fill="#747e8d"
                opacity=".42"
              />{:else if editor.doc.options.grid !== 'points'}<path
                d="M1 0H0V1"
                fill="none"
                stroke="#747e8d"
                stroke-width={editor.doc.options.grid === 'squares' ? 0.012 : 0.007}
                opacity=".24"
              />{/if}</pattern
          ><pattern
            id="major-grid"
            width={editor.doc.options.majorGrid}
            height={editor.doc.options.majorGrid}
            patternUnits="userSpaceOnUse"
            ><path
              d={`M${editor.doc.options.majorGrid} 0H0V${editor.doc.options.majorGrid}`}
              fill="none"
              stroke="#747e8d"
              stroke-width=".015"
              opacity=".2"
            /></pattern
          ></defs
        >
        <rect
          width={board.width}
          height={board.height}
          fill="url(#stud-grid)"
        />{#if editor.doc.options.grid !== 'points'}<rect
            width={board.width}
            height={board.height}
            fill="url(#major-grid)"
          />{/if}
      {/if}
      {#each editor.doc.passes as pass (pass.id)}
        {#if pass.visible}<g
            data-pass={pass.id}
            transform={printed ? passTransform(pass, editor.doc.printSettings) : undefined}
          >
            {#each pass.pieces as placed (placed.uid)}<g
                transform={dragKind === 'move' && selectedIds.has(placed.uid)
                  ? `translate(${delta.x} ${delta.y})`
                  : undefined}
                ><Piece
                  {placed}
                  color={pass.color}
                  {printed}
                  settings={editor.doc.printSettings}
                  paper={editor.doc.paper}
                /></g
              >{/each}
          </g>{/if}
      {/each}
      {#if !printed}
        <g pointer-events="none"
          >{#each selected as item (item.piece.uid)}{@const b = bounds(item.piece)}<rect
              x={b.x + (dragKind === 'move' ? delta.x : 0) + 0.01}
              y={b.y + (dragKind === 'move' ? delta.y : 0) + 0.01}
              width={b.width - 0.02}
              height={b.height - 0.02}
              rx=".02"
              fill="none"
              stroke={dragValid ? '#0057e6' : '#b42318'}
              stroke-width="1.2"
              vector-effect="non-scaling-stroke"
              stroke-dasharray="3 2"
            />{/each}</g
        >
        {#if marquee}<rect
            {...marquee}
            fill="#0057e6"
            fill-opacity=".08"
            stroke="#0057e6"
            stroke-width="1"
            vector-effect="non-scaling-stroke"
            pointer-events="none"
          />{/if}
        {#each ghostRows as row, i (i)}{@const ghost = row.piece}{@const p = getPiece(
            ghost.pieceId
          )!}<path
            d={p.geometry.path}
            transform={pieceTransform(p, ghost, 1)}
            fill={ghostsValid ? row.color : '#b42318'}
            fill-rule={p.geometry.fillRule}
            mask={`url(#${designMaskId(p)})`}
            opacity=".32"
            pointer-events="none"
          />{/each}
        {#if editor.doc.options.symmetry === 'x' || editor.doc.options.symmetry === 'both'}<path
            d={`M${board.width / 2} 0v${board.height}`}
            stroke="#0057e6"
            stroke-width=".016"
            stroke-dasharray=".1 .14"
            opacity=".6"
            pointer-events="none"
          />{/if}
        {#if editor.doc.options.symmetry === 'y' || editor.doc.options.symmetry === 'both'}<path
            d={`M0 ${board.height / 2}h${board.width}`}
            stroke="#0057e6"
            stroke-width=".016"
            stroke-dasharray=".1 .14"
            opacity=".6"
            pointer-events="none"
          />{/if}
      {/if}
    </svg>
    <div class="board-footnote">
      <span>{editor.doc.paper.preset.toUpperCase()}</span><span
        >{editor.doc.passes.filter((p) => p.visible).length} INK PASSES · {editor.allPieces.length} INDIVIDUAL
        PIECES</span
      >
    </div>
  </div>
  {#if hint}<div class="onboarding">
      <span class="onboarding-mark"><Icon name="stamp" size={19} /></span>
      <div>
        <strong>Choose a piece to begin</strong>
        <p>Click the grid or use arrows + Enter. <kbd>R</kbd> rotates.</p>
      </div>
      <button class="icon-button" aria-label="Dismiss editor hint" onclick={() => (hint = false)}
        ><Icon name="close" size={14} /></button
      >
    </div>{/if}
  <div class="canvas-bottom">
    <BoardSize {editor} compact />
    <ZoomControls {editor} location="Canvas" fit />
  </div>
</div>
