import AppKit
import Combine

/// The physical key that triggers dictation.
enum TriggerKey: String, CaseIterable, Identifiable, Codable {
    case rightOption, rightCommand, rightControl, fn, leftOption

    var id: String { rawValue }

    var keyCode: UInt16 {
        switch self {
        case .rightOption: return 61
        case .rightCommand: return 54
        case .rightControl: return 62
        case .fn: return 63
        case .leftOption: return 58
        }
    }

    var flag: NSEvent.ModifierFlags {
        switch self {
        case .rightOption, .leftOption: return .option
        case .rightCommand: return .command
        case .rightControl: return .control
        case .fn: return .function
        }
    }

    var label: String {
        switch self {
        case .rightOption: return "Right Option ⌥"
        case .rightCommand: return "Right Command ⌘"
        case .rightControl: return "Right Control ⌃"
        case .fn: return "Fn / Globe 🌐"
        case .leftOption: return "Left Option ⌥"
        }
    }

    var shortLabel: String {
        switch self {
        case .rightOption: return "Right ⌥"
        case .rightCommand: return "Right ⌘"
        case .rightControl: return "Right ⌃"
        case .fn: return "Fn"
        case .leftOption: return "Left ⌥"
        }
    }
}

/// How the trigger key behaves.
enum RecordMode: String, CaseIterable, Identifiable, Codable {
    /// Hold to talk; a quick tap switches to hands-free until the next tap.
    case smart
    /// Recording lasts only while the key is held.
    case hold
    /// Tap to start, tap again to stop.
    case toggle

    var id: String { rawValue }

    var label: String {
        switch self {
        case .smart: return "Smart — hold to talk, tap for hands-free"
        case .hold: return "Push to talk — hold the key"
        case .toggle: return "Toggle — tap to start, tap to stop"
        }
    }
}

struct Replacement: Codable, Identifiable, Hashable {
    var id = UUID()
    var from: String
    var to: String
}

/// User preferences, persisted in UserDefaults.
final class AppSettings: ObservableObject {
    static let shared = AppSettings()
    private let defaults = UserDefaults.standard

    @Published var triggerKey: TriggerKey { didSet { save(triggerKey.rawValue, "triggerKey") } }
    @Published var recordMode: RecordMode { didSet { save(recordMode.rawValue, "recordMode") } }
    @Published var playSounds: Bool { didSet { save(playSounds, "playSounds") } }
    @Published var addTrailingSpace: Bool { didSet { save(addTrailingSpace, "addTrailingSpace") } }
    @Published var restoreClipboard: Bool { didSet { save(restoreClipboard, "restoreClipboard") } }
    @Published var voiceCommands: Bool { didSet { save(voiceCommands, "voiceCommands") } }
    @Published var keepHistory: Bool { didSet { save(keepHistory, "keepHistory") } }
    @Published var hasCompletedOnboarding: Bool { didSet { save(hasCompletedOnboarding, "onboarded") } }
    @Published var replacements: [Replacement] {
        didSet { defaults.set(try? JSONEncoder().encode(replacements), forKey: "replacements") }
    }

    private init() {
        defaults.register(defaults: [
            "triggerKey": TriggerKey.rightOption.rawValue,
            "recordMode": RecordMode.smart.rawValue,
            "playSounds": true,
            "addTrailingSpace": true,
            "restoreClipboard": true,
            "voiceCommands": true,
            "keepHistory": true,
            "onboarded": false,
        ])
        triggerKey = TriggerKey(rawValue: defaults.string(forKey: "triggerKey") ?? "") ?? .rightOption
        recordMode = RecordMode(rawValue: defaults.string(forKey: "recordMode") ?? "") ?? .smart
        playSounds = defaults.bool(forKey: "playSounds")
        addTrailingSpace = defaults.bool(forKey: "addTrailingSpace")
        restoreClipboard = defaults.bool(forKey: "restoreClipboard")
        voiceCommands = defaults.bool(forKey: "voiceCommands")
        keepHistory = defaults.bool(forKey: "keepHistory")
        hasCompletedOnboarding = defaults.bool(forKey: "onboarded")
        if let data = defaults.data(forKey: "replacements"),
           let list = try? JSONDecoder().decode([Replacement].self, from: data) {
            replacements = list
        } else {
            replacements = []
        }
    }

    private func save(_ value: Any, _ key: String) { defaults.set(value, forKey: key) }
}
