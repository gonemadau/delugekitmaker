<script lang="ts">
  import { appStore } from "../stores/app.svelte";
  import { FolderOpen, Disc3, Save, Wand2 } from "lucide-svelte";

  function abbreviate(p: string): string {
    if (p.length <= 32) return p;
    return "…" + p.slice(-30);
  }

  let saving = $state(false);
  let savedLabel = $state<string | null>(null);

  async function onSave() {
    if (saving) return;
    saving = true;
    savedLabel = null;
    const r = await appStore.saveActiveKit();
    if (r) {
      savedLabel = "saved";
      setTimeout(() => (savedLabel = null), 2000);
    }
    saving = false;
  }
</script>

<div class="flex h-12 items-center justify-between border-b border-border bg-bg-1 px-4">
  <div class="flex items-center gap-3">
    <div class="flex h-7 w-7 items-center justify-center rounded-md" style="background: var(--color-accent);">
      <Disc3 class="h-4 w-4" style="color: var(--color-bg-0)" />
    </div>
    <h1 class="text-sm font-semibold tracking-tight">Deluge Kit Maker</h1>
  </div>

  <div class="flex items-center gap-2">
    {#if savedLabel}
      <span class="text-[11px] text-[color:var(--color-accent)]">{savedLabel}</span>
    {/if}
    <button
      class="flex items-center gap-2 rounded-md border border-border bg-bg-2 px-3 py-1.5 text-xs text-text-dim transition-colors hover:bg-bg-3 hover:text-text disabled:opacity-50"
      onclick={() => appStore.autoArrange()}
      disabled={!appStore.activeKit}
      title="Auto-arrange pads by category (kicks → snare → rim → hats → toms → other)"
    >
      <Wand2 class="h-3.5 w-3.5" />
      Auto-arrange
    </button>
    <button
      class="flex items-center gap-2 rounded-md border border-border bg-bg-2 px-3 py-1.5 text-xs text-text-dim transition-colors hover:bg-bg-3 hover:text-text disabled:opacity-50"
      onclick={onSave}
      disabled={!appStore.activeKit || !appStore.sdRoot || saving}
      title="Save kit (Ctrl+S)"
    >
      <Save class="h-3.5 w-3.5" />
      Save
    </button>
    <button
      class="flex items-center gap-2 rounded-md border border-border bg-bg-2 px-3 py-1.5 text-xs text-text-dim transition-colors hover:bg-bg-3 hover:text-text"
      onclick={() => appStore.pickAndSetRoot()}
      disabled={appStore.loading}
    >
      <FolderOpen class="h-3.5 w-3.5" />
      {appStore.sdRoot ? abbreviate(appStore.sdRoot.path) : "Select SD card folder"}
    </button>
  </div>
</div>
