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

pub struct Engine {
    dir: PathBuf,
    model: Mutex<Option<ParakeetModel>>,
}

impl Engine {
    pub fn new(models_root: PathBuf) -> Self {
        Self { dir: models_root.join(MODEL_DIR_NAME), model: Mutex::new(None) }
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
        let model = ParakeetModel::load(&self.dir, &Quantization::Int8)
            .map_err(|e| format!("Could not load speech model: {e}"))?;
        *self.model.lock().unwrap() = Some(model);
        // Warm-up so the first real dictation is instant.
        let _ = self.transcribe_16k(&vec![0.0; 16_000]);
        on_state(ModelState::Ready);
        Ok(())
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

    /// Transcribe 16 kHz mono audio.
    pub fn transcribe_16k(&self, audio: &[f32]) -> Result<String, String> {
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
