<script lang="ts">
  import { onMount } from "svelte";
  import { appStore } from "../stores/app.svelte";
  import { api } from "../api";
  import { dnd } from "../dnd.svelte";
  import { Folder, FolderOpen, FileAudio, ArrowUp, RefreshCw, Home, HardDrive, Plus } from "lucide-svelte";

  let loading = $state(false);
  let drives = $state<string[]>([]);
  let pathInput = $state("");
  let editingPath = $state(false);

  // Multi-select state. The set holds abs_path of WAV rows currently selected.
  let selection = $state<Set<string>>(new Set());
  let lastClickedIdx = $state<number>(-1);

  async function go(path?: string) {
    loading = true;
    selection = new Set();
    lastClickedIdx = -1;
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

  function onWavPointerDown(e: PointerEvent, absPath: string) {
    // Don't start drag from the + button.
    if ((e.target as Element).closest(".add-btn")) return;
    if (e.button !== 0) return;
    // If row is part of a multi-selection, drag the whole selection.
    const paths: string[] = selection.has(absPath) && selection.size > 1
      ? Array.from(selection)
      : [absPath];
    dnd.begin({ type: "files", paths, label: paths.length > 1 ? `${paths.length} files` : basename(paths[0]) }, e);
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

  // -- Click behaviour --
  // Click on a row = audition (preview).
  // Ctrl/Cmd+click = toggle in selection.
  // Shift+click = range select.
  // Plain click also resets selection to just this item (so + button adds one).
  function onRowClick(e: MouseEvent, absPath: string, rowIdx: number, wavRows: string[]) {
    // Suppress click if user just finished a drag.
    if (dnd.payload !== null || dnd.justDropped) return;
    if (e.ctrlKey || e.metaKey) {
      const next = new Set(selection);
      if (next.has(absPath)) next.delete(absPath);
      else next.add(absPath);
      selection = next;
      lastClickedIdx = rowIdx;
      return; // no preview when modifier-clicking
    }
    if (e.shiftKey && lastClickedIdx >= 0) {
      const lo = Math.min(lastClickedIdx, rowIdx);
      const hi = Math.max(lastClickedIdx, rowIdx);
      const next = new Set(selection);
      for (let i = lo; i <= hi; i++) next.add(wavRows[i]);
      selection = next;
      return;
    }
    // Plain click: replace selection with just this row, and preview.
    selection = new Set([absPath]);
    lastClickedIdx = rowIdx;
    void appStore.auditionFile(absPath);
  }

  function onAddRow(e: MouseEvent, absPath: string) {
    e.stopPropagation();
    // If multi-selected and this is one of them, add the whole selection.
    const paths = selection.has(absPath) && selection.size > 1
      ? Array.from(selection)
      : [absPath];
    void appStore.addFilesToKit(paths);
  }

  function onAddSelected() {
    if (selection.size === 0) return;
    void appStore.addFilesToKit(Array.from(selection));
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

  // Pre-compute the linear list of wav abs_paths for shift-range selection.
  let wavRows = $derived(
    (appStore.browseListing?.entries ?? [])
      .filter((e) => e.kind === "wav")
      .map((e) => e.abs_path)
  );
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
      {#each appStore.browseListing.entries as e, i (e.abs_path)}
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
          {@const wavIdx = wavRows.indexOf(e.abs_path)}
          {@const selected = selection.has(e.abs_path)}
          <li>
            <div
              class="row group flex w-full items-center gap-2 px-2 py-1 text-left text-xs transition-colors hover:bg-bg-2"
              class:selected
              role="button"
              tabindex="0"
              onpointerdown={(ev) => onWavPointerDown(ev, e.abs_path)}
              onclick={(ev) => onRowClick(ev, e.abs_path, wavIdx, wavRows)}
              onkeydown={(ev) => {
                if (ev.key === "Enter") void appStore.auditionFile(e.abs_path);
              }}
              title={`${e.name}\n${e.abs_path}\n\nclick = audition · + = add to kit · drag onto a pad`}
            >
              <FileAudio class="h-3.5 w-3.5 flex-shrink-0 text-text-muted" />
              <span class="min-w-0 flex-1 truncate">{e.name}</span>
              <span class="flex-shrink-0 text-text-muted">{(e.size_bytes / 1024).toFixed(0)}k</span>
              <button
                class="add-btn"
                onclick={(ev) => onAddRow(ev, e.abs_path)}
                title={selection.has(e.abs_path) && selection.size > 1 ? `Add ${selection.size} selected` : "Add to kit"}
                aria-label="Add to kit"
              >
                <Plus class="h-3 w-3" />
              </button>
            </div>
          </li>
        {/if}
      {/each}
    </ul>
  {/if}

  {#if selection.size > 1}
    <footer class="flex items-center gap-2 border-t border-border bg-bg-1 px-2 py-1.5 text-[10px] text-text-muted">
      <span>{selection.size} selected</span>
      <button
        class="ml-auto rounded border border-border bg-bg-2 px-2 py-0.5 text-[10px] text-text hover:bg-bg-3"
        onclick={onAddSelected}
      >Add to kit</button>
      <button
        class="rounded border border-border bg-bg-2 px-2 py-0.5 text-[10px] text-text-dim hover:bg-bg-3"
        onclick={() => (selection = new Set())}
      >clear</button>
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

  .row {
    cursor: default;
    user-select: none;
    touch-action: none;
  }
  .row.selected {
    background: color-mix(in oklch, var(--color-accent) 22%, transparent);
  }
  .add-btn {
    display: flex; align-items: center; justify-content: center;
    height: 18px; width: 18px;
    border-radius: 5px;
    background: color-mix(in oklch, var(--color-bg-1) 80%, transparent);
    color: var(--color-text-muted);
    opacity: 0;
    transition: opacity 0.12s, background 0.12s, color 0.12s;
    border: 1px solid var(--color-border);
    padding: 0;
  }
  .row:hover .add-btn,
  .row.selected .add-btn { opacity: 1; }
  .add-btn:hover {
    background: var(--color-accent);
    color: var(--color-bg-0);
    border-color: var(--color-accent);
  }
</style>
