import Foundation
import Security

enum KeychainError: Error {
    case unhandled(OSStatus)
}

enum KeychainStore {
    private static let service = "keyproxy"
    private static let caService = "keyproxy-ca"
    private static let caKeyAccount = "ca-private-key"
    private static let caCertAccount = "ca-certificate"

    static func setCredential(domain: String, value: String) throws {
        try set(service: service, account: domain, value: value)
    }

    static func getCredential(domain: String) -> String? {
        try? get(service: service, account: domain)
    }

    static func deleteCredential(domain: String) throws {
        try delete(service: service, account: domain)
    }

    static func saveCA(keyPEM: String, certPEM: String) throws {
        try set(service: caService, account: caKeyAccount, value: keyPEM)
        try set(service: caService, account: caCertAccount, value: certPEM)
    }

    static func loadCA() -> (keyPEM: String, certPEM: String)? {
        guard let k = try? get(service: caService, account: caKeyAccount),
              let c = try? get(service: caService, account: caCertAccount)
        else { return nil }
        return (k, c)
    }

    static func deleteCA() throws {
        try? delete(service: caService, account: caKeyAccount)
        try? delete(service: caService, account: caCertAccount)
    }

    // MARK: - primitives

    private static func set(service: String, account: String, value: String) throws {
        let data = Data(value.utf8)
        let baseQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
        let attrs: [String: Any] = [
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
        ]
        let status = SecItemUpdate(baseQuery as CFDictionary, attrs as CFDictionary)
        if status == errSecItemNotFound {
            var add = baseQuery
            add.merge(attrs) { _, new in new }
            let addStatus = SecItemAdd(add as CFDictionary, nil)
            guard addStatus == errSecSuccess else { throw KeychainError.unhandled(addStatus) }
        } else if status != errSecSuccess {
            throw KeychainError.unhandled(status)
        }
    }

    private static func get(service: String, account: String) throws -> String {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne,
        ]
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        guard status == errSecSuccess, let data = result as? Data,
              let str = String(data: data, encoding: .utf8)
        else { throw KeychainError.unhandled(status) }
        return str
    }

    private static func delete(service: String, account: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
        ]
        let status = SecItemDelete(query as CFDictionary)
        if status != errSecSuccess && status != errSecItemNotFound {
            throw KeychainError.unhandled(status)
        }
    }
}
