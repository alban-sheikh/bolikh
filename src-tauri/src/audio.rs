//! Microphone capture. Audio lives only in memory and is discarded after transcription.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub type LevelFn = Arc<dyn Fn(f32) + Send + Sync>;

enum Cmd {
    /// Device name to record from; empty means the system default.
    Start(String, Sender<Result<(), String>>),
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

    /// Starts capture from the named microphone, or the system default when `device` is empty
    /// or no longer connected.
    pub fn start(&self, device: &str) -> Result<(), String> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(Cmd::Start(device.to_string(), tx)).map_err(|e| e.to_string())?;
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
            Cmd::Start(device, reply) => {
                stream = None;
                buffer.lock().unwrap().clear();
                match open_stream(&device, buffer.clone(), on_level.clone()) {
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

/// Names of the microphones the system offers, in the system's order, without duplicates.
pub fn input_devices() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    if let Ok(devices) = cpal::default_host().input_devices() {
        for name in devices.filter_map(|d| d.name().ok()) {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// The microphone called `name`, or the system default when `name` is empty or not connected.
fn pick_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    if !name.is_empty() {
        let found = host
            .input_devices()
            .ok()
            .and_then(|mut devices| devices.find(|d| d.name().map(|n| n == name).unwrap_or(false)));
        if found.is_some() {
            return found;
        }
        log::warn!("microphone {name:?} not found, using the system default");
    }
    host.default_input_device()
}

/// Loudest sample of a recording, 0…1. Used to tell silence and very quiet speech apart from speech.
pub fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0f32, |m, s| m.max(s.abs()))
}

fn open_stream(
    device: &str,
    buffer: Arc<Mutex<Vec<f32>>>,
    on_level: LevelFn,
) -> Result<(cpal::Stream, u32), String> {
    let host = cpal::default_host();
    let device = pick_device(&host, device).ok_or_else(|| "No microphone found.".to_string())?;
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

const RATE_16K: usize = 16_000;
/// Analysis frame: 20 ms at 16 kHz.
const FRAME: usize = 320;

fn frame_rms(frame: &[f32]) -> f32 {
    (frame.iter().map(|s| s * s).sum::<f32>() / frame.len().max(1) as f32).sqrt()
}

/// Cuts silence from the start and end of 16 kHz audio, keeping a little room around the speech.
/// Returns the input unchanged when it is all quiet or nothing worth cutting is found.
pub fn trim_silence(audio: &[f32]) -> &[f32] {
    const PAD: usize = RATE_16K * 3 / 10;
    let loudest = audio.chunks(FRAME).map(frame_rms).fold(0f32, f32::max);
    // Speech frames stand well above the room's noise floor; 3% of the loudest frame (-30 dB) separates them.
    let threshold = (loudest * 0.03).max(0.0008);
    let voiced = |f: &[f32]| frame_rms(f) >= threshold;
    let Some(first) = audio.chunks(FRAME).position(voiced) else { return audio };
    let last = audio.chunks(FRAME).rposition(voiced).unwrap_or(first);
    let start = (first * FRAME).saturating_sub(PAD);
    let end = ((last + 1) * FRAME + PAD).min(audio.len());
    if end.saturating_sub(start) < RATE_16K / 4 {
        return audio;
    }
    &audio[start..end]
}

/// Splits long 16 kHz audio into pieces the speech model can take, cutting at the quietest
/// moment (a pause between sentences) near every minute. Audio up to `max_secs` is left whole.
pub fn split_long(audio: &[f32], max_secs: usize) -> Vec<&[f32]> {
    let max = max_secs * RATE_16K;
    let mut pieces = Vec::new();
    let mut rest = audio;
    while rest.len() > max {
        // Look for the pause in the last third of the allowed length.
        let (from, to) = (max * 2 / 3, max);
        const WINDOW: usize = FRAME * 15; // 0.3 s
        let mut best = (f32::MAX, to);
        let mut at = from;
        while at + WINDOW <= to {
            let level = frame_rms(&rest[at..at + WINDOW]);
            if level < best.0 {
                best = (level, at + WINDOW / 2);
            }
            at += FRAME * 5;
        }
        let (piece, tail) = rest.split_at(best.1);
        pieces.push(piece);
        rest = tail;
    }
    pieces.push(rest);
    pieces
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(secs: f32, amp: f32) -> Vec<f32> {
        (0..(secs * 16_000.0) as usize).map(|i| amp * (i as f32 * 0.2).sin()).collect()
    }

    #[test]
    fn trims_silence_around_speech() {
        let mut audio = vec![0.0001; 16_000 * 2];
        audio.extend(tone(1.0, 0.3));
        audio.extend(vec![0.0001; 16_000 * 3]);
        let trimmed = trim_silence(&audio);
        // 1 s of speech plus 0.3 s of room on each side.
        assert!((trimmed.len() as i64 - 16_000 * 16 / 10).abs() < 1_000, "{}", trimmed.len());
        // Silence only, or speech from edge to edge, is returned as it is.
        let quiet = vec![0.0; 16_000];
        assert_eq!(trim_silence(&quiet).len(), quiet.len());
        let full = tone(2.0, 0.3);
        assert_eq!(trim_silence(&full).len(), full.len());
    }

    #[test]
    fn splits_long_audio_at_a_pause() {
        // 50 s of speech, a 1 s pause, 40 s of speech; the limit is 60 s.
        let mut audio = tone(50.0, 0.3);
        audio.extend(vec![0.0; 16_000]);
        audio.extend(tone(40.0, 0.3));
        let pieces = split_long(&audio, 60);
        assert_eq!(pieces.len(), 2);
        let cut = pieces[0].len() as f32 / 16_000.0;
        assert!((50.0..=51.0).contains(&cut), "cut at {cut}");
        assert_eq!(pieces.iter().map(|p| p.len()).sum::<usize>(), audio.len());
        assert_eq!(split_long(&tone(30.0, 0.3), 60).len(), 1);
    }

    #[test]
    fn peak_is_the_loudest_sample() {
        assert_eq!(peak(&[]), 0.0);
        assert_eq!(peak(&[0.1, -0.6, 0.3]), 0.6);
    }

    /// Prints the microphones this computer offers: `cargo test --lib lists_microphones -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn lists_microphones() {
        println!("{:#?}", input_devices());
    }

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
