<script lang="ts">
  import { dnd } from "../dnd.svelte";
  import { Music4, GripVertical, Files } from "lucide-svelte";

  let label = $derived(() => {
    const p = dnd.payload;
    if (!p) return "";
    if (p.type === "pad") return `Pad ${p.padIndex + 1}`;
    if (p.type === "files") return p.label ?? `${p.paths.length} file${p.paths.length > 1 ? "s" : ""}`;
    return "";
  });
</script>

{#if dnd.payload}
  <div
    class="ghost"
    style="transform: translate({dnd.pointerX + 12}px, {dnd.pointerY + 8}px)"
  >
    {#if dnd.payload.type === "pad"}
      <GripVertical class="h-3 w-3" />
    {:else if dnd.payload.type === "files" && dnd.payload.paths.length > 1}
      <Files class="h-3 w-3" />
    {:else}
      <Music4 class="h-3 w-3" />
    {/if}
    <span>{label()}</span>
  </div>
{/if}

<style>
  .ghost {
    position: fixed; top: 0; left: 0;
    z-index: 9999;
    display: flex; align-items: center; gap: 6px;
    padding: 4px 8px;
    background: color-mix(in oklch, var(--color-bg-1) 92%, transparent);
    border: 1px solid var(--color-accent);
    border-radius: 6px;
    box-shadow: 0 8px 24px -6px var(--color-accent);
    color: var(--color-text);
    font-size: 11px;
    pointer-events: none;
    max-width: 240px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    backdrop-filter: blur(8px);
  }
  .ghost span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
