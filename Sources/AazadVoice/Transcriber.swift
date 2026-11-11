import Foundation
import FluidAudio

/// Wraps NVIDIA Parakeet TDT 0.6B v3 (CoreML build, runs on the Apple Neural Engine).
/// Models are downloaded once from Hugging Face (FluidInference/parakeet-tdt-0.6b-v3-coreml)
/// and cached locally; afterwards everything is offline.
actor Transcriber {
    enum State: Equatable {
        case notLoaded
        case downloading(Double)
        case loading
        case ready
        case failed(String)
    }

    private var manager: AsrManager?
    private var decoderLayers = 2
    private let converter = AudioConverter()

    func load(progress: @escaping @Sendable (State) -> Void) async {
        if manager != nil { progress(.ready); return }
        do {
            progress(.downloading(0))
            let models = try await AsrModels.downloadAndLoad(version: .v3) { p in
                switch p.phase {
                case .compiling: progress(.loading)
                default: progress(.downloading(p.fractionCompleted))
                }
            }
            progress(.loading)
            let asr = AsrManager(config: .default)
            try await asr.loadModels(models)
            decoderLayers = models.version.decoderLayers
            manager = asr
            // Warm up the Neural Engine so the first real dictation is instant.
            _ = try? await transcribeRaw(Array(repeating: 0, count: 16_000))
            progress(.ready)
        } catch {
            progress(.failed(error.localizedDescription))
        }
    }

    var isReady: Bool { manager != nil }

    /// Transcribe mono samples recorded at `sampleRate`.
    func transcribe(_ samples: [Float], sampleRate: Double) async throws -> String {
        let audio16k = sampleRate == 16_000 ? samples : try converter.resample(samples, from: sampleRate)
        return try await transcribeRaw(audio16k)
    }

    private func transcribeRaw(_ audio16k: [Float]) async throws -> String {
        guard let manager else { throw NSError(domain: "AazadVoice", code: 2,
            userInfo: [NSLocalizedDescriptionKey: "Speech model is not loaded yet."]) }
        var audio = audio16k
        // Parakeet needs at least ~1 s of context; pad short clips with silence.
        if audio.count < 16_000 { audio += Array(repeating: 0, count: 16_000 - audio.count) }
        var state = TdtDecoderState.make(decoderLayers: decoderLayers)
        let result = try await manager.transcribe(audio, decoderState: &state)
        return result.text
    }
}
