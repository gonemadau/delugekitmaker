import { api, type AudioInfo, type AuditionParams, type DirListing, type Kit, type KitSummary, type PeaksResponse, type SDRootInfo } from "../api";

interface PadRuntimeState {
  volume: number;
  pan: number;
  pitch_semitones: number;
  reverse: boolean;
  start_frames: number;
  end_frames: number;
}

function defaultPadState(): PadRuntimeState {
  return { volume: 0.85, pan: 0, pitch_semitones: 0, reverse: false, start_frames: 0, end_frames: 0 };
}

class AppStore {
  sdRoot = $state<SDRootInfo | null>(null);
  kits = $state<KitSummary[]>([]);
  openKits = $state<Kit[]>([]);
  activeKitIndex = $state<number>(-1);
  loading = $state<boolean>(false);
  error = $state<string | null>(null);
  audioInfo = $state<AudioInfo | null>(null);

  // Selected pad in the active kit (for PadDetail). -1 = none.
  selectedPad = $state<number>(-1);

  // Per-(kit-index, pad-index) runtime override params keyed by `${kitIndex}:${padIndex}`.
  padRuntime = $state<Record<string, PadRuntimeState>>({});

  // Cached peak data per sample path, keyed by path. Reset when path/width changes.
  peaksCache = $state<Record<string, PeaksResponse>>({});

  /// Map sample path -> SampleId, populated as samples are decoded.
  sampleIds = new Map<string, string>();

  get activeKit(): Kit | null {
    if (this.activeKitIndex < 0 || this.activeKitIndex >= this.openKits.length) return null;
    return this.openKits[this.activeKitIndex];
  }

  async refreshAudioInfo() {
    try {
      this.audioInfo = await api.getAudioInfo();
    } catch {
      this.audioInfo = null;
    }
  }

  padRuntimeKey(padIndex: number): string {
    return `${this.activeKitIndex}:${padIndex}`;
  }

  padRuntime_for(padIndex: number): PadRuntimeState {
    const k = this.padRuntimeKey(padIndex);
    if (!this.padRuntime[k]) {
      this.padRuntime = { ...this.padRuntime, [k]: defaultPadState() };
    }
    return this.padRuntime[k];
  }

  updatePadRuntime(padIndex: number, patch: Partial<PadRuntimeState>) {
    const k = this.padRuntimeKey(padIndex);
    const cur = this.padRuntime[k] ?? defaultPadState();
    this.padRuntime = { ...this.padRuntime, [k]: { ...cur, ...patch } };
  }

  /// Decode (if needed) and audition the sample mapped to `padIndex` in the active kit.
  async auditionPad(padIndex: number) {
    const kit = this.activeKit;
    if (!kit || padIndex < 0 || padIndex >= kit.drums.length) return;
    const drum = kit.drums[padIndex];
    const file = drum.osc1?.file_name;
    if (!file) return;
    this.selectedPad = padIndex;
    try {
      let id = this.sampleIds.get(file);
      if (!id) {
        id = await api.decodeSample(file);
        this.sampleIds.set(file, id);
      }
      const rt = this.padRuntime_for(padIndex);
      const params: AuditionParams = {
        volume: rt.volume,
        pan: rt.pan,
        pitch_semitones: rt.pitch_semitones,
        reverse: rt.reverse ?? drum.osc1?.reversed ?? false,
        start_frames: rt.start_frames || drum.osc1?.start_samples || 0,
        end_frames: rt.end_frames || drum.osc1?.end_samples || 0,
      };
      await api.auditionPad(padIndex, id, params);
    } catch (e: any) {
      console.warn("audition failed", e);
      this.error = `audition: ${String(e?.msg ?? e)}`;
    }
  }

  async stopAll() {
    try { await api.stopAll(); } catch {}
  }

  async getOrFetchPeaks(file: string, width: number): Promise<PeaksResponse | null> {
    const key = `${file}@${width}`;
    if (this.peaksCache[key]) return this.peaksCache[key];
    try {
      const peaks = await api.getWaveformPeaks(file, width);
      this.peaksCache = { ...this.peaksCache, [key]: peaks };
      return peaks;
    } catch (e: any) {
      console.warn("peaks fetch failed", e);
      return null;
    }
  }

  selectPad(padIndex: number) {
    this.selectedPad = padIndex;
  }

  // ---- Sequencer ----
  bpm = $state<number>(120);
  swing = $state<number>(0); // -0.5..0.5
  playing = $state<boolean>(false);
  currentStep = $state<number>(-1);
  /// Pattern grid: 16 pads × 16 steps (true = active)
  patternGrid = $state<boolean[][]>(
    Array.from({ length: 16 }, () => Array.from({ length: 16 }, () => false))
  );
  /// Per-cell velocity 0..127. Default 100 (normal). Accents = 127. Ghost = ~60.
  velocityGrid = $state<number[][]>(
    Array.from({ length: 16 }, () => Array.from({ length: 16 }, () => 100))
  );

  /// Set a step to on/off and optionally its velocity. Used by drag-paint.
  setStep(pad: number, step: number, on: boolean, velocity?: number) {
    if (this.patternGrid[pad][step] === on && velocity === undefined) return;
    const row = this.patternGrid[pad].slice();
    row[step] = on;
    const next = this.patternGrid.slice();
    next[pad] = row;
    this.patternGrid = next;
    if (velocity !== undefined) {
      const vrow = this.velocityGrid[pad].slice();
      vrow[step] = Math.max(0, Math.min(127, Math.round(velocity)));
      const vnext = this.velocityGrid.slice();
      vnext[pad] = vrow;
      this.velocityGrid = vnext;
    }
    void this.pushPattern();
  }

  toggleStep(pad: number, step: number) {
    this.setStep(pad, step, !this.patternGrid[pad][step]);
  }

  /// Toggle the accent state of a step (normal 100 <-> accent 127). If the step
  /// is off, turns it on as accent.
  toggleAccent(pad: number, step: number) {
    const isOn = this.patternGrid[pad][step];
    const v = this.velocityGrid[pad][step];
    const isAccent = v >= 120;
    if (!isOn) {
      this.setStep(pad, step, true, 127);
    } else if (isAccent) {
      this.setStep(pad, step, true, 100);
    } else {
      this.setStep(pad, step, true, 127);
    }
  }

  setSwing(s: number) {
    this.swing = Math.max(-0.5, Math.min(0.5, s));
    void this.pushPattern();
  }

  clearPattern() {
    this.patternGrid = Array.from({ length: 16 }, () => Array.from({ length: 16 }, () => false));
    this.velocityGrid = Array.from({ length: 16 }, () => Array.from({ length: 16 }, () => 100));
    void this.pushPattern();
  }

  async pushPattern() {
    const kit = this.activeKit;
    if (!kit) return;
    // Ensure samples are decoded so the audio engine knows about them by id.
    const pads: any[] = [];
    for (let p = 0; p < 16; p++) {
      const drum = kit.drums[p];
      let sid: string | null = null;
      const file = drum?.osc1?.file_name;
      if (file) {
        try {
          sid = this.sampleIds.get(file) ?? null;
          if (!sid) {
            sid = await api.decodeSample(file);
            this.sampleIds.set(file, sid);
          }
        } catch (e) {
          console.warn("decode failed", e);
        }
      }
      const rt = this.padRuntime[`${this.activeKitIndex}:${p}`] ?? null;
      pads.push({
        sample_id: sid,
        steps: this.patternGrid[p],
        velocities: this.velocityGrid[p],
        volume: rt?.volume ?? 0.85,
        pan: rt?.pan ?? 0,
        pitch_semitones: rt?.pitch_semitones ?? 0,
      });
    }
    try {
      await api.setPattern({ bpm: this.bpm, swing: this.swing, pads });
    } catch (e: any) {
      console.warn("setPattern failed", e);
    }
  }

  async setBpm(bpm: number) {
    this.bpm = bpm;
    try { await api.setBpm(bpm); } catch (e) { console.warn(e); }
  }

  async play() {
    await this.pushPattern();
    try {
      await api.transportPlay();
      this.playing = true;
    } catch (e) { console.warn(e); }
  }

  async stop() {
    try {
      await api.transportStop();
      this.playing = false;
      this.currentStep = -1;
    } catch (e) { console.warn(e); }
  }

  /// Save the active kit to the SD card.
  async saveActiveKit(bundleMode: "KitSubfolder" | "SharedSamples" = "KitSubfolder") {
    const kit = this.activeKit;
    if (!kit) return null;
    try {
      this.loading = true;
      const report = await api.saveKitToDisk(kit, bundleMode);
      await this.refreshKits();
      return report;
    } catch (e: any) {
      this.error = `save: ${String(e?.msg ?? e)}`;
      return null;
    } finally {
      this.loading = false;
    }
  }

  /// Mutate the active kit by replacing it with a new object; needed so Svelte
  /// reactivity picks up nested array changes.
  private bumpActiveKit() {
    this.openKits = this.openKits.slice();
  }

  /// Swap two pads in the active kit (used by drag-to-reorder).
  swapPads(a: number, b: number) {
    const kit = this.activeKit;
    if (!kit || a === b) return;
    if (a < 0 || a >= kit.drums.length || b < 0 || b >= kit.drums.length) return;
    const tmp = kit.drums[a];
    kit.drums[a] = kit.drums[b];
    kit.drums[b] = tmp;
    this.bumpActiveKit();
  }

  /// Remove the sample from a pad (leaves the pad in place but empty).
  clearPad(padIndex: number) {
    const kit = this.activeKit;
    if (!kit) return;
    if (padIndex < 0 || padIndex >= kit.drums.length) return;
    kit.drums[padIndex] = {
      name: `Pad ${padIndex + 1}`,
      osc1: null,
      volume_hex: null,
      pan_hex: null,
    };
    this.bumpActiveKit();
  }

  /// Assign an absolute on-disk WAV path to a pad. If the pad already has a
  /// sample, it's overwritten.
  assignSampleToPad(padIndex: number, absPath: string) {
    const kit = this.activeKit;
    if (!kit) return;
    if (padIndex < 0 || padIndex >= kit.drums.length) return;
    const baseName = absPath.split(/[\\/]/).pop() ?? "sample";
    const stem = baseName.replace(/\.[Ww][Aa][Vv]$/, "");
    kit.drums[padIndex] = {
      name: stem,
      osc1: {
        file_name: absPath,
        start_samples: 0,
        end_samples: 0,
        transpose: 0,
        cents: 0,
        reversed: false,
        loop_mode: 0,
      },
      volume_hex: null,
      pan_hex: null,
    };
    this.bumpActiveKit();
    this.selectedPad = padIndex;
  }

  /// Open the OS file picker and assign the chosen WAV to the given pad.
  async pickSampleForPad(padIndex: number) {
    try {
      const initial = this.lastBrowseDir ?? undefined;
      const path = await api.pickWavFile(initial);
      if (!path) return;
      this.assignSampleToPad(padIndex, path);
    } catch (e: any) {
      this.error = `pick: ${String(e?.msg ?? e)}`;
    }
  }

  /// Re-classify and re-arrange every loaded sample in the active kit using
  /// the keyword classifier + priority order.
  async autoArrange() {
    const kit = this.activeKit;
    if (!kit) return;
    // Collect (abs_path, drum-copy) for every populated pad.
    const populated: { path: string; drum: any }[] = [];
    for (const d of kit.drums) {
      if (d.osc1?.file_name) populated.push({ path: d.osc1.file_name, drum: { ...d, osc1: { ...d.osc1 } } });
    }
    if (populated.length === 0) return;
    try {
      const imported = await api.importDroppedPaths(populated.map((p) => p.path));
      // Wipe existing drums and place anew by pad_index from the classifier.
      for (let i = 0; i < kit.drums.length; i++) {
        kit.drums[i] = { name: `Pad ${i + 1}`, osc1: null, volume_hex: null, pan_hex: null };
      }
      for (const s of imported) {
        if (s.pad_index == null) continue;
        // Find original drum (preserving its trim / params) by abs_path.
        const orig = populated.find((p) => p.path === s.abs_path)?.drum;
        kit.drums[s.pad_index] = orig ?? {
          name: s.file_name.replace(/\.[Ww][Aa][Vv]$/, ""),
          osc1: {
            file_name: s.abs_path,
            start_samples: 0,
            end_samples: 0,
            transpose: 0,
            cents: 0,
            reversed: false,
            loop_mode: 0,
          },
          volume_hex: null,
          pan_hex: null,
        };
      }
      this.bumpActiveKit();
    } catch (e: any) {
      this.error = `auto-arrange: ${String(e?.msg ?? e)}`;
    }
  }

  /// File browser state.
  browseListing = $state<DirListing | null>(null);
  lastBrowseDir = $state<string | null>(null);

  async browseTo(path?: string) {
    try {
      const listing = await api.listDirectory(path);
      this.browseListing = listing;
      this.lastBrowseDir = listing.abs_path;
    } catch (e: any) {
      this.error = `browse: ${String(e?.msg ?? e)}`;
    }
  }

  /// Process dropped paths (files or a folder). Auto-map and load into a new kit
  /// (or the active kit if it's a fresh "New Kit").
  async importDroppedPaths(paths: string[], kitName?: string) {
    try {
      this.loading = true;
      const imported = await api.importDroppedPaths(paths);
      if (imported.length === 0) {
        this.error = "no WAV files found in drop";
        return;
      }
      // If there's no active kit OR active kit is empty, create/use a new kit.
      let kit = this.activeKit;
      const isEmpty = !kit || kit.drums.every((d) => !d.osc1?.file_name);
      if (!kit || !isEmpty) {
        const newName = kitName || `Dropped ${new Date().toLocaleTimeString().replace(/[:\s]/g, "-")}`;
        await this.newKit(newName);
        kit = this.activeKit!;
      }
      // Place samples
      for (const s of imported) {
        const pad = s.pad_index ?? -1;
        if (pad < 0 || pad >= kit.drums.length) continue;
        kit.drums[pad].name = s.file_name.replace(/\.[Ww][Aa][Vv]$/, "");
        kit.drums[pad].osc1 = {
          file_name: s.abs_path,
          start_samples: 0,
          end_samples: 0,
          transpose: 0,
          cents: 0,
          reversed: false,
          loop_mode: 0,
        };
      }
      // Trigger reactivity by reassigning openKits
      this.openKits = this.openKits.slice();
      // Auto-select first new pad
      const first = imported.find((s) => s.pad_index != null);
      if (first?.pad_index != null) this.selectedPad = first.pad_index;
    } catch (e: any) {
      this.error = `drop: ${String(e?.msg ?? e)}`;
    } finally {
      this.loading = false;
    }
  }

  async pickAndSetRoot() {
    try {
      this.loading = true;
      this.error = null;
      const path = await api.pickSdRoot();
      if (!path) return;
      this.sdRoot = await api.setSdRoot(path);
      await this.refreshKits();
    } catch (e: any) {
      this.error = String(e?.msg ?? e);
    } finally {
      this.loading = false;
    }
  }

  async setRoot(path: string) {
    try {
      this.loading = true;
      this.error = null;
      this.sdRoot = await api.setSdRoot(path);
      await this.refreshKits();
    } catch (e: any) {
      this.error = String(e?.msg ?? e);
    } finally {
      this.loading = false;
    }
  }

  async refreshKits() {
    if (!this.sdRoot) return;
    this.kits = await api.listKits();
  }

  async openKit(summary: KitSummary) {
    try {
      this.loading = true;
      this.error = null;
      const kit = await api.openKit(summary.rel_path);
      // Avoid duplicates
      const existing = this.openKits.findIndex(
        (k) => k.name === summary.name,
      );
      if (existing >= 0) {
        this.activeKitIndex = existing;
      } else {
        this.openKits = [...this.openKits, kit];
        this.activeKitIndex = this.openKits.length - 1;
      }
      // Auto-select the first pad with a sample, so the detail panel populates.
      const firstWithSample = kit.drums.findIndex((d) => d.osc1?.file_name);
      this.selectedPad = firstWithSample >= 0 ? firstWithSample : 0;
    } catch (e: any) {
      this.error = String(e?.msg ?? e);
    } finally {
      this.loading = false;
    }
  }

  async newKit(name: string) {
    try {
      this.loading = true;
      this.error = null;
      const kit = await api.newKit(name);
      this.openKits = [...this.openKits, kit];
      this.activeKitIndex = this.openKits.length - 1;
    } catch (e: any) {
      this.error = String(e?.msg ?? e);
    } finally {
      this.loading = false;
    }
  }

  closeKit(index: number) {
    this.openKits = this.openKits.filter((_, i) => i !== index);
    if (this.activeKitIndex >= this.openKits.length) {
      this.activeKitIndex = this.openKits.length - 1;
    }
  }

  setActiveKit(index: number) {
    this.activeKitIndex = index;
  }
}

export const appStore = new AppStore();
