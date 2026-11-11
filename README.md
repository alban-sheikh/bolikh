# Aazad Voice

**Private, offline dictation for every app on your Mac.**
Hold a key, speak, let go — your words are typed into whatever text field you're in: Slack, Mail, VS Code, Chrome, Notes, terminals, anything.

- **100 % on-device.** Speech recognition runs on the Apple Neural Engine. No account, no cloud, no telemetry. Audio is never written to disk.
- **Fast.** ~10 s of speech transcribes in under 0.1 s on Apple Silicon (M5 Pro measurement).
- **Accurate & multilingual.** NVIDIA Parakeet TDT 0.6B v3 with automatic punctuation and capitalisation, 25 European languages.
- **Works everywhere.** Inserts at the cursor in any app; your clipboard is restored afterwards.

## Using it

| Action | How |
|---|---|
| Push-to-talk | Hold **Right ⌥** while speaking, release to insert |
| Hands-free | Quick-tap **Right ⌥**, speak as long as you like, tap again to insert |
| Cancel | **Esc** |
| New line / paragraph | Say "new line" or "new paragraph" |
| Fix names & jargon | Settings → Dictionary (e.g. "azad" → "Aazad") |

Trigger key and behaviour (smart / push-to-talk / toggle) are configurable in Settings.

### Permissions
- **Microphone**: to hear you, only while the key is held.
- **Accessibility**: to paste into the focused app. Without it, text is copied to the clipboard instead.

## Supported languages
Bulgarian, Croatian, Czech, Danish, Dutch, English, Estonian, Finnish, French, German, Greek, Hungarian, Italian, Latvian, Lithuanian, Maltese, Polish, Portuguese, Romanian, Russian, Slovak, Slovenian, Spanish, Swedish, Ukrainian. The language is detected automatically.

## Requirements
macOS 14+ on Apple Silicon. About 500 MB of disk space for the speech model, downloaded once on first launch.

## Building from source
Only the Xcode Command Line Tools are needed (`xcode-select --install`), not full Xcode.

```bash
./scripts/build-app.sh          # → dist/Aazad Voice.app (ad-hoc signed)
./scripts/build-app.sh --dmg    # → also dist/AazadVoice-<version>.dmg
```

Transcribe a file from the command line:
```bash
.build/release/AazadVoice --transcribe recording.wav
```

> **Dev note:** ad-hoc signed builds get a new signature each build, so macOS may ask for Accessibility again after a rebuild. Remove the old entry in System Settings → Privacy & Security → Accessibility and add the new build.

### Shipping a public release
To let others install the app without Gatekeeper warnings you need an Apple Developer ID (US$99/yr):
```bash
SIGN_ID="Developer ID Application: Your Name (TEAMID)" VERSION=1.0.0 ./scripts/build-app.sh --dmg
xcrun notarytool submit dist/AazadVoice-1.0.0.dmg --keychain-profile notary --wait
xcrun stapler staple dist/AazadVoice-1.0.0.dmg
```
(Notarization needs full Xcode or its standalone notary tools.)

## Architecture
```
Sources/AazadVoice/
  main.swift              entry point (+ --transcribe CLI mode)
  AppDelegate.swift       menu bar item, windows
  DictationController     hotkey → record → transcribe → insert
  HotkeyMonitor           global modifier-key push-to-talk / tap / toggle logic
  AudioRecorder           AVAudioEngine capture, in-memory only, level meter
  Transcriber             Parakeet v3 via FluidAudio (CoreML / ANE)
  TextProcessor           dictionary replacements, voice commands, spacing
  TextInserter            clipboard + ⌘V into the focused app, clipboard restore
  HUD                     floating waveform pill
  SettingsView / OnboardingView / History
```

## License
MIT for this app. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) for the model (CC-BY-4.0) and FluidAudio (Apache-2.0).
