import Foundation

enum NetworkProxy {
    static func listServices() -> [String] {
        let out = run(["-listallnetworkservices"]) ?? ""
        return out.split(separator: "\n")
            .dropFirst() // header line
            .map(String.init)
            .filter { !$0.hasPrefix("*") && !$0.trimmingCharacters(in: .whitespaces).isEmpty }
    }

    static func enable(host: String, port: UInt16) {
        for svc in listServices() {
            _ = run(["-setwebproxy", svc, host, String(port)])
            _ = run(["-setsecurewebproxy", svc, host, String(port)])
            _ = run(["-setwebproxystate", svc, "on"])
            _ = run(["-setsecurewebproxystate", svc, "on"])
        }
    }

    static func disable() {
        for svc in listServices() {
            _ = run(["-setwebproxystate", svc, "off"])
            _ = run(["-setsecurewebproxystate", svc, "off"])
        }
    }

    @discardableResult
    private static func run(_ args: [String]) -> String? {
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/sbin/networksetup")
        proc.arguments = args
        let pipe = Pipe()
        proc.standardOutput = pipe
        proc.standardError = Pipe()
        do {
            try proc.run()
            proc.waitUntilExit()
            if proc.terminationStatus != 0 { return nil }
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            return String(data: data, encoding: .utf8)
        } catch {
            return nil
        }
    }
}

enum CATrust {
    static func isTrusted() -> Bool {
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/bin/security")
        proc.arguments = [
            "find-certificate", "-c", "KeyProxy Local CA",
            "/Library/Keychains/System.keychain",
        ]
        proc.standardOutput = Pipe()
        proc.standardError = Pipe()
        do {
            try proc.run()
            proc.waitUntilExit()
            return proc.terminationStatus == 0
        } catch {
            return false
        }
    }

    static func trust(certPath: URL) throws {
        let script = """
        do shell script "security add-trusted-cert -d -r trustRoot -k /Library/Keychains/System.keychain '\(certPath.path)'" with administrator privileges
        """
        try runOsascript(script)
    }

    static func untrust() throws {
        let script = """
        do shell script "security delete-certificate -c 'KeyProxy Local CA' /Library/Keychains/System.keychain" with administrator privileges
        """
        try runOsascript(script)
    }

    private static func runOsascript(_ src: String) throws {
        let proc = Process()
        proc.executableURL = URL(fileURLWithPath: "/usr/bin/osascript")
        proc.arguments = ["-e", src]
        let err = Pipe()
        proc.standardError = err
        proc.standardOutput = Pipe()
        try proc.run()
        proc.waitUntilExit()
        if proc.terminationStatus != 0 {
            let data = err.fileHandleForReading.readDataToEndOfFile()
            let msg = String(data: data, encoding: .utf8) ?? "osascript failed"
            throw NSError(domain: "KeyProxy", code: Int(proc.terminationStatus),
                          userInfo: [NSLocalizedDescriptionKey: msg])
        }
    }
}
