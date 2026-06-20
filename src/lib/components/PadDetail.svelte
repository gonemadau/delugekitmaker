<script lang="ts">
  import { appStore } from "../stores/app.svelte";
  import Waveform from "./Waveform.svelte";
  import { Play, Square, RotateCcw } from "lucide-svelte";

  let selectedPad = $derived(appStore.selectedPad);
  let kit = $derived(appStore.activeKit);
  let drum = $derived(
    kit && selectedPad >= 0 && selectedPad < kit.drums.length ? kit.drums[selectedPad] : null
  );
  let file = $derived(drum?.osc1?.file_name ?? null);
  let rt = $derived(selectedPad >= 0 ? appStore.padRuntime_for(selectedPad) : null);
  let sourceFrames = $derived(drum?.osc1?.end_samples ?? 0);

  function fmt(n: number): string {
    return n.toFixed(2);
  }
  function fmtFrames(n: number): string {
    return n.toLocaleString();
  }

  function onTrimChange(start: number, end: number) {
    if (selectedPad < 0) return;
    appStore.updatePadRuntime(selectedPad, { start_frames: start, end_frames: end });
  }

  function resetParams() {
    if (selectedPad < 0) return;
    appStore.updatePadRuntime(selectedPad, {
      volume: 0.85,
      pan: 0,
      pitch_semitones: 0,
      reverse: false,
      start_frames: 0,
      end_frames: 0,
    });
  }
</script>

<div class="flex h-full flex-col overflow-auto p-4">
  <header class="mb-3 flex items-center justify-between border-b border-border pb-2">
    <h2 class="text-xs font-semibold uppercase tracking-wider text-text-dim">Pad detail</h2>
    {#if selectedPad >= 0}
      <span class="text-[10px] text-text-muted">pad {String(selectedPad + 1).padStart(2, "0")}</span>
    {/if}
  </header>

  {#if !kit}
    <p class="text-xs text-text-muted">No kit open.</p>
  {:else if selectedPad < 0 || !drum}
    <p class="text-xs text-text-muted">Click a pad to inspect it.</p>
  {:else}
    <div class="mb-2">
      <h3 class="text-sm font-semibold tracking-tight">{drum.name || `Pad ${selectedPad + 1}`}</h3>
      <p class="mt-0.5 truncate text-[10px] text-text-muted">{file ?? "no sample"}</p>
    </div>

    <Waveform
      filePath={file}
      startFrames={rt?.start_frames ?? 0}
      endFrames={rt?.end_frames ?? 0}
      {onTrimChange}
    />

    <div class="mt-3 flex items-center gap-2">
      <button
        class="flex h-8 flex-1 items-center justify-center gap-2 rounded-md border border-border bg-bg-2 text-xs text-text-dim hover:bg-bg-3 hover:text-text"
        onclick={() => appStore.auditionPad(selectedPad)}
        disabled={!file}
      >
        <Play class="h-3 w-3" /> audition
      </button>
      <button
        class="flex h-8 w-8 items-center justify-center rounded-md border border-border bg-bg-2 text-xs text-text-dim hover:bg-bg-3 hover:text-text"
        onclick={() => appStore.stopAll()}
        title="stop all"
      >
        <Square class="h-3 w-3" />
      </button>
      <button
        class="flex h-8 w-8 items-center justify-center rounded-md border border-border bg-bg-2 text-xs text-text-dim hover:bg-bg-3 hover:text-text"
        onclick={resetParams}
        title="reset params"
      >
        <RotateCcw class="h-3 w-3" />
      </button>
    </div>

    <div class="mt-4 grid grid-cols-2 gap-3 text-[11px]">
      <label class="flex flex-col gap-1">
        <span class="text-text-muted">Volume</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={rt?.volume ?? 0.85}
          oninput={(e) => appStore.updatePadRuntime(selectedPad, { volume: +e.currentTarget.value })}
        />
        <span class="text-right text-text-dim">{fmt(rt?.volume ?? 0)}</span>
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-text-muted">Pan</span>
        <input
          type="range"
          min="-1"
          max="1"
          step="0.01"
          value={rt?.pan ?? 0}
          oninput={(e) => appStore.updatePadRuntime(selectedPad, { pan: +e.currentTarget.value })}
        />
        <span class="text-right text-text-dim">{fmt(rt?.pan ?? 0)}</span>
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-text-muted">Pitch (semitones)</span>
        <input
          type="range"
          min="-24"
          max="24"
          step="1"
          value={rt?.pitch_semitones ?? 0}
          oninput={(e) => appStore.updatePadRuntime(selectedPad, { pitch_semitones: +e.currentTarget.value })}
        />
        <span class="text-right text-text-dim">{rt?.pitch_semitones ?? 0}</span>
      </label>

      <label class="flex items-center gap-2 self-end pt-2">
        <input
          type="checkbox"
          checked={rt?.reverse ?? false}
          onchange={(e) => appStore.updatePadRuntime(selectedPad, { reverse: e.currentTarget.checked })}
        />
        <span class="text-text-dim">Reverse</span>
      </label>
    </div>

    <div class="mt-4 border-t border-border pt-3 text-[10px] text-text-muted">
      <p>start frame: {fmtFrames(rt?.start_frames ?? 0)}</p>
      <p>end frame: {fmtFrames((rt?.end_frames ?? 0) || sourceFrames)}</p>
    </div>
  {/if}
</div>

<style>
  input[type="range"] {
    accent-color: var(--color-accent);
    width: 100%;
  }
  input[type="checkbox"] {
    accent-color: var(--color-accent);
  }
</style>
