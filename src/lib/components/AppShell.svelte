<script lang="ts">
  import { onMount } from "svelte";
  import TitleBar from "./TitleBar.svelte";
  import KitTabs from "./KitTabs.svelte";
  import KitList from "./KitList.svelte";
  import PadGrid from "./PadGrid.svelte";
  import PadDetail from "./PadDetail.svelte";
  import StatusBar from "./StatusBar.svelte";
  import SequencerStrip from "./SequencerStrip.svelte";
  import TransportBar from "./TransportBar.svelte";
  import DragGhost from "./DragGhost.svelte";
  import { appStore } from "../stores/app.svelte";
  import { api } from "../api";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { getCurrentWebview } from "@tauri-apps/api/webview";

  let unlistenStep: UnlistenFn | null = null;
  let unlistenDrop: UnlistenFn | null = null;
  let dragHover = $state(false);

  // Pad keyboard shortcuts: 4x4 layout matching the on-screen grid.
  // Row 0 (top): 1 2 3 4
  // Row 1:       Q W E R
  // Row 2:       A S D F
  // Row 3:       Z X C V
  const PAD_KEYS = ["1", "2", "3", "4", "q", "w", "e", "r", "a", "s", "d", "f", "z", "x", "c", "v"];

  function onKeyDown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    const k = e.key.toLowerCase();
    const idx = PAD_KEYS.indexOf(k);
    if (idx >= 0) {
      e.preventDefault();
      appStore.auditionPad(idx);
      return;
    }
    if (k === " " || k === "spacebar") {
      e.preventDefault();
      if (appStore.playing) appStore.stop();
      else appStore.play();
      return;
    }
    if (k === "escape") {
      appStore.stopAll();
      appStore.stop();
    }
  }

  // Warn the user if they try to close the window while any kit has unsaved
  // changes. The browser will show its native "Leave site?" dialog.
  function onBeforeUnload(e: BeforeUnloadEvent) {
    if (appStore.anyDirty) {
      e.preventDefault();
      e.returnValue = "";
    }
  }

  onMount(async () => {
    await appStore.refreshAudioInfo();
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("beforeunload", onBeforeUnload);

    // Listen for engine step events to drive the sequencer step indicator.
    unlistenStep = await listen<{ step: number }>("engine://step", (e) => {
      appStore.currentStep = e.payload.step;
    });

    // Tauri 2 file drop integration.
    try {
      unlistenDrop = await getCurrentWebview().onDragDropEvent(async (e) => {
        if (e.payload.type === "enter" || e.payload.type === "over") {
          dragHover = true;
        } else if (e.payload.type === "leave") {
          dragHover = false;
        } else if (e.payload.type === "drop") {
          dragHover = false;
          const paths: string[] = (e.payload as any).paths ?? [];
          if (paths.length === 0) return;
          // If a single path that's a folder, use its basename as kit name.
          const guessName = paths.length === 1
            ? paths[0].split(/[\\/]/).pop()
            : undefined;
          await appStore.importDroppedPaths(paths, guessName);
        }
      });
    } catch (err) {
      console.warn("drop init failed", err);
    }

    // Ctrl+S to save
    window.addEventListener("keydown", (e) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        appStore.saveActiveKit();
      }
    });

    // If the Rust side auto-loaded an SD root via DKM_AUTO_SD, populate the UI.
    try {
      const existing = await api.getSdRoot();
      if (existing) {
        appStore.sdRoot = existing;
        await appStore.refreshKits();
        // Auto-open the first kit so the pad grid is populated on launch.
        if (appStore.kits.length > 0 && appStore.openKits.length === 0) {
          await appStore.openKit(appStore.kits[0]);
        }
      }
    } catch (e) {
      console.warn("auto-load skipped", e);
    }

    // Dev convenience: when ?demo=1 is in the URL hash, automatically play a
    // kick/snare/hat pattern once. Useful for autonomous self-tests.
    if (window.location.search.includes("demo=1")) {
      setTimeout(async () => {
        const kit = appStore.activeKit;
        if (!kit) return;
        console.log("[demo] auditioning pads 0..2");
        for (const i of [0, 1, 2, 0, 2, 1, 2, 0]) {
          appStore.auditionPad(i);
          await new Promise((r) => setTimeout(r, 250));
        }
      }, 800);
    }
  });
</script>

<div
  class="grid h-screen w-screen grid-rows-[auto_auto_1fr_auto] overflow-hidden bg-bg-0 text-text"
  class:drag-hover={dragHover}
>
  <TitleBar />
  <KitTabs />

  <div class="grid grid-cols-[260px_1fr_320px] overflow-hidden border-t border-border">
    <aside class="border-r border-border overflow-y-auto bg-bg-1">
      <KitList />
    </aside>

    <main class="flex flex-col overflow-hidden">
      <div class="flex-1 overflow-auto p-6">
        <PadGrid />
      </div>
      <div class="border-t border-border bg-bg-1">
        <SequencerStrip />
      </div>
      <div class="border-t border-border bg-bg-0 px-4 py-2">
        <TransportBar />
      </div>
    </main>

    <aside class="border-l border-border overflow-y-auto bg-bg-1">
      <PadDetail />
    </aside>
  </div>

  <StatusBar />
</div>

<DragGhost />

{#if appStore.error}
  <div class="fixed bottom-12 right-6 max-w-md rounded-lg border border-[var(--color-accent-danger)] bg-bg-2 px-4 py-3 shadow-2xl">
    <p class="text-sm text-text">{appStore.error}</p>
    <button class="mt-2 text-xs text-text-dim hover:text-text" onclick={() => (appStore.error = null)}>dismiss</button>
  </div>
{/if}

<style>
  :global(.bg-bg-0) { background: var(--color-bg-0); }
  :global(.bg-bg-1) { background: var(--color-bg-1); }
  :global(.bg-bg-2) { background: var(--color-bg-2); }
  :global(.bg-bg-3) { background: var(--color-bg-3); }
  :global(.text-text) { color: var(--color-text); }
  :global(.text-text-dim) { color: var(--color-text-dim); }
  :global(.text-text-muted) { color: var(--color-text-muted); }
  :global(.border-border) { border-color: var(--color-border); }
  :global(.text-accent) { color: var(--color-accent); }

  .drag-hover {
    outline: 2px dashed var(--color-accent);
    outline-offset: -8px;
  }
</style>
