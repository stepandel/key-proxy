import Foundation

@MainActor
final class ConfigStore: ObservableObject {
    @Published var config: ProxyConfig

    private let url: URL

    init() {
        let fm = FileManager.default
        let base = try! fm.url(
            for: .applicationSupportDirectory,
            in: .userDomainMask,
            appropriateFor: nil,
            create: true
        ).appendingPathComponent("KeyProxy", isDirectory: true)
        try? fm.createDirectory(at: base, withIntermediateDirectories: true)
        self.url = base.appendingPathComponent("config.json")
        if let data = try? Data(contentsOf: url),
           let decoded = try? JSONDecoder().decode(ProxyConfig.self, from: data) {
            self.config = decoded
        } else {
            self.config = .defaults
        }
    }

    func save() {
        let enc = JSONEncoder()
        enc.outputFormatting = [.prettyPrinted, .sortedKeys]
        guard let data = try? enc.encode(config) else { return }
        try? data.write(to: url, options: .atomic)
    }

    var appSupportDirectory: URL { url.deletingLastPathComponent() }

    // MARK: - mutations

    func upsert(_ rule: Rule) {
        if let idx = config.rules.firstIndex(where: { $0.id == rule.id }) {
            config.rules[idx] = rule
        } else {
            config.rules.append(rule)
        }
        save()
    }

    func delete(_ id: UUID) {
        config.rules.removeAll { $0.id == id }
        save()
    }

    func toggle(_ id: UUID, enabled: Bool) {
        if let idx = config.rules.firstIndex(where: { $0.id == id }) {
            config.rules[idx].enabled = enabled
            save()
        }
    }

    func setPort(_ port: UInt16) {
        config.port = port
        save()
    }
}
