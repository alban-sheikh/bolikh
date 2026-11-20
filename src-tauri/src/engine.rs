//! NVIDIA Parakeet TDT 0.6B v3 (int8 ONNX), downloaded once from Hugging Face and run
//! locally through ONNX Runtime. Works on macOS, Windows and Linux.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams};
use transcribe_rs::onnx::Quantization;

pub const MODEL_DIR_NAME: &str = "parakeet-tdt-0.6b-v3-int8";
const REPO: &str = "istupakov/parakeet-tdt-0.6b-v3-onnx";
/// Pinned revision so every install gets byte-identical, verified files.
const REVISION: &str = "8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce";

struct ModelFile {
    name: &'static str,
    size: u64,
    sha256: &'static str,
}

const FILES: &[ModelFile] = &[
    ModelFile { name: "nemo128.onnx", size: 139_764, sha256: "a9fde1486ebfcc08f328d75ad4610c67835fea58c73ba57e3209a6f6cf019e9f" },
    ModelFile { name: "vocab.txt", size: 93_939, sha256: "d58544679ea4bc6ac563d1f545eb7d474bd6cfa467f0a6e2c1dc1c7d37e3c35d" },
    ModelFile { name: "decoder_joint-model.int8.onnx", size: 18_202_004, sha256: "eea7483ee3d1a30375daedc8ed83e3960c91b098812127a0d99d1c8977667a70" },
    ModelFile { name: "encoder-model.int8.onnx", size: 652_183_999, sha256: "6139d2fa7e1b086097b277c7149725edbab89cc7c7ae64b23c741be4055aff09" },
];

pub fn total_download_bytes() -> u64 {
    FILES.iter().map(|f| f.size).sum()
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ModelState {
    NotLoaded,
    Downloading { progress: f32 },
    Loading,
    Ready,
    Failed { error: String },
}

/// Longest piece of audio handed to the model in one go, in seconds. The model itself fails
/// beyond about 6.5 minutes; one-minute pieces also keep memory use flat on long dictations.
const MAX_PIECE_SECS: usize = 60;

pub struct Engine {
    dir: PathBuf,
    model: Mutex<Option<ParakeetModel>>,
    /// Held while the model is being read from disk, so two callers never load it twice.
    loading: Mutex<()>,
}

impl Engine {
    pub fn new(models_root: PathBuf) -> Self {
        Self { dir: models_root.join(MODEL_DIR_NAME), model: Mutex::new(None), loading: Mutex::new(()) }
    }

    pub fn is_ready(&self) -> bool {
        self.model.lock().unwrap().is_some()
    }

    /// Present on disk with the right sizes (full hashes are checked at download time).
    pub fn is_downloaded(&self) -> bool {
        FILES.iter().all(|f| std::fs::metadata(self.dir.join(f.name)).map(|m| m.len() == f.size).unwrap_or(false))
    }

    /// Downloads (if needed) and loads the model. Blocking; call from a worker thread.
    pub fn prepare(&self, on_state: &dyn Fn(ModelState)) -> Result<(), String> {
        if self.is_ready() {
            on_state(ModelState::Ready);
            return Ok(());
        }
        if !self.is_downloaded() {
            on_state(ModelState::Downloading { progress: 0.0 });
            self.download(&|p| on_state(ModelState::Downloading { progress: p }))?;
        }
        on_state(ModelState::Loading);
        self.load()?;
        on_state(ModelState::Ready);
        Ok(())
    }

    /// Reads the downloaded model into memory if it isn't there. Blocking (a few seconds).
    pub fn load(&self) -> Result<(), String> {
        let _one_at_a_time = self.loading.lock().unwrap();
        if self.is_ready() {
            return Ok(());
        }
        let model = ParakeetModel::load(&self.dir, &Quantization::Int8)
            .map_err(|e| format!("Could not load speech model: {e}"))?;
        *self.model.lock().unwrap() = Some(model);
        // Warm-up so the first real dictation is instant.
        let _ = self.transcribe_16k(&vec![0.0; 16_000]);
        Ok(())
    }

    /// Drops the model from memory. `load` brings it back from disk.
    pub fn unload(&self) {
        *self.model.lock().unwrap() = None;
    }

    fn download(&self, progress: &dyn Fn(f32)) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let total = total_download_bytes() as f64;
        let mut done_before = 0u64;
        let client = reqwest::blocking::Client::builder()
            .timeout(None)
            .user_agent("AazadVoice")
            .build()
            .map_err(|e| e.to_string())?;

        for f in FILES {
            let dest = self.dir.join(f.name);
            if std::fs::metadata(&dest).map(|m| m.len() == f.size).unwrap_or(false) {
                done_before += f.size;
                continue;
            }
            let url = format!("https://huggingface.co/{REPO}/resolve/{REVISION}/{}", f.name);
            let part = self.dir.join(format!("{}.part", f.name));
            let mut resp = client
                .get(&url)
                .send()
                .and_then(|r| r.error_for_status())
                .map_err(|e| format!("Download failed ({}): {e}", f.name))?;
            let mut out = std::fs::File::create(&part).map_err(|e| e.to_string())?;
            let mut hasher = Sha256::new();
            let mut buf = vec![0u8; 1 << 16];
            let mut got = 0u64;
            let mut last_emit = std::time::Instant::now();
            loop {
                let n = resp.read(&mut buf).map_err(|e| format!("Download interrupted: {e}"))?;
                if n == 0 {
                    break;
                }
                out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
                hasher.update(&buf[..n]);
                got += n as u64;
                if last_emit.elapsed().as_millis() > 150 {
                    progress(((done_before + got) as f64 / total) as f32);
                    last_emit = std::time::Instant::now();
                }
            }
            out.flush().map_err(|e| e.to_string())?;
            drop(out);
            let digest = hex::encode(hasher.finalize());
            if digest != f.sha256 {
                let _ = std::fs::remove_file(&part);
                return Err(format!("{} failed its integrity check. Please retry.", f.name));
            }
            std::fs::rename(&part, &dest).map_err(|e| e.to_string())?;
            done_before += f.size;
            progress((done_before as f64 / total) as f32);
        }
        Ok(())
    }

    /// Transcribe 16 kHz mono audio of any length. Long audio is cut at pauses into pieces the
    /// model can take, and the pieces' text is joined.
    pub fn transcribe_16k(&self, audio: &[f32]) -> Result<String, String> {
        let mut text = String::new();
        for piece in crate::audio::split_long(audio, MAX_PIECE_SECS) {
            let part = self.transcribe_piece(piece)?;
            if !part.is_empty() {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&part);
            }
        }
        Ok(text)
    }

    fn transcribe_piece(&self, audio: &[f32]) -> Result<String, String> {
        let mut guard = self.model.lock().unwrap();
        let model = guard.as_mut().ok_or("Speech model is not loaded yet.")?;
        let mut samples = audio.to_vec();
        // Very short clips decode more reliably with a little trailing silence.
        if samples.len() < 16_000 {
            samples.resize(16_000, 0.0);
        }
        model
            .transcribe_with(&samples, &ParakeetParams::default())
            .map(|r| r.text.trim().to_string())
            .map_err(|e| format!("Transcription failed: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Developer check against the real model: shows what it writes for spoken phrases and
    /// what the clean-up turns that into. Needs the model already downloaded by the app.
    ///   BOLIKH_MODELS=<app data>/models BOLIKH_WAVS=<folder of 16 kHz mono 16-bit .wav> \
    ///   cargo test --lib real_model -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_model() {
        let engine = Engine::new(PathBuf::from(std::env::var("BOLIKH_MODELS").unwrap()));
        engine.prepare(&|_| {}).unwrap();
        let settings = crate::settings::Settings { add_trailing_space: false, ..Default::default() };
        let mut files: Vec<_> = std::fs::read_dir(std::env::var("BOLIKH_WAVS").unwrap())
            .unwrap()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "wav"))
            .collect();
        files.sort();
        for path in files {
            let bytes = std::fs::read(&path).unwrap();
            let data = bytes.windows(4).position(|w| w == b"data").unwrap() + 8;
            let samples: Vec<f32> =
                bytes[data..].chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0).collect();
            // Same path as the app: silence trimmed first.
            let speech = crate::audio::trim_silence(&samples);
            println!("  {:.1} s, {:.1} s after trimming", samples.len() as f32 / 16_000.0, speech.len() as f32 / 16_000.0);
            let raw = engine.transcribe_16k(speech).unwrap();
            println!("{}\n  model: {raw}\n  typed: {}", path.display(), crate::text::process(&raw, &settings));
        }
    }

    /// Developer check of "free memory when idle": the model can be dropped and brought back.
    ///   BOLIKH_MODELS=<app data>/models cargo test --lib reloads_after_unload -- --ignored --nocapture
    #[test]
    #[ignore]
    fn reloads_after_unload() {
        let engine = Engine::new(std::env::var("BOLIKH_MODELS").expect("set BOLIKH_MODELS").into());
        engine.load().unwrap();
        assert!(engine.is_ready());
        engine.unload();
        assert!(!engine.is_ready());
        assert!(engine.transcribe_16k(&vec![0.0; 16_000]).is_err());
        let t = std::time::Instant::now();
        engine.load().unwrap();
        println!("reloaded in {:?}", t.elapsed());
        assert!(engine.transcribe_16k(&vec![0.0; 16_000]).is_ok());
    }
}
