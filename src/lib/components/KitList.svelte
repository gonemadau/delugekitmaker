<script lang="ts">
  import { appStore } from "../stores/app.svelte";
  import FileBrowser from "./FileBrowser.svelte";
  import { Folder, FileMusic, RefreshCw, FolderTree } from "lucide-svelte";

  type Tab = "kits" | "files";
  let tab = $state<Tab>("kits");
</script>

<div class="flex h-full flex-col">
  <div class="flex border-b border-border bg-bg-1">
    <button
      class="tab"
      class:active={tab === "kits"}
      onclick={() => (tab = "kits")}
      title="Kits on the current SD card"
    >
      <Folder class="h-3 w-3" /> Kits {appStore.kits.length ? `(${appStore.kits.length})` : ""}
    </button>
    <button
      class="tab"
      class:active={tab === "files"}
      onclick={() => (tab = "files")}
      title="Browse WAV files anywhere on disk"
    >
      <FolderTree class="h-3 w-3" /> Files
    </button>
  </div>

  {#if tab === "kits"}
    <header class="flex items-center justify-between border-b border-border px-3 py-1.5">
      <span class="text-[10px] uppercase tracking-wider text-text-muted">
        {appStore.sdRoot ? "SD card kits" : "no SD card"}
      </span>
      <button
        class="rounded p-1 text-text-dim transition-colors hover:bg-bg-2 hover:text-text disabled:opacity-50"
        onclick={() => appStore.refreshKits()}
        disabled={!appStore.sdRoot || appStore.loading}
        title="Refresh"
      >
        <RefreshCw class="h-3 w-3" />
      </button>
    </header>

    {#if !appStore.sdRoot}
      <div class="flex flex-1 items-center justify-center p-4 text-center">
        <p class="text-xs text-text-muted">Pick an SD card folder above.</p>
      </div>
    {:else if appStore.kits.length === 0}
      <div class="flex flex-1 items-center justify-center p-4 text-center">
        <p class="text-xs text-text-muted">No kits in <code class="text-accent">KITS/</code></p>
      </div>
    {:else}
      <ul class="flex flex-1 flex-col overflow-auto">
        {#each appStore.kits as kit (kit.rel_path)}
          <li>
            <button
              class="flex w-full items-center gap-2 px-3 py-2 text-left text-xs transition-colors hover:bg-bg-2"
              onclick={() => appStore.openKit(kit)}
              title={`${kit.name}\n${kit.rel_path}`}
            >
              <FileMusic class="h-3.5 w-3.5 flex-shrink-0 text-text-dim" />
              <span class="min-w-0 flex-1 truncate">{kit.name}</span>
              <span class="flex-shrink-0 text-text-muted">{(kit.size_bytes / 1024).toFixed(1)}k</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  {:else}
    <FileBrowser />
  {/if}
</div>

<style>
  .tab {
    flex: 1;
    display: flex; align-items: center; justify-content: center; gap: 4px;
    padding: 8px 4px;
    font-size: 11px;
    color: var(--color-text-muted);
    border-bottom: 2px solid transparent;
    transition: color 0.15s, border-color 0.15s, background 0.15s;
  }
  .tab:hover { color: var(--color-text); background: var(--color-bg-2); }
  .tab.active {
    color: var(--color-text);
    border-bottom-color: var(--color-accent);
  }
</style>
