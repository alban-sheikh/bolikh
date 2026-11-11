import Foundation

struct HistoryItem: Codable, Identifiable, Hashable {
    var id = UUID()
    var text: String
    var date: Date
    var app: String?
    var duration: Double
}

/// Recent dictations, stored only on this Mac (~/Library/Application Support/AazadVoice/history.json).
@MainActor
final class HistoryStore: ObservableObject {
    static let shared = HistoryStore()
    @Published private(set) var items: [HistoryItem] = []
    private let limit = 200

    private var url: URL {
        let dir = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("AazadVoice", isDirectory: true)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        return dir.appendingPathComponent("history.json")
    }

    private init() {
        if let data = try? Data(contentsOf: url),
           let list = try? JSONDecoder().decode([HistoryItem].self, from: data) { items = list }
    }

    var totalWords: Int { items.reduce(0) { $0 + $1.text.split(whereSeparator: \.isWhitespace).count } }

    func add(_ item: HistoryItem) {
        items.insert(item, at: 0)
        if items.count > limit { items.removeLast(items.count - limit) }
        persist()
    }

    func clear() { items.removeAll(); persist() }

    private func persist() {
        if let data = try? JSONEncoder().encode(items) { try? data.write(to: url, options: .atomic) }
    }
}
