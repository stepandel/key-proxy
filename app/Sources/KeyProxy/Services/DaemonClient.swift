import Foundation
import Network
import OSLog

/// Manages the lifecycle of `keyproxyd` and communicates over a Unix socket.
@MainActor
final class DaemonClient: ObservableObject {
    @Published private(set) var connected: Bool = false
    @Published private(set) var running: Bool = false
    @Published private(set) var logs: [LogEntry] = []
    @Published private(set) var lastError: String?

    private let logger = Logger(subsystem: "app.keyproxy", category: "daemon")
    private var process: Process?
    private var connection: NWConnection?
    private var readBuffer = Data()
    private let socketURL: URL
    private var nextId: UInt64 = 1
    private var pending: [UInt64: (Result<DaemonEvent, Error>) -> Void] = [:]

    init(appSupport: URL) {
        self.socketURL = appSupport.appendingPathComponent("daemon.sock")
    }

    // MARK: - lifecycle

    func startDaemonIfNeeded() async {
        if process != nil, process?.isRunning == true { return }
        guard let binURL = resolveDaemonBinary() else {
            lastError = "keyproxyd binary not found in app bundle"
            return
        }
        // Clean up stale socket
        try? FileManager.default.removeItem(at: socketURL)

        let proc = Process()
        proc.executableURL = binURL
        proc.arguments = [socketURL.path]
        proc.terminationHandler = { [weak self] _ in
            Task { @MainActor in
                self?.connected = false
                self?.running = false
            }
        }
        do {
            try proc.run()
            self.process = proc
            logger.info("spawned keyproxyd pid=\(proc.processIdentifier)")
        } catch {
            lastError = "failed to spawn daemon: \(error.localizedDescription)"
            return
        }

        // Wait for socket to appear, up to ~2s
        for _ in 0..<40 {
            if FileManager.default.fileExists(atPath: socketURL.path) { break }
            try? await Task.sleep(nanoseconds: 50_000_000)
        }
        await connect()
    }

    func stopDaemon() async {
        _ = try? await send(.stop)
        connection?.cancel()
        connection = nil
        connected = false
        if let proc = process {
            proc.terminate()
        }
        process = nil
        running = false
    }

    // MARK: - commands

    func configure(rules: [Rule], credentials: [UUID: String], caKeyPEM: String, caCertPEM: String) async throws {
        _ = try await send(.setCa(keyPEM: caKeyPEM, certPEM: caCertPEM))
        let dtos: [RuleDto] = rules.compactMap { r in
            guard r.enabled, let cred = credentials[r.id] else { return nil }
            return RuleDto(domain: r.domain, headerName: r.headerName, credential: cred)
        }
        _ = try await send(.setRules(dtos))
    }

    func start(port: UInt16) async throws {
        _ = try await send(.start(port: port))
        running = true
    }

    func stop() async throws {
        _ = try await send(.stop)
        running = false
    }

    func subscribeLogs() async throws {
        _ = try await send(.subscribeLogs)
    }

    func generateCA() async throws -> (keyPEM: String, certPEM: String) {
        let ev = try await send(.generateCa)
        guard case let .ca(_, keyPEM, certPEM) = ev else {
            throw DaemonError.protocolError("expected ca event")
        }
        return (keyPEM, certPEM)
    }

    // MARK: - private

    private func resolveDaemonBinary() -> URL? {
        // Prefer bundled binary in .app/Contents/Resources/keyproxyd
        let bundle = Bundle.main
        if let url = bundle.url(forResource: "keyproxyd", withExtension: nil) {
            return url
        }
        // Fallback: sibling to the main executable (dev builds)
        if let exe = bundle.executableURL {
            let sibling = exe.deletingLastPathComponent().appendingPathComponent("keyproxyd")
            if FileManager.default.isExecutableFile(atPath: sibling.path) { return sibling }
        }
        // Dev fallback: assume daemon/target/release/keyproxyd
        let cwd = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let dev = cwd.appendingPathComponent("daemon/target/release/keyproxyd")
        if FileManager.default.isExecutableFile(atPath: dev.path) { return dev }
        let dev2 = cwd.appendingPathComponent("daemon/target/debug/keyproxyd")
        if FileManager.default.isExecutableFile(atPath: dev2.path) { return dev2 }
        return nil
    }

    private func connect() async {
        let endpoint = NWEndpoint.unix(path: socketURL.path)
        let conn = NWConnection(to: endpoint, using: .tcp)
        self.connection = conn
        conn.stateUpdateHandler = { [weak self] state in
            Task { @MainActor in
                switch state {
                case .ready:
                    self?.connected = true
                    self?.receiveLoop()
                    try? await self?.subscribeLogs()
                case .failed(let e):
                    self?.connected = false
                    self?.lastError = "socket failed: \(e.localizedDescription)"
                case .cancelled:
                    self?.connected = false
                default:
                    break
                }
            }
        }
        conn.start(queue: .main)
    }

    private func receiveLoop() {
        connection?.receive(minimumIncompleteLength: 1, maximumLength: 64 * 1024) { [weak self] data, _, isComplete, error in
            guard let self else { return }
            if let data, !data.isEmpty {
                Task { @MainActor in self.ingest(data) }
            }
            if let error {
                Task { @MainActor in
                    self.lastError = "recv: \(error.localizedDescription)"
                    self.connected = false
                }
                return
            }
            if isComplete {
                Task { @MainActor in self.connected = false }
                return
            }
            Task { @MainActor in self.receiveLoop() }
        }
    }

    private func ingest(_ data: Data) {
        readBuffer.append(data)
        while let nl = readBuffer.firstIndex(of: 0x0A) {
            let line = readBuffer.subdata(in: readBuffer.startIndex..<nl)
            readBuffer.removeSubrange(readBuffer.startIndex...nl)
            if line.isEmpty { continue }
            do {
                let ev = try JSONDecoder.snake.decode(DaemonEvent.self, from: line)
                handle(ev)
            } catch {
                logger.error("decode error: \(error.localizedDescription) line=\(String(data: line, encoding: .utf8) ?? "?")")
            }
        }
    }

    private func handle(_ ev: DaemonEvent) {
        switch ev {
        case .log(let entry):
            logs.insert(entry, at: 0)
            if logs.count > 200 { logs.removeLast(logs.count - 200) }
        case .ok(let id), .pong(let id):
            if let id, let cb = pending.removeValue(forKey: id) { cb(.success(ev)) }
        case .ca(let id, _, _):
            if let id, let cb = pending.removeValue(forKey: id) { cb(.success(ev)) }
        case .error(let id, let msg):
            lastError = msg
            if let id, let cb = pending.removeValue(forKey: id) {
                cb(.failure(DaemonError.remote(msg)))
            }
        }
    }

    private func send(_ cmd: DaemonCommand) async throws -> DaemonEvent {
        guard let conn = connection, connected else {
            throw DaemonError.notConnected
        }
        let id = nextId; nextId += 1
        let envelope = CommandEnvelope(id: id, command: cmd)
        let json = try JSONEncoder.snake.encode(envelope)
        var line = json
        line.append(0x0A)

        return try await withCheckedThrowingContinuation { cont in
            pending[id] = { result in
                switch result {
                case .success(let ev): cont.resume(returning: ev)
                case .failure(let e): cont.resume(throwing: e)
                }
            }
            conn.send(content: line, completion: .contentProcessed { err in
                if let err {
                    Task { @MainActor in
                        self.pending.removeValue(forKey: id)
                        cont.resume(throwing: err)
                    }
                }
            })
        }
    }
}

// MARK: - Protocol types

enum DaemonError: LocalizedError {
    case notConnected
    case protocolError(String)
    case remote(String)

    var errorDescription: String? {
        switch self {
        case .notConnected: return "Not connected to daemon"
        case .protocolError(let m): return "Protocol error: \(m)"
        case .remote(let m): return m
        }
    }
}

struct RuleDto: Codable {
    let domain: String
    let headerName: String
    let credential: String

    enum CodingKeys: String, CodingKey {
        case domain, credential
        case headerName = "header_name"
    }
}

enum DaemonCommand {
    case generateCa
    case setCa(keyPEM: String, certPEM: String)
    case setRules([RuleDto])
    case start(port: UInt16)
    case stop
    case ping
    case subscribeLogs
}

struct CommandEnvelope: Encodable {
    let id: UInt64
    let command: DaemonCommand

    private enum CodingKeys: String, CodingKey {
        case id, cmd, key_pem, cert_pem, rules, port
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        try c.encode(id, forKey: .id)
        switch command {
        case .generateCa:
            try c.encode("generate_ca", forKey: .cmd)
        case .setCa(let k, let ce):
            try c.encode("set_ca", forKey: .cmd)
            try c.encode(k, forKey: .key_pem)
            try c.encode(ce, forKey: .cert_pem)
        case .setRules(let rs):
            try c.encode("set_rules", forKey: .cmd)
            try c.encode(rs, forKey: .rules)
        case .start(let port):
            try c.encode("start", forKey: .cmd)
            try c.encode(port, forKey: .port)
        case .stop:
            try c.encode("stop", forKey: .cmd)
        case .ping:
            try c.encode("ping", forKey: .cmd)
        case .subscribeLogs:
            try c.encode("subscribe_logs", forKey: .cmd)
        }
    }
}

enum DaemonEvent: Decodable {
    case ok(id: UInt64?)
    case error(id: UInt64?, message: String)
    case pong(id: UInt64?)
    case log(LogEntry)
    case ca(id: UInt64?, keyPEM: String, certPEM: String)

    private enum Keys: String, CodingKey {
        case type, id, message
        case key_pem, cert_pem
        case timestamp, domain, status, latency_ms, intercepted, error
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: Keys.self)
        let type = try c.decode(String.self, forKey: .type)
        switch type {
        case "ok":
            self = .ok(id: try c.decodeIfPresent(UInt64.self, forKey: .id))
        case "pong":
            self = .pong(id: try c.decodeIfPresent(UInt64.self, forKey: .id))
        case "error":
            self = .error(
                id: try c.decodeIfPresent(UInt64.self, forKey: .id),
                message: try c.decode(String.self, forKey: .message)
            )
        case "log":
            let entry = try LogEntry(from: decoder)
            self = .log(entry)
        case "ca":
            self = .ca(
                id: try c.decodeIfPresent(UInt64.self, forKey: .id),
                keyPEM: try c.decode(String.self, forKey: .key_pem),
                certPEM: try c.decode(String.self, forKey: .cert_pem)
            )
        default:
            throw DecodingError.dataCorruptedError(
                forKey: .type, in: c, debugDescription: "unknown event type \(type)"
            )
        }
    }
}

extension JSONDecoder {
    static let snake: JSONDecoder = {
        let d = JSONDecoder()
        d.dateDecodingStrategy = .iso8601
        return d
    }()
}

extension JSONEncoder {
    static let snake: JSONEncoder = {
        let e = JSONEncoder()
        e.dateEncodingStrategy = .iso8601
        return e
    }()
}
