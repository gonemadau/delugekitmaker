use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use deluge_audio::{
    compute_peaks, AudioCommand, AudioEngine, DecodedSample, PadParams as AudioPadParams,
    PadPattern, Pattern as AudioPattern, Peaks, SampleId,
};
use deluge_fs::{
    default_browse_root, list_dir, list_kits, save_kit as fs_save_kit, BundleMode, DirListing,
    KitSummary, SDRoot, SaveOptions, SaveReport,
};
use deluge_xml::{parse_kit, write_kit, Flavor, Kit};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

mod error;
use error::AppError;

struct AppState {
    sd_root: Mutex<Option<SDRoot>>,
    audio: Mutex<Option<AudioEngine>>,
    /// Map from absolute sample path -> SampleId, so we don't decode the same file twice.
    sample_ids: Mutex<HashMap<PathBuf, SampleId>>,
    /// Cache of decoded samples on the Rust side so we can compute peaks without re-decoding.
    decoded: Mutex<HashMap<SampleId, Arc<DecodedSample>>>,
    /// Last directory the user browsed to in the file picker — persists across sessions.
    last_dir: Mutex<Option<PathBuf>>,
}

impl Default for AppState {
    fn default() -> Self {
        AppState {
            sd_root: Mutex::new(None),
            audio: Mutex::new(None),
            sample_ids: Mutex::new(HashMap::new()),
            decoded: Mutex::new(HashMap::new()),
            last_dir: Mutex::new(load_last_dir()),
        }
    }
}

fn settings_path() -> Option<PathBuf> {
    let base = std::env::var_os("APPDATA").map(PathBuf::from)?;
    Some(base.join("DelugeKitMaker").join("settings.json"))
}

fn load_settings() -> serde_json::Value {
    settings_path()
        .and_then(|p| std::fs::read_to_string(&p).ok())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn save_settings(s: &serde_json::Value) {
    if let Some(p) = settings_path() {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&p, serde_json::to_string_pretty(s).unwrap_or_default());
    }
}

fn load_last_dir() -> Option<PathBuf> {
    let s = load_settings();
    let p = s.get("last_dir")?.as_str()?;
    let pb = PathBuf::from(p);
    if pb.is_dir() { Some(pb) } else { None }
}

fn load_last_sd_root() -> Option<PathBuf> {
    let s = load_settings();
    let p = s.get("sd_root")?.as_str()?;
    let pb = PathBuf::from(p);
    if pb.is_dir() { Some(pb) } else { None }
}

fn save_last_dir(dir: &Path) {
    let mut s = load_settings();
    s["last_dir"] = serde_json::Value::String(dir.to_string_lossy().into_owned());
    save_settings(&s);
}

fn save_last_sd_root(dir: &Path) {
    let mut s = load_settings();
    s["sd_root"] = serde_json::Value::String(dir.to_string_lossy().into_owned());
    save_settings(&s);
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SDRootInfo {
    path: String,
    kits_dir: String,
    samples_dir: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AudioInfo {
    sample_rate: u32,
    channels: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct AuditionParams {
    #[serde(default = "default_volume")]
    volume: f32,
    #[serde(default)]
    pan: f32,
    #[serde(default)]
    pitch_semitones: f32,
    #[serde(default)]
    reverse: bool,
    #[serde(default)]
    start_frames: u32,
    #[serde(default)]
    end_frames: u32,
}

fn default_volume() -> f32 {
    0.85
}

impl From<AuditionParams> for AudioPadParams {
    fn from(p: AuditionParams) -> Self {
        AudioPadParams {
            volume: p.volume,
            pan: p.pan,
            pitch_semitones: p.pitch_semitones,
            reverse: p.reverse,
            start_frames: p.start_frames,
            end_frames: p.end_frames,
        }
    }
}

#[tauri::command]
async fn pick_sd_root(app: tauri::AppHandle) -> Result<Option<String>, AppError> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_title("Select Deluge SD card root (or working folder)")
        .pick_folder(move |folder| {
            let _ = tx.send(folder);
        });
    let folder = rx.recv().map_err(|e| AppError::Other(e.to_string()))?;
    Ok(folder.and_then(|f| f.into_path().ok().map(|p| p.to_string_lossy().to_string())))
}

#[tauri::command]
fn set_sd_root(state: tauri::State<'_, AppState>, path: String) -> Result<SDRootInfo, AppError> {
    let p = PathBuf::from(&path);
    let root = SDRoot::validate(&p).map_err(|e| AppError::Other(e.to_string()))?;
    let info = SDRootInfo {
        path: root.root().to_string_lossy().to_string(),
        kits_dir: root.kits_dir().to_string_lossy().to_string(),
        samples_dir: root.samples_dir().to_string_lossy().to_string(),
    };
    save_last_sd_root(root.root());
    *state.sd_root.lock().unwrap() = Some(root);
    Ok(info)
}

#[tauri::command]
fn get_sd_root(state: tauri::State<'_, AppState>) -> Option<SDRootInfo> {
    state.sd_root.lock().unwrap().as_ref().map(|root| SDRootInfo {
        path: root.root().to_string_lossy().to_string(),
        kits_dir: root.kits_dir().to_string_lossy().to_string(),
        samples_dir: root.samples_dir().to_string_lossy().to_string(),
    })
}

#[tauri::command]
fn list_kits_cmd(state: tauri::State<'_, AppState>) -> Result<Vec<KitSummary>, AppError> {
    let guard = state.sd_root.lock().unwrap();
    let root = guard.as_ref().ok_or(AppError::NoSdRoot)?;
    list_kits(root).map_err(|e| AppError::Other(e.to_string()))
}

#[tauri::command]
fn open_kit(state: tauri::State<'_, AppState>, rel_path: String) -> Result<Kit, AppError> {
    let guard = state.sd_root.lock().unwrap();
    let root = guard.as_ref().ok_or(AppError::NoSdRoot)?;
    let path = root.root().join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
    let xml = std::fs::read_to_string(&path)
        .map_err(|e| AppError::Other(format!("read {}: {}", path.display(), e)))?;
    let mut kit = parse_kit(&xml).map_err(|e| AppError::Other(format!("parse: {}", e)))?;
    // Kit name lives in the filename, not the XML — populate it so tabs render
    // a real label and duplicate-detection works on subsequent clicks.
    if kit.name.is_empty() {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            kit.name = stem.to_string();
        }
    }
    Ok(kit)
}

#[tauri::command]
fn new_kit(name: String) -> Kit {
    Kit::with_default_drums(name, 16)
}

#[tauri::command]
fn save_kit_xml(kit: Kit) -> Result<String, AppError> {
    write_kit(&kit, Flavor::OfficialV4).map_err(|e| AppError::Other(format!("write: {}", e)))
}

#[tauri::command]
fn save_kit_to_disk(
    state: tauri::State<'_, AppState>,
    mut kit: Kit,
    bundle_mode: Option<String>,
    flavor: Option<String>,
) -> Result<SaveReport, AppError> {
    let guard = state.sd_root.lock().unwrap();
    let root = guard.as_ref().ok_or(AppError::NoSdRoot)?;
    let bundle = match bundle_mode.as_deref() {
        Some("SharedSamples") => BundleMode::SharedSamples,
        _ => BundleMode::KitSubfolder,
    };
    let opts = SaveOptions {
        bundle_mode: bundle,
        flavor: flavor.unwrap_or_else(|| "OfficialV4".into()),
    };
    fs_save_kit(root, &mut kit, &opts).map_err(|e| AppError::Other(e.to_string()))
}

#[derive(Debug, Serialize, Deserialize)]
struct ImportedSample {
    abs_path: String,
    file_name: String,
    category: Option<String>,
    pad_index: Option<u32>,
}

/// Take a list of absolute paths (from drag-drop or a folder pick) and produce
/// a 16-pad auto-mapped layout. Each output entry tells the frontend which pad
/// (if any) the sample should go into.
#[tauri::command]
fn import_dropped_paths(paths: Vec<String>) -> Result<Vec<ImportedSample>, AppError> {
    use deluge_classify::{auto_layout, classify};
    // Expand directories to their wav children.
    let mut files: Vec<std::path::PathBuf> = Vec::new();
    for p in paths {
        let path = std::path::PathBuf::from(&p);
        if path.is_dir() {
            for entry in std::fs::read_dir(&path).map_err(|e| AppError::Other(e.to_string()))? {
                let entry = entry.map_err(|e| AppError::Other(e.to_string()))?;
                let ep = entry.path();
                if ep.is_file() && is_wav(&ep) {
                    files.push(ep);
                }
            }
        } else if path.is_file() && is_wav(&path) {
            files.push(path);
        }
    }
    files.sort();
    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string())
        .collect();
    let layout = auto_layout(&names);

    let mut out = Vec::new();
    for (i, p) in files.iter().enumerate() {
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
        let cat = classify(&name).map(|c| format!("{:?}", c));
        // Find the pad slot this name landed in.
        let pad_index = layout
            .iter()
            .enumerate()
            .find(|(_, slot)| slot.as_deref() == Some(&name))
            .map(|(idx, _)| idx as u32);
        out.push(ImportedSample {
            abs_path: p.to_string_lossy().into_owned(),
            file_name: name,
            category: cat,
            pad_index,
        });
        // suppress unused warning for i
        let _ = i;
    }
    Ok(out)
}

fn is_wav(p: &std::path::Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("wav") || e.eq_ignore_ascii_case("wave"))
        .unwrap_or(false)
}

#[tauri::command]
fn get_audio_info(state: tauri::State<'_, AppState>) -> Option<AudioInfo> {
    state.audio.lock().unwrap().as_ref().map(|a| AudioInfo {
        sample_rate: a.sample_rate,
        channels: a.channels,
    })
}

/// Resolve a SD-root-relative path (e.g. "SAMPLES/KICK.WAV") to an absolute path
/// under the current SD root. If `rel` is already absolute, return it unchanged.
fn resolve_sample_path(state: &tauri::State<'_, AppState>, rel: &str) -> Result<PathBuf, AppError> {
    let p = PathBuf::from(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    if p.is_absolute() {
        return Ok(p);
    }
    let guard = state.sd_root.lock().unwrap();
    let root = guard.as_ref().ok_or(AppError::NoSdRoot)?;
    Ok(root.root().join(p))
}

/// Decode a WAV (deduping by abs path) and send LoadSample to the engine.
/// Returns the SampleId so the frontend can later audition by it.
#[tauri::command]
fn decode_sample(state: tauri::State<'_, AppState>, rel_path: String) -> Result<String, AppError> {
    let abs = resolve_sample_path(&state, &rel_path)?;
    // Dedup
    if let Some(id) = state.sample_ids.lock().unwrap().get(&abs).copied() {
        return Ok(id.0.to_string());
    }
    let sample = DecodedSample::decode_wav(&abs)
        .map_err(|e| AppError::Other(format!("decode {}: {}", abs.display(), e)))?;
    let id = SampleId::new();
    let sample_arc = Arc::new(sample);
    state.sample_ids.lock().unwrap().insert(abs.clone(), id);
    state.decoded.lock().unwrap().insert(id, sample_arc.clone());
    let engine_guard = state.audio.lock().unwrap();
    if let Some(engine) = engine_guard.as_ref() {
        engine.send(AudioCommand::LoadSample(id, sample_arc));
    } else {
        return Err(AppError::Other("audio engine not initialized".into()));
    }
    Ok(id.0.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct PeaksResponse {
    sample_id: String,
    frames: u32,
    sample_rate: u32,
    channels: u16,
    min: Vec<f32>,
    max: Vec<f32>,
}

#[tauri::command]
fn get_waveform_peaks(
    state: tauri::State<'_, AppState>,
    rel_path: String,
    width: u32,
) -> Result<PeaksResponse, AppError> {
    // Decode if needed (also populates the engine cache so subsequent audition is instant).
    let id_str = decode_sample(state.clone(), rel_path)?;
    let id = SampleId(
        uuid::Uuid::parse_str(&id_str)
            .map_err(|e| AppError::Other(format!("uuid: {}", e)))?,
    );
    let decoded_guard = state.decoded.lock().unwrap();
    let sample = decoded_guard
        .get(&id)
        .ok_or_else(|| AppError::Other("decoded sample missing".into()))?
        .clone();
    drop(decoded_guard);
    let peaks: Peaks = compute_peaks(&sample, width.max(1) as usize);
    Ok(PeaksResponse {
        sample_id: id_str,
        frames: peaks.source_frames,
        sample_rate: peaks.source_sample_rate,
        channels: peaks.source_channels,
        min: peaks.min,
        max: peaks.max,
    })
}

#[tauri::command]
fn audition_pad(
    state: tauri::State<'_, AppState>,
    pad: u8,
    sample_id: String,
    params: Option<AuditionParams>,
) -> Result<(), AppError> {
    let id = uuid::Uuid::parse_str(&sample_id)
        .map_err(|e| AppError::Other(format!("bad sample id: {}", e)))?;
    let params = params.unwrap_or_default();
    let engine_guard = state.audio.lock().unwrap();
    let engine = engine_guard
        .as_ref()
        .ok_or_else(|| AppError::Other("audio engine not initialized".into()))?;
    engine.send(AudioCommand::AuditionPad {
        pad,
        sample: SampleId(id),
        params: params.into(),
    });
    Ok(())
}

/// Enumerate logical drives (Windows). On non-Windows, returns `["/"]`.
#[tauri::command]
fn list_drives() -> Vec<String> {
    #[cfg(windows)]
    {
        let mut out = Vec::new();
        for letter in b'A'..=b'Z' {
            let path = format!("{}:\\", letter as char);
            if std::path::Path::new(&path).exists() {
                out.push(path);
            }
        }
        out
    }
    #[cfg(not(windows))]
    {
        vec!["/".to_string()]
    }
}

#[tauri::command]
fn list_directory(
    state: tauri::State<'_, AppState>,
    path: Option<String>,
) -> Result<DirListing, AppError> {
    let target = match path {
        Some(p) if !p.is_empty() => PathBuf::from(p),
        _ => {
            let last = state.last_dir.lock().unwrap().clone();
            default_browse_root(last.as_deref().and_then(|p| p.to_str()))
        }
    };
    let listing = list_dir(&target).map_err(|e| AppError::Other(e.to_string()))?;
    // Persist as last_dir so the next launch starts here.
    *state.last_dir.lock().unwrap() = Some(target.clone());
    save_last_dir(&target);
    Ok(listing)
}

#[tauri::command]
async fn pick_wav_file(
    app: tauri::AppHandle,
    initial_dir: Option<String>,
) -> Result<Option<String>, AppError> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut builder = app
        .dialog()
        .file()
        .set_title("Pick a WAV sample")
        .add_filter("WAV", &["wav", "WAV", "wave", "WAVE"]);
    if let Some(dir) = initial_dir.as_deref().filter(|d| !d.is_empty()) {
        builder = builder.set_directory(dir);
    }
    builder.pick_file(move |file| {
        let _ = tx.send(file);
    });
    let result = rx.recv().map_err(|e| AppError::Other(e.to_string()))?;
    let abs = result.and_then(|f| f.into_path().ok());
    if let Some(p) = abs.as_ref() {
        if let Some(parent) = p.parent() {
            let state: tauri::State<'_, AppState> = app.state();
            *state.last_dir.lock().unwrap() = Some(parent.to_path_buf());
            save_last_dir(parent);
        }
    }
    Ok(abs.map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
fn stop_all(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let engine_guard = state.audio.lock().unwrap();
    if let Some(e) = engine_guard.as_ref() {
        e.send(AudioCommand::StopAll);
    }
    Ok(())
}

/// JSON-friendly pattern shape that the frontend sends.
#[derive(Debug, Deserialize)]
struct SequencerPattern {
    bpm: f32,
    #[serde(default)]
    swing: f32,
    /// Per-pad info: sample id (uuid string or null), 16-step boolean array
    pads: Vec<SequencerPad>,
}

#[derive(Debug, Deserialize)]
struct SequencerPad {
    sample_id: Option<String>,
    steps: Vec<bool>,
    /// Optional per-step velocity 0..127. Missing entries default to 100.
    #[serde(default)]
    velocities: Vec<u8>,
    #[serde(default)]
    volume: Option<f32>,
    #[serde(default)]
    pan: Option<f32>,
    #[serde(default)]
    pitch_semitones: Option<f32>,
}

#[tauri::command]
fn set_pattern(state: tauri::State<'_, AppState>, pattern: SequencerPattern) -> Result<(), AppError> {
    let mut ap = AudioPattern::default();
    ap.bpm = pattern.bpm;
    ap.swing = pattern.swing.clamp(-0.5, 0.5);
    for (i, p) in pattern.pads.iter().enumerate().take(16) {
        let mut mask: u16 = 0;
        for (s, on) in p.steps.iter().enumerate().take(16) {
            if *on {
                mask |= 1 << s;
            }
        }
        let mut vels = [100u8; 16];
        for (s, v) in p.velocities.iter().enumerate().take(16) {
            vels[s] = (*v).min(127);
        }
        let sid = match &p.sample_id {
            Some(s) => uuid::Uuid::parse_str(s).ok().map(SampleId),
            None => None,
        };
        ap.pads[i] = PadPattern {
            step_mask: mask,
            velocities: vels,
            sample: sid,
            params: AudioPadParams {
                volume: p.volume.unwrap_or(0.85),
                pan: p.pan.unwrap_or(0.0),
                pitch_semitones: p.pitch_semitones.unwrap_or(0.0),
                reverse: false,
                start_frames: 0,
                end_frames: 0,
            },
        };
    }
    let engine_guard = state.audio.lock().unwrap();
    if let Some(e) = engine_guard.as_ref() {
        e.send(AudioCommand::SetPattern(Arc::new(ap)));
    }
    Ok(())
}

#[tauri::command]
fn set_swing(state: tauri::State<'_, AppState>, swing: f32) -> Result<(), AppError> {
    // Set swing on the engine via SetPattern preserving current pattern shape.
    // The mixer's SetPattern handler preserves `playing`. We need a tiny dedicated
    // command, so for now we re-send the pattern from JS. Stub kept for parity.
    let _ = (state, swing);
    Ok(())
}

#[tauri::command]
fn set_bpm(state: tauri::State<'_, AppState>, bpm: f32) -> Result<(), AppError> {
    let engine_guard = state.audio.lock().unwrap();
    if let Some(e) = engine_guard.as_ref() {
        e.send(AudioCommand::SetBpm(bpm));
    }
    Ok(())
}

#[tauri::command]
fn transport_play(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let engine_guard = state.audio.lock().unwrap();
    if let Some(e) = engine_guard.as_ref() {
        e.send(AudioCommand::TransportPlay);
    }
    Ok(())
}

#[tauri::command]
fn transport_stop(state: tauri::State<'_, AppState>) -> Result<(), AppError> {
    let engine_guard = state.audio.lock().unwrap();
    if let Some(e) = engine_guard.as_ref() {
        e.send(AudioCommand::TransportStop);
    }
    Ok(())
}

pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            pick_sd_root,
            set_sd_root,
            get_sd_root,
            list_kits_cmd,
            open_kit,
            new_kit,
            save_kit_xml,
            get_audio_info,
            decode_sample,
            get_waveform_peaks,
            audition_pad,
            stop_all,
            set_pattern,
            set_bpm,
            set_swing,
            transport_play,
            transport_stop,
            save_kit_to_disk,
            import_dropped_paths,
            list_directory,
            list_drives,
            pick_wav_file,
        ])
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_webview_window("main").unwrap();
                window.open_devtools();
            }

            let state = app.state::<AppState>();

            // Initialize the audio engine.
            match AudioEngine::new() {
                Ok(engine) => {
                    tracing::info!(
                        "audio engine ready: {} Hz, {} ch",
                        engine.sample_rate,
                        engine.channels
                    );
                    // Spawn a bridge thread that forwards engine events to the frontend.
                    let evt_rx = engine.evt_rx.clone();
                    let app_handle = app.handle().clone();
                    std::thread::Builder::new()
                        .name("dkm-evt-bridge".into())
                        .spawn(move || loop {
                            match evt_rx.recv() {
                                Ok(deluge_audio::AudioEvent::Step { step }) => {
                                    let _ = app_handle
                                        .emit("engine://step", serde_json::json!({ "step": step }));
                                }
                                Ok(deluge_audio::AudioEvent::Underrun) => {
                                    let _ = app_handle.emit("engine://underrun", ());
                                }
                                Ok(_) => {}
                                Err(_) => break,
                            }
                        })
                        .ok();
                    *state.audio.lock().unwrap() = Some(engine);
                }
                Err(e) => {
                    tracing::error!("failed to start audio engine: {}", e);
                }
            }

            // Auto-load the last-used SD root from settings, or DKM_AUTO_SD env var if set.
            let auto = std::env::var("DKM_AUTO_SD").ok().map(PathBuf::from)
                .or_else(load_last_sd_root);
            if let Some(path) = auto {
                if let Ok(root) = SDRoot::validate(&path) {
                    tracing::info!("auto-loaded SD root: {}", path.display());
                    *state.sd_root.lock().unwrap() = Some(root);
                }
            }

            // Dev convenience: when DKM_AUTO_DEMO is set, audition the first kit's
            // first three pads on a short delay. Lets us verify the full IPC chain
            // end-to-end without UI interaction.
            if std::env::var("DKM_AUTO_DEMO").is_ok() {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(1200));
                    let state: tauri::State<'_, AppState> = handle.state();
                    // Look up the SD root and the first KIT_BASIC kit.
                    let sd = match state.sd_root.lock().unwrap().as_ref() {
                        Some(r) => r.root().to_path_buf(),
                        None => return,
                    };
                    let kit_xml_path = sd.join("KITS").join("KIT_BASIC.XML");
                    let xml = match std::fs::read_to_string(&kit_xml_path) {
                        Ok(s) => s,
                        Err(_) => return,
                    };
                    let kit = match parse_kit(&xml) {
                        Ok(k) => k,
                        Err(_) => return,
                    };
                    let engine_present = state.audio.lock().unwrap().is_some();
                    if !engine_present {
                        tracing::warn!("auto-demo: no engine");
                        return;
                    }
                    for (idx, drum) in kit.drums.iter().take(3).enumerate() {
                        let Some(osc) = drum.osc1.as_ref() else { continue };
                        let abs = sd.join(osc.file_name.replace('/', std::path::MAIN_SEPARATOR_STR.as_ref()));
                        let sample = match DecodedSample::decode_wav(&abs) {
                            Ok(s) => s,
                            Err(e) => {
                                tracing::warn!("auto-demo decode {}: {}", abs.display(), e);
                                continue;
                            }
                        };
                        let id = SampleId::new();
                        state
                            .audio
                            .lock()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .send(AudioCommand::LoadSample(id, Arc::new(sample)));
                        std::thread::sleep(std::time::Duration::from_millis(80));
                        tracing::info!("auto-demo: audition pad {} ({})", idx, drum.name);
                        state
                            .audio
                            .lock()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .send(AudioCommand::AuditionPad {
                                pad: idx as u8,
                                sample: id,
                                params: AudioPadParams::default(),
                            });
                        std::thread::sleep(std::time::Duration::from_millis(450));
                    }
                });
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running tauri application");
}
