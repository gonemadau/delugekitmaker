<script lang="ts">
  import { appStore } from "../stores/app.svelte";
  import { Plus, X } from "lucide-svelte";

  async function addNewKit() {
    const n = `New Kit ${appStore.openKits.length + 1}`;
    await appStore.newKit(n);
  }
</script>

<div class="flex h-10 items-center gap-1 border-b border-border bg-bg-1 px-2">
  {#each appStore.openKits as kit, i (i)}
    <div
      role="tab"
      tabindex="0"
      title={appStore.isDirty(i) ? `${kit.name} (unsaved changes)` : kit.name}
      class="group flex max-w-[180px] min-w-0 cursor-pointer items-center gap-2 rounded-t-md border-b-2 px-3 py-1 text-xs transition-colors"
      class:border-transparent={appStore.activeKitIndex !== i}
      class:text-text-dim={appStore.activeKitIndex !== i}
      class:bg-bg-2={appStore.activeKitIndex === i}
      class:text-text={appStore.activeKitIndex === i}
      style={appStore.activeKitIndex === i ? "border-color: var(--color-accent)" : ""}
      onclick={() => appStore.setActiveKit(i)}
      onkeydown={(e) => { if (e.key === "Enter" || e.key === " ") appStore.setActiveKit(i); }}
    >
      {#if appStore.isDirty(i)}
        <span class="dirty-dot" aria-label="Unsaved changes"></span>
      {/if}
      <span class="min-w-0 truncate">{kit.name}</span>
      <button
        class="rounded p-0.5 opacity-0 transition-opacity hover:bg-bg-3 group-hover:opacity-100"
        onclick={(e) => { e.stopPropagation(); appStore.closeKit(i); }}
        aria-label="Close kit"
      >
        <X class="h-3 w-3" />
      </button>
    </div>
  {/each}

  <button
    class="ml-1 flex items-center gap-1 rounded-md px-2 py-1 text-xs text-text-dim transition-colors hover:bg-bg-2 hover:text-text"
    onclick={addNewKit}
  >
    <Plus class="h-3 w-3" />
    New
  </button>
</div>

<style>
  .dirty-dot {
    width: 6px;
    height: 6px;
    border-radius: 999px;
    background: var(--color-accent-warm);
    box-shadow: 0 0 6px var(--color-accent-warm);
    flex-shrink: 0;
  }
</style>
