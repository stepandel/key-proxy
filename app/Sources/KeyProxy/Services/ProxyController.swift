import Foundation

/// Coordinates ConfigStore + Keychain + CA + DaemonClient so views interact
/// with a single top-level state object.
@MainActor
final class ProxyController: ObservableObject {
    @Published var configStore = ConfigStore()
    @Published var daemon: DaemonClient
    @Published var caTrusted: Bool = false
    @Published var status: String = "Paused"

    init() {
        let base = FileManager.default
            .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
            .appendingPathComponent("KeyProxy", isDirectory: true)
        try? FileManager.default.createDirectory(at: base, withIntermediateDirectories: true)
        self.daemon = DaemonClient(appSupport: base)
    }

    // MARK: - lifecycle

    func onAppLaunch() async {
        caTrusted = CATrust.isTrusted()
        await daemon.startDaemonIfNeeded()
        await ensureCAExists()
        await pushCurrentConfig()
    }

    func onAppQuit() async {
        NetworkProxy.disable()
        await daemon.stopDaemon()
    }

    // MARK: - actions

    func toggleProxy() async {
        if daemon.running {
            NetworkProxy.disable()
            try? await daemon.stop()
            status = "Paused"
        } else {
            await pushCurrentConfig()
            do {
                try await daemon.start(port: configStore.config.port)
                NetworkProxy.enable(host: "127.0.0.1", port: configStore.config.port)
                status = "Active"
            } catch {
                status = "Error: \(error.localizedDescription)"
            }
        }
    }

    func pushCurrentConfig() async {
        guard let ca = KeychainStore.loadCA() else { return }
        let credentials: [UUID: String] = Dictionary(uniqueKeysWithValues:
            configStore.config.rules.compactMap { r in
                guard let v = KeychainStore.getCredential(domain: r.domain) else { return nil }
                return (r.id, v)
            }
        )
        do {
            try await daemon.configure(
                rules: configStore.config.rules,
                credentials: credentials,
                caKeyPEM: ca.keyPEM,
                caCertPEM: ca.certPEM
            )
        } catch {
            status = "Config push failed: \(error.localizedDescription)"
        }
    }

    func setRule(_ rule: Rule, credential: String?) async {
        configStore.upsert(rule)
        if let cred = credential, !cred.isEmpty {
            try? KeychainStore.setCredential(domain: rule.domain, value: cred)
        }
        await pushCurrentConfig()
    }

    func deleteRule(_ id: UUID) async {
        if let r = configStore.config.rules.first(where: { $0.id == id }) {
            try? KeychainStore.deleteCredential(domain: r.domain)
        }
        configStore.delete(id)
        await pushCurrentConfig()
    }

    func toggleRule(_ id: UUID, enabled: Bool) async {
        configStore.toggle(id, enabled: enabled)
        await pushCurrentConfig()
    }

    // MARK: - CA

    private func ensureCAExists() async {
        if KeychainStore.loadCA() != nil { return }
        do {
            let ca = try await daemon.generateCA()
            try KeychainStore.saveCA(keyPEM: ca.keyPEM, certPEM: ca.certPEM)
        } catch {
            status = "CA generation failed: \(error.localizedDescription)"
        }
    }

    func exportCA(to path: URL) throws {
        guard let ca = KeychainStore.loadCA() else {
            throw NSError(domain: "KeyProxy", code: 1, userInfo: [NSLocalizedDescriptionKey: "No CA"])
        }
        try ca.certPEM.write(to: path, atomically: true, encoding: .utf8)
    }

    func caCertTempURL() throws -> URL {
        guard let ca = KeychainStore.loadCA() else {
            throw NSError(domain: "KeyProxy", code: 1, userInfo: [NSLocalizedDescriptionKey: "No CA"])
        }
        let url = configStore.appSupportDirectory.appendingPathComponent("ca.pem")
        try ca.certPEM.write(to: url, atomically: true, encoding: .utf8)
        return url
    }

    func trustCA() async {
        do {
            let url = try caCertTempURL()
            try CATrust.trust(certPath: url)
            caTrusted = CATrust.isTrusted()
        } catch {
            status = "Trust failed: \(error.localizedDescription)"
        }
    }

    func untrustCA() async {
        do {
            try CATrust.untrust()
            caTrusted = CATrust.isTrusted()
        } catch {
            status = "Untrust failed: \(error.localizedDescription)"
        }
    }

    func regenerateCA() async {
        try? KeychainStore.deleteCA()
        await ensureCAExists()
        await pushCurrentConfig()
    }
}
