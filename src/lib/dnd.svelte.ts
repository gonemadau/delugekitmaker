// Pointer-based in-app drag-and-drop.
//
// Tauri's onDragDropEvent (for OS Explorer drops) is wired at the WebView2 level
// on Windows and reliably eats HTML5 dragstart/dragover events inside the
// webview. We can't disable it without losing OS file drops, so in-app drags
// (pad <-> pad, file-browser <-> pad) are implemented with pointer events
// instead. Drop targets register a `data-pad-target="<index>"` attribute and
// the manager finds them via document.elementFromPoint on pointerup.

import { appStore } from "./stores/app.svelte";

export type DragPayload =
  | { type: "pad"; padIndex: number }
  | { type: "files"; paths: string[]; label?: string };

const DRAG_THRESHOLD_PX = 5;

class DndManager {
  payload = $state<DragPayload | null>(null);
  hoverPadIndex = $state<number | null>(null);
  pointerX = $state(0);
  pointerY = $state(0);

  /// True for ~one frame after a drop, so post-drop click events on pads can be
  /// suppressed (the browser still dispatches click on the element under the
  /// pointer after pointerup, which would otherwise audition the pad we just
  /// dropped onto).
  justDropped = false;

  private startX = 0;
  private startY = 0;
  private candidate: DragPayload | null = null;
  private listening = false;
  private safetyTimer: number | undefined;

  begin(payload: DragPayload, e: PointerEvent) {
    if (e.button !== 0) return;
    // If a previous drag is somehow still armed, tear it down before starting fresh.
    if (this.listening) this.reset();
    this.candidate = payload;
    this.startX = e.clientX;
    this.startY = e.clientY;
    this.pointerX = e.clientX;
    this.pointerY = e.clientY;
    this.listening = true;
    window.addEventListener("pointermove", this.onMove);
    window.addEventListener("pointerup", this.onUp);
    window.addEventListener("pointercancel", this.onCancel);
    window.addEventListener("keydown", this.onKey);
    // Safety: nothing should drag for longer than 30s. If it does, force-reset.
    this.safetyTimer = window.setTimeout(() => this.reset(), 30_000);
  }

  private onMove = (e: PointerEvent) => {
    this.pointerX = e.clientX;
    this.pointerY = e.clientY;
    if (!this.payload) {
      if (!this.candidate) return;
      const dx = e.clientX - this.startX;
      const dy = e.clientY - this.startY;
      if (Math.hypot(dx, dy) < DRAG_THRESHOLD_PX) return;
      this.payload = this.candidate;
    }
    this.hoverPadIndex = this.targetAt(e.clientX, e.clientY);
  };

  private onUp = (e: PointerEvent) => {
    const hadPayload = this.payload !== null;
    let dispatchTarget: number | null = null;
    try {
      if (hadPayload) {
        dispatchTarget = this.targetAt(e.clientX, e.clientY);
      }
    } catch (err) {
      console.warn("dnd targetAt failed", err);
    }
    // Tear listeners down BEFORE the synchronous dispatch so a thrown error
    // inside dispatch can't leave us stuck listening forever.
    this.stopListening();
    if (hadPayload && dispatchTarget !== null) {
      try {
        this.dispatch(dispatchTarget);
      } catch (err) {
        console.warn("dnd dispatch failed", err);
      }
      // Suppress the click event that the browser will dispatch right after.
      this.justDropped = true;
      window.setTimeout(() => (this.justDropped = false), 0);
    }
    this.payload = null;
    this.candidate = null;
    this.hoverPadIndex = null;
  };

  private onCancel = () => {
    this.reset();
  };

  private onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") this.reset();
  };

  private stopListening() {
    this.listening = false;
    if (this.safetyTimer !== undefined) {
      window.clearTimeout(this.safetyTimer);
      this.safetyTimer = undefined;
    }
    window.removeEventListener("pointermove", this.onMove);
    window.removeEventListener("pointerup", this.onUp);
    window.removeEventListener("pointercancel", this.onCancel);
    window.removeEventListener("keydown", this.onKey);
  }

  private reset() {
    this.stopListening();
    this.payload = null;
    this.candidate = null;
    this.hoverPadIndex = null;
  }

  private targetAt(x: number, y: number): number | null {
    const el = document.elementFromPoint(x, y) as HTMLElement | null;
    if (!el) return null;
    const target = el.closest("[data-pad-target]") as HTMLElement | null;
    if (!target) return null;
    const v = Number(target.dataset.padTarget);
    return Number.isFinite(v) ? v : null;
  }

  private dispatch(targetPadIdx: number) {
    const p = this.payload;
    if (!p) return;
    if (p.type === "pad") {
      if (p.padIndex !== targetPadIdx) {
        appStore.swapPads(p.padIndex, targetPadIdx);
      }
      return;
    }
    if (p.type === "files" && p.paths.length > 0) {
      // Only assign the first path synchronously; defer the rest so the heavy
      // reactivity cascade for one drop doesn't run inside a long sync chain.
      appStore.assignSampleToPad(targetPadIdx, p.paths[0]);
      if (p.paths.length > 1) {
        const rest = p.paths.slice(1);
        queueMicrotask(() => void appStore.addFilesToKit(rest));
      }
    }
  }
}

export const dnd = new DndManager();
