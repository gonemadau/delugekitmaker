<script lang="ts">
  import { onDestroy } from "svelte";
  import { appStore } from "../stores/app.svelte";

  const STEPS = 16;

  let kitPads = $derived(() => {
    const kit = appStore.activeKit;
    if (!kit) return [] as number[];
    const out: number[] = [];
    for (let i = 0; i < Math.min(16, kit.drums.length); i++) {
      if (kit.drums[i].osc1?.file_name) out.push(i);
    }
    while (out.length < 4 && out.length < 16) {
      const next = out.length;
      if (!out.includes(next)) out.push(next);
    }
    return out;
  });

  // Drag-paint state.
  let painting: { mode: "on" | "off"; padIdx: number; pointerId: number } | null = null;

  function isOn(p: number, s: number): boolean { return appStore.patternGrid[p][s] === true; }
  function vel(p: number, s: number): number { return appStore.velocityGrid[p][s]; }
  function isAccent(p: number, s: number): boolean { return isOn(p, s) && vel(p, s) >= 120; }

  function paintAt(p: number, s: number) {
    if (!painting) return;
    if (painting.padIdx !== p) return; // single-row paint
    const wantOn = painting.mode === "on";
    if (appStore.patternGrid[p][s] === wantOn) return; // no-op, avoid redundant pushPattern
    appStore.setStep(p, s, wantOn, wantOn ? 100 : undefined);
  }

  function onCellPointerDown(e: PointerEvent, p: number, s: number) {
    // Right-click or alt-click = toggle accent on this step
    if (e.button === 2 || e.altKey) {
      e.preventDefault();
      appStore.toggleAccent(p, s);
      return;
    }
    if (e.button !== 0) return;
    e.preventDefault();
    const wasOn = isOn(p, s);
    painting = { mode: wasOn ? "off" : "on", padIdx: p, pointerId: e.pointerId };
    paintAt(p, s);
    // Force-handle the very first cell (paintAt early-returns if same state — but we
    // want to actually toggle this one because it's the user's intentional click).
    if (wasOn) {
      appStore.setStep(p, s, false);
    } else {
      appStore.setStep(p, s, true, 100);
    }
    window.addEventListener("pointermove", onWinPointerMove);
    window.addEventListener("pointerup", onWinPointerUp);
    window.addEventListener("pointercancel", onWinPointerUp);
  }

  function onWinPointerMove(e: PointerEvent) {
    if (!painting || e.pointerId !== painting.pointerId) return;
    const el = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null;
    if (!el) return;
    const cell = el.closest("[data-step-cell]") as HTMLElement | null;
    if (!cell) return;
    const p = Number(cell.dataset.pad);
    const s = Number(cell.dataset.step);
    if (!Number.isFinite(p) || !Number.isFinite(s)) return;
    paintAt(p, s);
  }

  function onWinPointerUp(_e: PointerEvent) {
    painting = null;
    window.removeEventListener("pointermove", onWinPointerMove);
    window.removeEventListener("pointerup", onWinPointerUp);
    window.removeEventListener("pointercancel", onWinPointerUp);
  }

  onDestroy(() => {
    window.removeEventListener("pointermove", onWinPointerMove);
    window.removeEventListener("pointerup", onWinPointerUp);
    window.removeEventListener("pointercancel", onWinPointerUp);
  });

  function labelFor(padIndex: number): string {
    const kit = appStore.activeKit;
    if (!kit) return `P${padIndex + 1}`;
    return kit.drums[padIndex]?.name || `P${padIndex + 1}`;
  }
</script>

<div class="flex max-h-[280px] flex-col gap-1 overflow-auto px-3 py-2">
  <div class="flex items-center gap-2 text-[10px] uppercase tracking-wider text-text-muted">
    <span>Sequencer</span>
    <span class="text-text-dim">·</span>
    <span>{appStore.bpm.toFixed(0)} BPM</span>
    {#if appStore.swing !== 0}
      <span class="text-text-dim">·</span>
      <span>swing {(appStore.swing * 100).toFixed(0)}%</span>
    {/if}
    {#if appStore.playing}
      <span class="text-accent">▶ step {(appStore.currentStep + 1).toString().padStart(2, "0")}</span>
    {/if}
    <span class="ml-auto text-text-dim text-[9px]">click drag = paint · alt/right-click = accent</span>
    <button
      class="ml-2 rounded border border-border bg-bg-2 px-2 py-0.5 text-[10px] text-text-dim hover:bg-bg-3 hover:text-text"
      onclick={() => appStore.clearPattern()}
    >clear</button>
  </div>
  {#each kitPads() as p (p)}
    <div class="flex items-center gap-1" data-pad-row={p}>
      <span class="w-14 flex-shrink-0 truncate text-right text-[10px] text-text-muted" title={labelFor(p)}>
        {labelFor(p)}
      </span>
      <div class="flex flex-1 gap-1">
        {#each Array(STEPS) as _, s (s)}
          <button
            class="step-cell"
            class:on={isOn(p, s)}
            class:accent={isAccent(p, s)}
            class:beat={s % 4 === 0}
            class:playing={appStore.playing && appStore.currentStep === s}
            data-step-cell="1"
            data-pad={p}
            data-step={s}
            onpointerdown={(ev) => onCellPointerDown(ev, p, s)}
            oncontextmenu={(ev) => { ev.preventDefault(); appStore.toggleAccent(p, s); }}
            aria-label={`pad ${p + 1} step ${s + 1}`}
            title={`pad ${p + 1} step ${s + 1}${isOn(p, s) ? ` (vel ${vel(p, s)})` : ""}`}
          ></button>
        {/each}
      </div>
    </div>
  {/each}
</div>

<style>
  .step-cell {
    flex: 1;
    aspect-ratio: 1 / 1;
    min-width: 18px;
    max-width: 28px;
    border-radius: 4px;
    background: var(--color-bg-2);
    border: 1px solid var(--color-border);
    transition: background 0.1s, transform 0.06s, box-shadow 0.1s;
    padding: 0;
    touch-action: none;
    user-select: none;
  }
  .step-cell.beat {
    border-color: color-mix(in oklch, var(--color-border) 60%, var(--color-text-muted));
  }
  .step-cell:hover {
    background: var(--color-bg-3);
  }
  .step-cell.on {
    background: var(--color-accent);
    border-color: var(--color-accent);
    box-shadow: 0 0 8px -2px var(--color-accent);
  }
  .step-cell.accent {
    background: var(--color-accent-warm);
    border-color: var(--color-accent-warm);
    box-shadow: 0 0 12px -1px var(--color-accent-warm);
    transform: scale(1.08);
  }
  .step-cell.playing {
    box-shadow: 0 0 14px -1px var(--color-accent-warm);
  }
  .step-cell.on.playing {
    transform: scale(1.08);
  }
</style>
