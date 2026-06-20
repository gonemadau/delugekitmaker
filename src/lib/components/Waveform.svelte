<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import type { PeaksResponse } from "../api";
  import { appStore } from "../stores/app.svelte";

  let { filePath, startFrames = 0, endFrames = 0, onTrimChange }: {
    filePath: string | null;
    startFrames?: number;
    endFrames?: number;
    onTrimChange?: (start: number, end: number) => void;
  } = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let peaks = $state<PeaksResponse | null>(null);
  let width = $state(640);
  let height = 120;
  let resizeObserver: ResizeObserver | undefined;
  let revealAt = $state(0);
  let raf: number | undefined;

  $effect(() => {
    if (!filePath) {
      peaks = null;
      return;
    }
    appStore.getOrFetchPeaks(filePath, Math.max(64, Math.floor(width))).then((p) => {
      peaks = p;
      revealAt = performance.now();
      requestRedraw();
    });
  });

  $effect(() => {
    // Trigger redraw when trim points change.
    void startFrames;
    void endFrames;
    requestRedraw();
  });

  function requestRedraw() {
    if (raf) cancelAnimationFrame(raf);
    raf = requestAnimationFrame(redraw);
  }

  function redraw() {
    raf = undefined;
    if (!canvas || !peaks) return;
    const dpr = window.devicePixelRatio || 1;
    const cssW = canvas.clientWidth;
    const cssH = canvas.clientHeight;
    if (canvas.width !== Math.floor(cssW * dpr)) canvas.width = Math.floor(cssW * dpr);
    if (canvas.height !== Math.floor(cssH * dpr)) canvas.height = Math.floor(cssH * dpr);
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cssW, cssH);

    const cols = peaks.min.length;
    const cw = cssW / cols;
    const mid = cssH / 2;
    const elapsed = Math.min((performance.now() - revealAt) / 600, 1);
    const revealCols = Math.floor(cols * elapsed);

    // Accent color from CSS var.
    const accent = getComputedStyle(document.documentElement).getPropertyValue("--color-accent").trim() || "oklch(0.75 0.18 280)";

    // Out-of-trim region: dim background
    const trimStartCol = Math.floor((startFrames / Math.max(1, peaks.frames)) * cols);
    const trimEndCol = endFrames > 0 ? Math.floor((endFrames / Math.max(1, peaks.frames)) * cols) : cols;

    ctx.fillStyle = "rgba(255,255,255,0.04)";
    if (trimStartCol > 0) ctx.fillRect(0, 0, trimStartCol * cw, cssH);
    if (trimEndCol < cols) ctx.fillRect(trimEndCol * cw, 0, (cols - trimEndCol) * cw, cssH);

    ctx.strokeStyle = accent;
    ctx.lineWidth = 1;
    ctx.beginPath();
    for (let i = 0; i < revealCols; i++) {
      const min = peaks.min[i];
      const max = peaks.max[i];
      const y0 = mid - max * mid * 0.95;
      const y1 = mid - min * mid * 0.95;
      const x = i * cw + cw * 0.5;
      ctx.moveTo(x, y0);
      ctx.lineTo(x, y1);
    }
    ctx.stroke();

    // Continue revealing
    if (elapsed < 1) requestRedraw();
  }

  function onPointerDown(e: PointerEvent, which: "start" | "end") {
    e.preventDefault();
    e.stopPropagation();
    if (!canvas || !peaks) return;
    const rect = canvas.getBoundingClientRect();
    function update(ev: PointerEvent) {
      if (!peaks) return;
      const x = Math.max(0, Math.min(rect.width, ev.clientX - rect.left));
      const frame = Math.round((x / rect.width) * peaks.frames);
      if (which === "start") {
        const newStart = Math.min(frame, (endFrames || peaks.frames) - 1);
        onTrimChange?.(newStart, endFrames);
      } else {
        const newEnd = Math.max(frame, startFrames + 1);
        onTrimChange?.(startFrames, newEnd);
      }
    }
    function up() {
      window.removeEventListener("pointermove", update);
      window.removeEventListener("pointerup", up);
    }
    window.addEventListener("pointermove", update);
    window.addEventListener("pointerup", up);
  }

  onMount(() => {
    if (canvas) {
      resizeObserver = new ResizeObserver((entries) => {
        for (const e of entries) {
          width = e.contentRect.width;
          requestRedraw();
        }
      });
      resizeObserver.observe(canvas);
    }
  });

  onDestroy(() => {
    resizeObserver?.disconnect();
    if (raf) cancelAnimationFrame(raf);
  });

  let startX = $derived(peaks ? `${(startFrames / Math.max(1, peaks.frames)) * 100}%` : "0%");
  let endX = $derived(
    peaks ? `${((endFrames > 0 ? endFrames : peaks.frames) / peaks.frames) * 100}%` : "100%"
  );
</script>

<div class="relative w-full" style="height: {height}px">
  <canvas
    bind:this={canvas}
    class="absolute inset-0 h-full w-full rounded-md bg-[color:var(--color-bg-2)]"
  ></canvas>

  {#if filePath && peaks}
    <!-- Trim handles -->
    <div
      class="trim-handle pointer-events-auto absolute top-0 h-full w-1 cursor-ew-resize"
      style="left: {startX}; background: var(--color-accent-warm)"
      onpointerdown={(e) => onPointerDown(e, "start")}
      aria-label="Start trim"
      tabindex="0"
      role="slider"
      aria-valuenow={startFrames}
      aria-valuemin="0"
      aria-valuemax={peaks?.frames ?? 0}
    ></div>
    <div
      class="trim-handle pointer-events-auto absolute top-0 h-full w-1 cursor-ew-resize"
      style="left: {endX}; background: var(--color-accent-warm)"
      onpointerdown={(e) => onPointerDown(e, "end")}
      aria-label="End trim"
      tabindex="0"
      role="slider"
      aria-valuenow={endFrames || (peaks?.frames ?? 0)}
      aria-valuemin="0"
      aria-valuemax={peaks?.frames ?? 0}
    ></div>
  {/if}

  {#if !filePath}
    <div class="absolute inset-0 flex items-center justify-center text-[10px] text-text-muted">
      no sample
    </div>
  {:else if !peaks}
    <div class="absolute inset-0 flex items-center justify-center text-[10px] text-text-muted">
      loading…
    </div>
  {/if}
</div>

<style>
  .trim-handle {
    transition: width 0.1s ease;
  }
  .trim-handle::before {
    content: "";
    position: absolute;
    inset: 0 -4px;
  }
  .trim-handle:hover {
    width: 3px;
  }
</style>
