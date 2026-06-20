<script lang="ts">
  import { onMount } from "svelte";
  import { appStore } from "../stores/app.svelte";
  import { api } from "../api";
  import { Folder, FolderOpen, FileAudio, ArrowUp, RefreshCw, Home, HardDrive } from "lucide-svelte";

  let loading = $state(false);
  let drives = $state<string[]>([]);
  let pathInput = $state("");
  let editingPath = $state(false);

  async function go(path?: string) {
    loading = true;
    await appStore.browseTo(path);
    pathInput = appStore.browseListing?.abs_path ?? "";
    loading = false;
  }

  async function selectDrive(d: string) {
    await go(d);
  }

  function onPathEditKey(e: KeyboardEvent) {
    if (e.key === "Enter") {
      editingPath = false;
      void go(pathInput);
    } else if (e.key === "Escape") {
      editingPath = false;
      pathInput = appStore.browseListing?.abs_path ?? "";
    }
  }

  function onWavDragStart(e: DragEvent, absPath: string) {
    e.dataTransfer?.setData("application/x-dkm-file", absPath);
    e.dataTransfer?.setData("text/plain", absPath);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = "copy";
  }

  function onWavClick(absPath: string) {
    if (appStore.selectedPad >= 0) {
      appStore.assignSampleToPad(appStore.selectedPad, absPath);
    }
  }

  function basename(p: string): string {
    const ix = Math.max(p.lastIndexOf("/"), p.lastIndexOf("\\"));
    return ix >= 0 ? p.slice(ix + 1) || p : p;
  }

  function currentDriveOf(path: string | undefined | null): string | null {
    if (!path) return null;
    const m = /^([A-Za-z]:)/.exec(path);
    return m ? `${m[1]}\\` : null;
  }

  onMount(async () => {
    try {
      drives = await api.listDrives();
    } catch (e) {
      console.warn("list_drives failed", e);
    }
    if (!appStore.browseListing) await go();
    else pathInput = appStore.browseListing.abs_path;
  });
</script>

<div class="flex h-full flex-col">
  <header class="flex items-center gap-1 border-b border-border px-2 py-1.5">
    <button
      class="rounded p-1 text-text-dim hover:bg-bg-2 hover:text-text disabled:opacity-40"
      onclick={() => go(appStore.browseListing?.parent ?? undefined)}
      disabled={!appStore.browseListing?.parent}
      title="Up one folder"
    >
      <ArrowUp class="h-3.5 w-3.5" />
    </button>
    <button
      class="rounded p-1 text-text-dim hover:bg-bg-2 hover:text-text"
      onclick={() => go()}
      title="Home / default"
    >
      <Home class="h-3.5 w-3.5" />
    </button>
    <button
      class="rounded p-1 text-text-dim hover:bg-bg-2 hover:text-text disabled:opacity-40"
      onclick={() => go(appStore.browseListing?.abs_path)}
      disabled={!appStore.browseListing || loading}
      title="Refresh"
    >
      <RefreshCw class="h-3.5 w-3.5 {loading ? 'animate-spin' : ''}" />
    </button>

    {#if drives.length > 1}
      <select
        class="drive-select"
        value={currentDriveOf(appStore.browseListing?.abs_path) ?? drives[0]}
        onchange={(e) => selectDrive(e.currentTarget.value)}
        title="Switch drive"
      >
        {#each drives as d (d)}
          <option value={d}>{d.replace("\\", "")}</option>
        {/each}
      </select>
    {:else if drives.length === 1}
      <span class="flex items-center gap-1 px-1 text-[10px] text-text-muted">
        <HardDrive class="h-3 w-3" /> {drives[0]}
      </span>
    {/if}
  </header>

  <!-- Editable path bar -->
  <div class="flex items-center gap-1 border-b border-border px-2 py-1">
    {#if editingPath}
      <!-- svelte-ignore a11y_autofocus -->
      <input
        type="text"
        bind:value={pathInput}
        onkeydown={onPathEditKey}
        onblur={() => (editingPath = false)}
        class="flex-1 rounded border border-border bg-bg-2 px-2 py-0.5 text-[10px] text-text outline-none focus:border-[color:var(--color-accent)]"
        autofocus
      />
    {:else}
      <button
        class="min-w-0 flex-1 truncate text-left text-[10px] text-text-muted hover:text-text"
        onclick={() => { editingPath = true; pathInput = appStore.browseListing?.abs_path ?? ""; }}
        title={appStore.browseListing?.abs_path ?? ""}
      >
        {appStore.browseListing ? appStore.browseListing.abs_path : "—"}
      </button>
    {/if}
  </div>

  {#if !appStore.browseListing}
    <div class="flex flex-1 items-center justify-center p-4 text-center">
      <p class="text-xs text-text-muted">{loading ? "loading…" : "pick a folder"}</p>
    </div>
  {:else if appStore.browseListing.entries.length === 0}
    <div class="flex flex-1 items-center justify-center p-4 text-center">
      <p class="text-xs text-text-muted">no folders or .wav files here</p>
    </div>
  {:else}
    <ul class="flex flex-1 flex-col overflow-auto">
      {#each appStore.browseListing.entries as e (e.abs_path)}
        {#if e.kind === "dir"}
          <li>
            <button
              class="group flex w-full items-center gap-2 px-2 py-1 text-left text-xs transition-colors hover:bg-bg-2"
              onclick={() => go(e.abs_path)}
              title={e.abs_path}
            >
              <Folder class="h-3.5 w-3.5 flex-shrink-0 text-text-dim group-hover:hidden" />
              <FolderOpen class="hidden h-3.5 w-3.5 flex-shrink-0 text-[color:var(--color-accent)] group-hover:block" />
              <span class="min-w-0 flex-1 truncate">{e.name}</span>
            </button>
          </li>
        {:else}
          <li>
            <button
              class="flex w-full items-center gap-2 px-2 py-1 text-left text-xs transition-colors hover:bg-bg-2"
              draggable="true"
              ondragstart={(ev) => onWavDragStart(ev, e.abs_path)}
              onclick={() => onWavClick(e.abs_path)}
              title={`${e.name}\n${e.abs_path}`}
            >
              <FileAudio class="h-3.5 w-3.5 flex-shrink-0 text-text-muted" />
              <span class="min-w-0 flex-1 truncate">{e.name}</span>
              <span class="flex-shrink-0 text-text-muted">{(e.size_bytes / 1024).toFixed(0)}k</span>
            </button>
          </li>
        {/if}
      {/each}
    </ul>
  {/if}

  {#if appStore.selectedPad >= 0}
    <footer class="border-t border-border bg-bg-1 px-2 py-1 text-[10px] text-text-muted">
      Click a .wav to assign to pad {appStore.selectedPad + 1}, or drag onto any pad.
    </footer>
  {/if}
</div>

<style>
  .drive-select {
    background: var(--color-bg-2);
    border: 1px solid var(--color-border);
    color: var(--color-text);
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 4px;
    margin-left: 4px;
    cursor: pointer;
  }
  .drive-select:hover { background: var(--color-bg-3); }
  .drive-select:focus { outline: 1px solid var(--color-accent); }
</style>
