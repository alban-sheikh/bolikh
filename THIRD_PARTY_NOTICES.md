# Third-party notices

Aazad Voice includes or downloads the following third-party components.

| Component | Use | License |
|---|---|---|
| [NVIDIA Parakeet TDT 0.6B v3](https://huggingface.co/nvidia/parakeet-tdt-0.6b-v3) | Speech recognition model | CC-BY-4.0 (commercial use permitted with attribution) |
| [ONNX export by istupakov](https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx) | Model files downloaded at first launch | CC-BY-4.0 |
| [ONNX Runtime](https://github.com/microsoft/onnxruntime) via [ort](https://github.com/pykeio/ort) | Model inference | MIT / Apache-2.0 |
| [transcribe-rs](https://github.com/cjpais/transcribe-rs) | Parakeet decoding | MIT |
| [handy-keys](https://crates.io/crates/handy-keys) | Global hotkeys | MIT |
| [Tauri](https://tauri.app) | App framework | MIT / Apache-2.0 |
| [cpal](https://github.com/RustAudio/cpal), [enigo](https://github.com/enigo-rs/enigo), [arboard](https://github.com/1Password/arboard) | Audio, keyboard, clipboard | Apache-2.0 / MIT |
| [FluidAudio](https://github.com/FluidInference/FluidAudio) | Legacy macOS build only (`legacy/macos-swift`) | Apache-2.0 |

The model is not bundled in the installer; it is downloaded from Hugging Face on first launch
and verified against pinned SHA-256 hashes.
