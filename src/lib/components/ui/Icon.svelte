<script lang="ts">
  import Menu from '@lucide/svelte/icons/menu';
  import EllipsisVertical from '@lucide/svelte/icons/ellipsis-vertical';
  import Box from '@lucide/svelte/icons/box';
  import Plus from '@lucide/svelte/icons/plus';
  import Minus from '@lucide/svelte/icons/minus';
  import MousePointer2 from '@lucide/svelte/icons/mouse-pointer-2';
  import Hand from '@lucide/svelte/icons/hand';
  import Grid3X3 from '@lucide/svelte/icons/grid-3x3';
  import Undo2 from '@lucide/svelte/icons/undo-2';
  import Redo2 from '@lucide/svelte/icons/redo-2';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import Copy from '@lucide/svelte/icons/copy';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import Save from '@lucide/svelte/icons/save';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import File from '@lucide/svelte/icons/file';
  import Upload from '@lucide/svelte/icons/upload';
  import Search from '@lucide/svelte/icons/search';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import X from '@lucide/svelte/icons/x';
  import Eye from '@lucide/svelte/icons/eye';
  import EyeOff from '@lucide/svelte/icons/eye-off';
  import LockKeyhole from '@lucide/svelte/icons/lock-keyhole';
  import LockKeyholeOpen from '@lucide/svelte/icons/lock-keyhole-open';
  import ChevronUp from '@lucide/svelte/icons/chevron-up';
  import Scan from '@lucide/svelte/icons/scan';
  import Info from '@lucide/svelte/icons/info';
  import Check from '@lucide/svelte/icons/check';
  import SlidersHorizontal from '@lucide/svelte/icons/sliders-horizontal';
  import Layers from '@lucide/svelte/icons/layers';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Stamp from '@lucide/svelte/icons/stamp';
  import BringToFront from '@lucide/svelte/icons/bring-to-front';
  import SendToBack from '@lucide/svelte/icons/send-to-back';
  import GripVertical from '@lucide/svelte/icons/grip-vertical';
  let { name, size = 18 }: { name: string; size?: number } = $props();
  const icons = {
    menu: Menu,
    more: EllipsisVertical,
    cube: Box,
    plus: Plus,
    minus: Minus,
    arrow: MousePointer2,
    hand: Hand,
    grid: Grid3X3,
    undo: Undo2,
    redo: Redo2,
    rotate: RotateCw,
    duplicate: Copy,
    trash: Trash2,
    save: Save,
    open: FolderOpen,
    file: File,
    export: Upload,
    search: Search,
    chevron: ChevronDown,
    close: X,
    eye: Eye,
    hidden: EyeOff,
    lock: LockKeyhole,
    unlock: LockKeyholeOpen,
    up: ChevronUp,
    down: ChevronDown,
    fit: Scan,
    info: Info,
    check: Check,
    settings: SlidersHorizontal,
    layers: Layers,
    seed: RefreshCw,
    stamp: Stamp,
    forward: BringToFront,
    backward: SendToBack,
    grip: GripVertical
  };
  let Glyph = $derived(icons[name as keyof typeof icons] ?? File);
</script>

<Glyph {size} strokeWidth={1.65} aria-hidden="true" class="interface-icon" />
