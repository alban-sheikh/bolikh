import AppKit
import Combine

/// Observable app-wide status for the menu and windows.
@MainActor
final class AppState: ObservableObject {
    static let shared = AppState()
    @Published var model: Transcriber.State = .notLoaded
    @Published var micGranted = AudioRecorder.permissionGranted
    @Published var accessibilityGranted = TextInserter.accessibilityGranted
    @Published var isRecording = false

    var allReady: Bool { micGranted && accessibilityGranted && model == .ready }

    func refreshPermissions() {
        micGranted = AudioRecorder.permissionGranted
        accessibilityGranted = TextInserter.accessibilityGranted
    }
}

/// Orchestrates hotkey → record → transcribe → insert.
@MainActor
final class DictationController {
    let state = AppState.shared
    let settings = AppSettings.shared
    let transcriber = Transcriber()
    let hud = HUDController()
    private let recorder = AudioRecorder()
    private let hotkey = HotkeyMonitor()
    private var bag = Set<AnyCancellable>()
    private var targetApp: NSRunningApplication?
    private var recordingStarted = Date()
    private var levelThrottle = Date.distantPast

    func start() {
        recorder.onLevel = { [weak self] level in
            Task { @MainActor in self?.handleLevel(level) }
        }
        hotkey.isActive = { [weak self] in self?.state.isRecording ?? false }
        hotkey.onStart = { [weak self] in self?.beginRecording() }
        hotkey.onStop = { [weak self] in self?.finishRecording() }
        hotkey.onCancel = { [weak self] in self?.cancelRecording() }

        settings.$triggerKey.sink { [weak self] in self?.hotkey.triggerKey = $0 }.store(in: &bag)
        settings.$recordMode.sink { [weak self] in self?.hotkey.mode = $0 }.store(in: &bag)
        hotkey.start()
        loadModel()
    }

    func loadModel() {
        let state = self.state
        Task {
            await transcriber.load { s in
                Task { @MainActor in state.model = s }
            }
        }
    }

    // MARK: - Recording lifecycle

    func toggleFromMenu() {
        state.isRecording ? finishRecording() : beginRecording(handsFree: true)
    }

    private func beginRecording(handsFree: Bool = false) {
        guard !state.isRecording else { return }
        state.refreshPermissions()
        guard state.micGranted else {
            hud.show(.message("Microphone access needed — open Aazad Voice setup"))
            Task { _ = await AudioRecorder.requestPermission(); state.refreshPermissions() }
            return
        }
        guard state.model == .ready else {
            hud.show(.message(modelStatusMessage))
            return
        }
        targetApp = NSWorkspace.shared.frontmostApplication
        do {
            try recorder.start()
        } catch {
            hud.show(.message(error.localizedDescription))
            return
        }
        recordingStarted = Date()
        state.isRecording = true
        if settings.playSounds { Sounds.start() }
        hud.show(handsFree || settings.recordMode == .toggle ? .handsFree : .listening)

        // In smart mode, flip the HUD to "hands-free" if the key was just tapped.
        if settings.recordMode == .smart && !handsFree {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.35) { [weak self] in
                guard let self, self.state.isRecording, !NSEvent.modifierFlags.contains(self.settings.triggerKey.flag) else { return }
                self.hud.show(.handsFree)
            }
        }
    }

    private func finishRecording() {
        guard state.isRecording else { return }
        state.isRecording = false
        let (samples, rate) = recorder.stop()
        let duration = Date().timeIntervalSince(recordingStarted)
        if settings.playSounds { Sounds.stop() }

        guard duration > 0.25, samples.count > Int(rate * 0.25) else { hud.hide(); return }
        hud.show(.transcribing)

        Task {
            do {
                let raw = try await transcriber.transcribe(samples, sampleRate: rate)
                let text = TextProcessor.process(raw, settings: settings)
                guard !text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
                    hud.show(.message("Didn't catch that"))
                    return
                }
                hud.hide()
                insert(text)
                if settings.keepHistory {
                    HistoryStore.shared.add(HistoryItem(text: text.trimmingCharacters(in: .whitespacesAndNewlines),
                                                        date: Date(), app: targetApp?.localizedName, duration: duration))
                }
            } catch {
                hud.show(.message("Transcription failed: \(error.localizedDescription)"))
            }
        }
    }

    private func cancelRecording() {
        guard state.isRecording else { return }
        state.isRecording = false
        _ = recorder.stop()
        hud.show(.message("Cancelled"))
    }

    private func insert(_ text: String) {
        state.refreshPermissions()
        guard state.accessibilityGranted else {
            // Fall back to clipboard so the words are never lost.
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
            hud.show(.message("Copied — enable Accessibility to auto-paste"))
            return
        }
        // Make sure the app the user was dictating into is still frontmost.
        if let app = targetApp, app != NSWorkspace.shared.frontmostApplication {
            app.activate()
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.12) { [settings] in
                TextInserter.insert(text, restoreClipboard: settings.restoreClipboard)
            }
        } else {
            TextInserter.insert(text, restoreClipboard: settings.restoreClipboard)
        }
    }

    private func handleLevel(_ level: Float) {
        // ~30 fps is plenty for the meter.
        let now = Date()
        guard now.timeIntervalSince(levelThrottle) > 0.033 else { return }
        levelThrottle = now
        hud.model.push(level: level)
    }

    var modelStatusMessage: String {
        switch state.model {
        case .notLoaded: return "Speech model not loaded"
        case .downloading(let p): return "Downloading speech model… \(Int(p * 100))%"
        case .loading: return "Preparing speech model…"
        case .ready: return "Ready"
        case .failed(let e): return "Model error: \(e)"
        }
    }
}

enum Sounds {
    static func start() { NSSound(named: "Tink")?.play() }
    static func stop() { NSSound(named: "Pop")?.play() }
}
