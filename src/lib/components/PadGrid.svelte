<script lang="ts">
  import { appStore } from "../stores/app.svelte";
  import Pad from "./Pad.svelte";

  const PAD_COUNT = 16;

  let pads = $derived(() => {
    const kit = appStore.activeKit;
    if (!kit) return Array(PAD_COUNT).fill(null);
    const out = Array(PAD_COUNT).fill(null);
    for (let i = 0; i < PAD_COUNT && i < kit.drums.length; i++) {
      out[i] = kit.drums[i];
    }
    return out;
  });
</script>

{#if !appStore.activeKit}
  <div class="flex h-full flex-col items-center justify-center gap-3 text-center">
    <h2 class="text-lg font-semibold text-text-dim">No kit open</h2>
    <p class="max-w-sm text-sm text-text-muted">
      Pick an SD card folder, then choose a kit from the left, or drop a folder of WAV files anywhere.
    </p>
  </div>
{:else}
  <div class="mb-4 flex items-baseline justify-between">
    <h2 class="text-lg font-semibold tracking-tight">{appStore.activeKit.name}</h2>
    <span class="text-xs text-text-muted">
      firmware {appStore.activeKit.firmware_version} · {appStore.activeKit.drums.length} drums
    </span>
  </div>

  <div class="grid w-full max-w-md grid-cols-4 gap-2">
    {#each pads() as drum, i (i)}
      <Pad index={i} {drum} />
    {/each}
  </div>
{/if}
