//! Microphone capture. Audio lives only in memory and is discarded after transcription.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub type LevelFn = Arc<dyn Fn(f32) + Send + Sync>;

enum Cmd {
    Start(Sender<Result<(), String>>),
    Stop(Sender<(Vec<f32>, u32)>),
}

/// Handle to the capture thread. A cpal `Stream` is not `Send` on every platform,
/// so it is created, owned and dropped on one dedicated thread.
pub struct Recorder {
    tx: Sender<Cmd>,
}

impl Recorder {
    pub fn new(on_level: LevelFn) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || worker(rx, on_level))
            .expect("spawn audio thread");
        Self { tx }
    }

    pub fn start(&self) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(Cmd::Start(tx)).map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }

    /// Stops capture and returns mono samples at the device sample rate.
    pub fn stop(&self) -> (Vec<f32>, u32) {
        let (tx, rx) = mpsc::channel();
        if self.tx.send(Cmd::Stop(tx)).is_err() {
            return (Vec::new(), 16_000);
        }
        rx.recv().unwrap_or((Vec::new(), 16_000))
    }
}

fn worker(rx: Receiver<Cmd>, on_level: LevelFn) {
    let buffer: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let mut stream: Option<cpal::Stream> = None;
    let mut rate = 16_000u32;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            Cmd::Start(reply) => {
                stream = None;
                buffer.lock().unwrap().clear();
                match open_stream(buffer.clone(), on_level.clone()) {
                    Ok((s, r)) => {
                        rate = r;
                        stream = Some(s);
                        let _ = reply.send(Ok(()));
                    }
                    Err(e) => {
                        let _ = reply.send(Err(e));
                    }
                }
            }
            Cmd::Stop(reply) => {
                if let Some(s) = stream.take() {
                    let _ = s.pause();
                    drop(s);
                }
                let samples = std::mem::take(&mut *buffer.lock().unwrap());
                let _ = reply.send((samples, rate));
            }
        }
    }
}

fn open_stream(buffer: Arc<Mutex<Vec<f32>>>, on_level: LevelFn) -> Result<(cpal::Stream, u32), String> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No microphone found.".to_string())?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("Microphone unavailable: {e}"))?;
    let channels = supported.channels() as usize;
    let rate = supported.sample_rate().0;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    let err_fn = |e| log::error!("audio stream error: {e}");
    let push = move |mono: Vec<f32>| {
        let mut sum = 0f32;
        for s in &mono {
            sum += s * s;
        }
        let rms = (sum / mono.len().max(1) as f32).sqrt();
        // Map roughly -55 dB…-10 dB to 0…1 for the meter.
        let db = 20.0 * rms.max(1e-7).log10();
        on_level(((db + 55.0) / 45.0).clamp(0.0, 1.0));
        buffer.lock().unwrap().extend_from_slice(&mono);
    };

    fn downmix<T: Copy>(data: &[T], channels: usize, conv: impl Fn(T) -> f32) -> Vec<f32> {
        data.chunks(channels)
            .map(|frame| frame.iter().map(|&s| conv(s)).sum::<f32>() / channels as f32)
            .collect()
    }

    let stream = match format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |d: &[f32], _: &_| push(downmix(d, channels, |s| s)),
            err_fn,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |d: &[i16], _: &_| push(downmix(d, channels, |s| s as f32 / 32768.0)),
            err_fn,
            None,
        ),
        cpal::SampleFormat::I32 => device.build_input_stream(
            &config,
            move |d: &[i32], _: &_| push(downmix(d, channels, |s| s as f32 / 2147483648.0)),
            err_fn,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config,
            move |d: &[u16], _: &_| push(downmix(d, channels, |s| (s as f32 - 32768.0) / 32768.0)),
            err_fn,
            None,
        ),
        other => return Err(format!("Unsupported microphone sample format: {other:?}")),
    }
    .map_err(|e| format!("Could not open microphone: {e}"))?;

    stream.play().map_err(|e| format!("Could not start microphone: {e}"))?;
    Ok((stream, rate))
}

/// Band-limited resampling (windowed sinc) to 16 kHz mono for the speech model.
pub fn resample_to_16k(input: &[f32], from_rate: u32) -> Vec<f32> {
    const TARGET: f64 = 16_000.0;
    if from_rate == 16_000 || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from_rate as f64 / TARGET; // input samples per output sample
    let cutoff = if ratio > 1.0 { 0.95 / ratio } else { 0.95 }; // normalised to input Nyquist
    let half_width = (16.0 * ratio.max(1.0)).ceil() as isize;
    let out_len = (input.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let center = i as f64 * ratio;
        let c = center.floor() as isize;
        let mut acc = 0f64;
        let mut norm = 0f64;
        for j in (c - half_width + 1)..=(c + half_width) {
            if j < 0 || j as usize >= input.len() {
                continue;
            }
            let x = j as f64 - center;
            let sinc = if x.abs() < 1e-9 {
                1.0
            } else {
                let a = std::f64::consts::PI * x * cutoff;
                a.sin() / a
            };
            // Blackman window over the kernel span.
            let t = (x / half_width as f64 + 1.0) / 2.0;
            let w = if (0.0..=1.0).contains(&t) {
                0.42 - 0.5 * (2.0 * std::f64::consts::PI * t).cos() + 0.08 * (4.0 * std::f64::consts::PI * t).cos()
            } else {
                0.0
            };
            let k = sinc * w;
            acc += input[j as usize] as f64 * k;
            norm += k;
        }
        out.push(if norm.abs() > 1e-9 { (acc / norm) as f32 } else { 0.0 });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_length_and_tone() {
        // 1 s of a 440 Hz tone at 48 kHz → 16 000 samples, amplitude preserved.
        let input: Vec<f32> = (0..48_000)
            .map(|n| (2.0 * std::f32::consts::PI * 440.0 * n as f32 / 48_000.0).sin())
            .collect();
        let out = resample_to_16k(&input, 48_000);
        assert_eq!(out.len(), 16_000);
        let peak = out[1000..15000].iter().fold(0f32, |m, s| m.max(s.abs()));
        assert!((peak - 1.0).abs() < 0.05, "peak {peak}");
    }
}
