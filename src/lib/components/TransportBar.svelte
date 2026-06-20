<script lang="ts">
  import { appStore } from "../stores/app.svelte";
  import { Play, Square } from "lucide-svelte";

  let bpmInput = $state(appStore.bpm.toFixed(0));

  function onBpmChange() {
    const n = Number(bpmInput);
    if (Number.isFinite(n) && n >= 40 && n <= 240) {
      appStore.setBpm(n);
    } else {
      bpmInput = appStore.bpm.toFixed(0);
    }
  }
</script>

<div class="flex items-center gap-3">
  {#if appStore.playing}
    <button
      class="flex h-8 w-8 items-center justify-center rounded-md border bg-[color:var(--color-accent)] text-bg-0 transition-colors hover:opacity-90"
      onclick={() => appStore.stop()}
      title="Stop (Esc)"
    >
      <Square class="h-3.5 w-3.5" />
    </button>
  {:else}
    <button
      class="flex h-8 w-8 items-center justify-center rounded-md border border-border bg-bg-2 text-text-dim transition-colors hover:bg-bg-3 hover:text-text"
      onclick={() => appStore.play()}
      title="Play (Space)"
    >
      <Play class="h-3.5 w-3.5" />
    </button>
  {/if}

  <label class="flex items-center gap-1 text-xs text-text-muted">
    BPM
    <input
      type="number"
      min="40"
      max="240"
      step="1"
      bind:value={bpmInput}
      onchange={onBpmChange}
      class="w-14 rounded border border-border bg-bg-2 px-2 py-0.5 text-xs text-text"
    />
  </label>

  <label class="flex items-center gap-2 text-xs text-text-muted">
    Swing
    <input
      type="range"
      min="-50"
      max="50"
      step="1"
      value={Math.round(appStore.swing * 100)}
      oninput={(e) => appStore.setSwing(+e.currentTarget.value / 100)}
      class="swing-slider"
      title="Sixteenth-note swing"
    />
    <span class="w-8 text-right text-[10px] text-text-dim tabular-nums">{Math.round(appStore.swing * 100)}%</span>
  </label>

  {#if appStore.currentStep >= 0}
    <span class="text-[10px] text-text-dim">step {appStore.currentStep + 1}/16</span>
  {/if}
</div>

<style>
  .swing-slider { accent-color: var(--color-accent); width: 100px; }
</style>
