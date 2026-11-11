import AppKit
import FluidAudio

// CLI mode: `AazadVoice --transcribe file.wav` prints the transcript and exits.
if let i = CommandLine.arguments.firstIndex(of: "--transcribe"), i + 1 < CommandLine.arguments.count {
    let path = CommandLine.arguments[i + 1]
    let done = DispatchSemaphore(value: 0)
    Task {
        let t = Transcriber()
        let start = Date()
        await t.load { s in if case .failed(let e) = s { FileHandle.standardError.write("Model error: \(e)\n".data(using: .utf8)!) } }
        let loaded = Date()
        do {
            let samples = try AudioConverter().resampleAudioFile(path: path)
            let text = try await t.transcribe(samples, sampleRate: 16_000)
            let end = Date()
            print(text)
            FileHandle.standardError.write(String(format: "[load %.2fs · transcribe %.2fs for %.1fs audio]\n",
                loaded.timeIntervalSince(start), end.timeIntervalSince(loaded), Double(samples.count) / 16_000).data(using: .utf8)!)
        } catch {
            FileHandle.standardError.write("Error: \(error.localizedDescription)\n".data(using: .utf8)!)
        }
        done.signal()
    }
    done.wait()
    exit(0)
}

// Aazad Voice is a menu-bar (agent) app — no Dock icon.
let app = NSApplication.shared
let delegate = MainActor.assumeIsolated { AppDelegate() }
app.delegate = delegate
app.setActivationPolicy(.accessory)
app.run()
