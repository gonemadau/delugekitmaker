import { invoke } from "@tauri-apps/api/core";

// Types mirroring the Rust side. Hand-written until we wire up tauri-specta.
export interface KitSummary {
  name: string;
  file_name: string;
  rel_path: string;
  size_bytes: number;
}

export interface OscSample {
  file_name: string;
  start_samples: number;
  end_samples: number;
  transpose: number;
  cents: number;
  reversed: boolean;
  loop_mode: number;
}

export interface Drum {
  name: string;
  osc1: OscSample | null;
  volume_hex: string | null;
  pan_hex: string | null;
}

export interface Kit {
  name: string;
  firmware_version: string;
  earliest_compatible_firmware: string;
  drums: Drum[];
}

export interface SDRootInfo {
  path: string;
  kits_dir: string;
  samples_dir: string;
}

export interface AudioInfo {
  sample_rate: number;
  channels: number;
}

export interface AuditionParams {
  volume?: number;
  pan?: number;
  pitch_semitones?: number;
  reverse?: boolean;
  start_frames?: number;
  end_frames?: number;
}

export interface PeaksResponse {
  sample_id: string;
  frames: number;
  sample_rate: number;
  channels: number;
  min: number[];
  max: number[];
}

export interface SequencerPadInput {
  sample_id: string | null;
  steps: boolean[]; // length 16
  velocities?: number[]; // length 16, 0..127
  volume?: number;
  pan?: number;
  pitch_semitones?: number;
}

export interface SequencerPatternInput {
  bpm: number;
  swing?: number; // -0.5..0.5
  pads: SequencerPadInput[];
}

export interface SaveReport {
  xml_path: string;
  copied_samples: string[];
  reused_samples: string[];
}

export interface ImportedSample {
  abs_path: string;
  file_name: string;
  category: string | null;
  pad_index: number | null;
}

export interface DirEntry {
  name: string;
  abs_path: string;
  kind: "dir" | "wav";
  size_bytes: number;
}

export interface DirListing {
  abs_path: string;
  parent: string | null;
  entries: DirEntry[];
}

export const api = {
  pickSdRoot: () => invoke<string | null>("pick_sd_root"),
  setSdRoot: (path: string) => invoke<SDRootInfo>("set_sd_root", { path }),
  getSdRoot: () => invoke<SDRootInfo | null>("get_sd_root"),
  listKits: () => invoke<KitSummary[]>("list_kits_cmd"),
  openKit: (relPath: string) => invoke<Kit>("open_kit", { relPath }),
  newKit: (name: string) => invoke<Kit>("new_kit", { name }),
  saveKitXml: (kit: Kit) => invoke<string>("save_kit_xml", { kit }),
  getAudioInfo: () => invoke<AudioInfo | null>("get_audio_info"),
  decodeSample: (relPath: string) => invoke<string>("decode_sample", { relPath }),
  getWaveformPeaks: (relPath: string, width: number) =>
    invoke<PeaksResponse>("get_waveform_peaks", { relPath, width }),
  auditionPad: (pad: number, sampleId: string, params?: AuditionParams) =>
    invoke<void>("audition_pad", { pad, sampleId, params: params ?? {} }),
  stopAll: () => invoke<void>("stop_all"),
  setPattern: (pattern: SequencerPatternInput) =>
    invoke<void>("set_pattern", { pattern }),
  setBpm: (bpm: number) => invoke<void>("set_bpm", { bpm }),
  transportPlay: () => invoke<void>("transport_play"),
  transportStop: () => invoke<void>("transport_stop"),
  saveKitToDisk: (kit: Kit, bundleMode: "KitSubfolder" | "SharedSamples" = "KitSubfolder") =>
    invoke<SaveReport>("save_kit_to_disk", { kit, bundleMode, flavor: "OfficialV4" }),
  importDroppedPaths: (paths: string[]) => invoke<ImportedSample[]>("import_dropped_paths", { paths }),
  listDirectory: (path?: string) => invoke<DirListing>("list_directory", { path: path ?? null }),
  listDrives: () => invoke<string[]>("list_drives"),
  pickWavFile: (initialDir?: string) =>
    invoke<string | null>("pick_wav_file", { initialDir: initialDir ?? null }),
};
