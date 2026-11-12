# Bolikh

**Private, offline dictation for every app, on macOS, Windows and Linux.**
Hold a key, speak, let go: your words are typed into whatever text field you're in. Slack, Gmail, VS Code, Word, terminals, anything.

- **100% on-device.** Speech recognition (NVIDIA Parakeet TDT 0.6B v3) runs locally through ONNX Runtime. No account, no cloud, no telemetry. Audio is never written to disk.
- **Fast.** ~10 s of speech transcribes in ~0.25 s on an Apple M5 Pro CPU.
- **Clean output.** Automatic punctuation and capitalisation, filler words (um, uh, hmm) removed, custom dictionary, "new line" / "new paragraph" commands.
- **Works everywhere.** Inserts at the cursor in any app and restores your clipboard afterwards.

## Using it

| Action | How |
|---|---|
| Push-to-talk | Hold **Right ⌥** (macOS) or **Right Ctrl** (Windows/Linux), speak, release |
| Hands-free | Quick-tap the key, speak as long as you like, tap again |
| Cancel | **Esc** |
| Line breaks | Say "new line" or "new paragraph" |
| Fix names & jargon | Dictionary page, e.g. "azad" → "Aazad" |

The key and behaviour (Smart / Hold / Toggle) are configurable in Settings.

## Platform setup

| OS | What's needed |
|---|---|
| macOS 12+ | Microphone + Accessibility permission (the app walks you through it) |
| Windows 10/11 | Nothing extra |
| Linux (X11 or Wayland): Ubuntu 24.04+, Debian 13+, Fedora 40+ (64-bit Intel/AMD) | Add yourself to the `input` group: `sudo usermod -aG input $USER`, then log out/in. On **Wayland** also install a typing helper: `wtype` (KDE, Sway, Hyprland) or `ydotool` (GNOME). |

Supported languages (auto-detected): Bulgarian, Croatian, Czech, Danish, Dutch, English, Estonian, Finnish, French, German, Greek, Hungarian, Italian, Latvian, Lithuanian, Maltese, Polish, Portuguese, Romanian, Russian, Slovak, Slovenian, Spanish, Swedish, Ukrainian.

## Development

Requirements: Rust (stable), Node 20+. Linux builds need a distro with glibc 2.38+ (e.g. Ubuntu 24.04), because the bundled ONNX Runtime is built against it. Also install: `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libasound2-dev libssl-dev`.

```bash
npm install
npm run dev                       # run with hot reload
npm run build                     # installers in src-tauri/target/release/bundle
cargo test --manifest-path src-tauri/Cargo.toml --lib
```

Builds for all platforms run on GitHub Actions (`.github/workflows/build.yml`): trigger it manually from the Actions tab, or push a tag like `v0.2.0` to create a draft release with every installer attached.

### Project layout
```
src-tauri/src/
  lib.rs          app core: tray, HUD, commands, record → transcribe → insert pipeline
  hotkey.rs       global push-to-talk state machine (smart / hold / toggle, Esc cancels)
  audio.rs        microphone capture (cpal) + 16 kHz windowed-sinc resampler
  engine.rs       Parakeet v3 ONNX: verified download from Hugging Face, load, transcribe
  text.rs         filler removal, dictionary, voice commands (unit-tested)
  insert.rs       clipboard + paste shortcut per OS (incl. Wayland), clipboard restore
  permissions.rs  per-OS permission checks
  settings.rs, history.rs
ui/               settings window, onboarding, HUD (plain HTML/CSS/JS)
legacy/macos-swift/   original native macOS prototype (CoreML / Neural Engine)
```

### Shipping signed releases
- **macOS:** Apple Developer ID (US$99/yr). Add the `APPLE_*` secrets listed in the workflow and tauri-action signs + notarizes.
- **Windows:** without a code-signing certificate, SmartScreen shows a warning on first run. An OV/EV certificate or Azure Trusted Signing removes it.
- **Linux:** `.deb`, `.rpm` and `.AppImage` need no signing.

## License
Copyright © 2026 Aazad Software Solutions.

Bolikh is free software under the [GNU General Public License v3.0](LICENSE): you may use, study, change and share it, and anything you distribute that is based on it must also be released under GPL-3.0 with its source code. The name "Bolikh" and its logo identify official builds from Aazad Software Solutions.

Third-party components: see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). The speech model (NVIDIA Parakeet TDT 0.6B v3) is downloaded separately and licensed CC BY 4.0.
