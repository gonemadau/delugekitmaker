<script lang="ts">
  import type { Drum } from "../api";
  import { appStore } from "../stores/app.svelte";
  import { Music4, X, Plus } from "lucide-svelte";

  let { index, drum }: { index: number; drum: Drum | null } = $props();

  let pressed = $state(false);
  let dragOver = $state(false);

  let hasSample = $derived(!!(drum?.osc1?.file_name && drum.osc1.file_name.length > 0));
  let displayName = $derived(drum?.name || `Pad ${index + 1}`);
  let sampleFile = $derived(drum?.osc1?.file_name ?? "");
  let sampleLabel = $derived(sampleFile ? basename(sampleFile) : "—");
  let isSelected = $derived(appStore.selectedPad === index);

  function basename(p: string): string {
    const ix = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
    return ix >= 0 ? p.slice(ix + 1) : p;
  }

  async function onClick() {
    if (!hasSample) {
      appStore.selectPad(index);
      await appStore.pickSampleForPad(index);
      return;
    }
    pressed = true;
    setTimeout(() => (pressed = false), 120);
    appStore.selectPad(index);
    appStore.auditionPad(index);
  }

  function onClear(e: MouseEvent) {
    e.stopPropagation();
    appStore.clearPad(index);
  }

  // ---- Drag and drop (HTML5, in-app) ----
  function onDragStart(e: DragEvent) {
    if (!hasSample) {
      e.preventDefault();
      return;
    }
    e.dataTransfer?.setData("application/x-dkm-pad", String(index));
    e.dataTransfer?.setData("text/plain", `pad-${index}`);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "move";
  }

  function onDragOver(e: DragEvent) {
    const types = e.dataTransfer?.types ?? [];
    if (types.includes("application/x-dkm-pad") || types.includes("application/x-dkm-file")) {
      e.preventDefault();
      if (e.dataTransfer) e.dataTransfer.dropEffect = types.includes("application/x-dkm-file") ? "copy" : "move";
      dragOver = true;
    }
  }
  function onDragLeave() {
    dragOver = false;
  }
  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    const padSrc = e.dataTransfer?.getData("application/x-dkm-pad");
    const filePath = e.dataTransfer?.getData("application/x-dkm-file");
    if (filePath && filePath.length > 0) {
      appStore.assignSampleToPad(index, filePath);
      return;
    }
    if (padSrc !== undefined && padSrc !== "") {
      const src = Number(padSrc);
      if (Number.isFinite(src) && src !== index) {
        appStore.swapPads(src, index);
      }
    }
  }
</script>

<div
  role="button"
  tabindex="0"
  class="pad group relative flex aspect-square flex-col items-start justify-between rounded-xl border p-2 text-left"
  class:active={pressed}
  class:has-sample={hasSample}
  class:empty={!hasSample}
  class:selected={isSelected}
  class:drag-over={dragOver}
  draggable={hasSample}
  onclick={onClick}
  onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); onClick(); } }}
  ondragstart={onDragStart}
  ondragover={onDragOver}
  ondragleave={onDragLeave}
  ondrop={onDrop}
  title={hasSample ? `${displayName}\n${sampleFile}` : `Pad ${index + 1} — click to pick a sample`}
>
  <div class="flex w-full items-center justify-between">
    <span class="text-[9px] font-medium uppercase tracking-wider text-text-muted">
      {String(index + 1).padStart(2, "0")}
    </span>
    {#if hasSample}
      <span class="dot"></span>
    {/if}
  </div>

  <div class="flex w-full min-w-0 flex-col items-start gap-0.5">
    <span class="line-clamp-1 w-full truncate text-sm font-semibold">{displayName}</span>
    <div class="flex w-full min-w-0 items-center gap-1 text-[10px] text-text-muted">
      <Music4 class="h-2.5 w-2.5 flex-shrink-0" />
      <span class="block w-full truncate">{sampleLabel}</span>
    </div>
  </div>

  {#if hasSample}
    <button
      class="clear-btn"
      onclick={onClear}
      aria-label="Clear pad"
      title="Clear pad"
    >
      <X class="h-3 w-3" />
    </button>
  {:else}
    <span class="hint-plus" aria-hidden="true">
      <Plus class="h-3.5 w-3.5" />
    </span>
  {/if}
</div>

<style>
  .pad {
    border-color: var(--color-border);
    background: var(--color-bg-1);
    transition: transform 0.12s cubic-bezier(0.34, 1.56, 0.64, 1),
      background 0.18s, border-color 0.18s, box-shadow 0.18s;
    overflow: hidden;
    min-width: 0;
  }
  .pad.has-sample {
    background: var(--color-bg-2);
    border-color: color-mix(in oklch, var(--color-accent) 30%, var(--color-border));
  }
  .pad.empty {
    background: color-mix(in oklch, var(--color-bg-1) 60%, transparent);
    border-style: dashed;
    color: var(--color-text-muted);
  }
  .pad:hover {
    background: var(--color-bg-3);
    border-color: var(--color-accent);
    box-shadow: 0 4px 18px -8px var(--color-accent);
  }
  .pad.active {
    transform: scale(0.95);
    background: color-mix(in oklch, var(--color-accent) 35%, var(--color-bg-2));
    box-shadow: 0 0 0 2px var(--color-accent), 0 6px 24px -6px var(--color-accent);
  }
  .pad.selected {
    box-shadow: 0 0 0 2px var(--color-accent), 0 0 20px -2px color-mix(in oklch, var(--color-accent) 60%, transparent);
  }
  .pad.drag-over {
    background: color-mix(in oklch, var(--color-accent-warm) 25%, var(--color-bg-2));
    border-color: var(--color-accent-warm);
    box-shadow: 0 0 0 2px var(--color-accent-warm);
  }

  .dot {
    height: 6px; width: 6px; border-radius: 999px;
    background: var(--color-accent); opacity: 0.85;
  }

  .clear-btn {
    position: absolute; top: 4px; right: 4px;
    display: flex; align-items: center; justify-content: center;
    height: 18px; width: 18px;
    border-radius: 6px;
    background: color-mix(in oklch, var(--color-bg-0) 80%, transparent);
    color: var(--color-text-muted);
    border: 1px solid transparent;
    opacity: 0;
    transition: opacity 0.15s, background 0.15s, color 0.15s;
    cursor: pointer;
    padding: 0;
  }
  .pad:hover .clear-btn { opacity: 1; }
  .clear-btn:hover {
    background: var(--color-accent-danger, oklch(0.7 0.2 25));
    color: white;
  }

  .hint-plus {
    position: absolute; top: 50%; left: 50%;
    transform: translate(-50%, -50%);
    color: color-mix(in oklch, var(--color-text-muted) 60%, transparent);
    pointer-events: none;
    opacity: 0;
    transition: opacity 0.15s;
  }
  .pad.empty:hover .hint-plus { opacity: 1; }
</style>
