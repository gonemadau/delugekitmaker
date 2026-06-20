<script lang="ts">
  import { appStore } from "../stores/app.svelte";
  import Pad from "./Pad.svelte";
  import { Pencil } from "lucide-svelte";

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

  let editing = $state(false);
  let nameInput = $state("");

  function startEdit() {
    if (!appStore.activeKit) return;
    nameInput = appStore.activeKit.name;
    editing = true;
  }

  function commit() {
    if (!editing) return;
    appStore.renameActiveKit(nameInput);
    editing = false;
  }

  function cancel() {
    editing = false;
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Enter") commit();
    else if (e.key === "Escape") cancel();
  }
</script>

{#if !appStore.activeKit}
  <div class="flex h-full flex-col items-center justify-center gap-3 text-center">
    <h2 class="text-lg font-semibold text-text-dim">No kit open</h2>
    <p class="max-w-sm text-sm text-text-muted">
      Pick an SD card folder, then choose a kit from the left, or drop a folder of WAV files anywhere.
    </p>
  </div>
{:else}
  <div class="mb-4 flex items-baseline justify-between gap-3">
    {#if editing}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        bind:value={nameInput}
        onkeydown={onKey}
        onblur={commit}
        class="kit-name-input"
        maxlength="100"
        autofocus
      />
    {:else}
      <button
        class="kit-name"
        onclick={startEdit}
        ondblclick={startEdit}
        title="Click to rename this kit"
      >
        <span class="min-w-0 truncate">{appStore.activeKit.name || "Untitled kit"}</span>
        <Pencil class="h-3.5 w-3.5 flex-shrink-0 opacity-50 transition-opacity" />
        <span class="rename-hint">rename</span>
      </button>
    {/if}
    <span class="flex-shrink-0 text-xs text-text-muted">
      firmware {appStore.activeKit.firmware_version} · {appStore.activeKit.drums.length} drums
    </span>
  </div>

  <div class="grid w-full max-w-md grid-cols-4 gap-2">
    {#each pads() as drum, i (i)}
      <Pad index={i} {drum} />
    {/each}
  </div>
{/if}

<style>
  .kit-name {
    display: flex;
    flex: 1;
    min-width: 0;
    align-items: center;
    gap: 8px;
    padding: 4px 8px;
    margin: -4px -8px;
    border-radius: 6px;
    color: var(--color-text);
    background: transparent;
    border: 1px dashed transparent;
    border-color: color-mix(in oklch, var(--color-text-muted) 35%, transparent);
    font-size: 1.125rem;
    line-height: 1.75rem;
    font-weight: 600;
    letter-spacing: -0.01em;
    text-align: left;
    transition: background 0.15s, border-color 0.15s;
    cursor: text;
  }
  .kit-name:hover {
    background: var(--color-bg-2);
    border-color: var(--color-accent);
  }
  .kit-name:hover :global(svg) {
    opacity: 1 !important;
  }
  .rename-hint {
    font-size: 10px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--color-text-muted);
    opacity: 0;
    transition: opacity 0.15s;
  }
  .kit-name:hover .rename-hint {
    opacity: 0.9;
  }
  .kit-name-input {
    flex: 1;
    min-width: 0;
    border-radius: 6px;
    border: 1px solid var(--color-accent);
    background: var(--color-bg-2);
    color: var(--color-text);
    padding: 4px 8px;
    margin: -4px -8px;
    font-size: 1.125rem;
    line-height: 1.75rem;
    font-weight: 600;
    letter-spacing: -0.01em;
    outline: none;
    box-shadow: 0 0 0 3px color-mix(in oklch, var(--color-accent) 30%, transparent);
  }
</style>
