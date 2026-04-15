import Foundation

struct Rule: Identifiable, Codable, Equatable, Hashable {
    var id: UUID
    var domain: String
    var label: String
    var headerName: String
    var enabled: Bool

    init(id: UUID = UUID(), domain: String, label: String, headerName: String, enabled: Bool = true) {
        self.id = id
        self.domain = domain
        self.label = label
        self.headerName = headerName
        self.enabled = enabled
    }

    enum CodingKeys: String, CodingKey {
        case id, domain, label
        case headerName = "header_name"
        case enabled
    }
}

struct ProxyConfig: Codable {
    var port: UInt16
    var rules: [Rule]

    static let defaults = ProxyConfig(port: 7777, rules: [])
}

struct LogEntry: Identifiable, Codable {
    var id = UUID()
    let timestamp: Date
    let domain: String
    let status: Int?
    let latencyMs: Int
    let intercepted: Bool
    let error: String?

    enum CodingKeys: String, CodingKey {
        case timestamp, domain, status, intercepted, error
        case latencyMs = "latency_ms"
    }
}
