import AVFoundation

/// Captures microphone audio into memory. Audio never touches disk.
final class AudioRecorder {
    private let engine = AVAudioEngine()
    private var samples: [Float] = []
    private var sampleRate: Double = 48_000
    private let lock = NSLock()

    /// Called on the audio thread with a 0…1 loudness value for the HUD meter.
    var onLevel: ((Float) -> Void)?

    private(set) var isRecording = false

    func start() throws {
        guard !isRecording else { return }
        let input = engine.inputNode
        let format = input.outputFormat(forBus: 0)
        guard format.sampleRate > 0, format.channelCount > 0 else {
            throw NSError(domain: "AazadVoice", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "No microphone input available."])
        }
        sampleRate = format.sampleRate
        lock.lock(); samples.removeAll(keepingCapacity: true); lock.unlock()

        input.installTap(onBus: 0, bufferSize: 1024, format: format) { [weak self] buffer, _ in
            guard let self, let channel = buffer.floatChannelData?[0] else { return }
            let count = Int(buffer.frameLength)
            let chunk = UnsafeBufferPointer(start: channel, count: count)
            var sum: Float = 0
            for s in chunk { sum += s * s }
            let rms = sqrt(sum / Float(max(count, 1)))
            // Map roughly -60 dB…0 dB to 0…1
            let db = 20 * log10(max(rms, 1e-7))
            let level = max(0, min(1, (db + 55) / 45))
            self.lock.lock(); self.samples.append(contentsOf: chunk); self.lock.unlock()
            self.onLevel?(level)
        }
        engine.prepare()
        try engine.start()
        isRecording = true
    }

    /// Stops capture and returns mono samples at the device sample rate.
    func stop() -> (samples: [Float], sampleRate: Double) {
        guard isRecording else { return ([], sampleRate) }
        engine.inputNode.removeTap(onBus: 0)
        engine.stop()
        isRecording = false
        lock.lock(); let out = samples; samples.removeAll(); lock.unlock()
        return (out, sampleRate)
    }

    static func requestPermission() async -> Bool {
        switch AVCaptureDevice.authorizationStatus(for: .audio) {
        case .authorized: return true
        case .notDetermined: return await AVCaptureDevice.requestAccess(for: .audio)
        default: return false
        }
    }

    static var permissionGranted: Bool {
        AVCaptureDevice.authorizationStatus(for: .audio) == .authorized
    }
}
