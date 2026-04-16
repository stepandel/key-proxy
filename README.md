# KeyProxy

Credential-injecting HTTPS proxy for macOS. Stores API keys in the Keychain and injects them into outbound requests to whitelisted domains — transparently, for every process on the machine.

## Architecture

Two components:

- **`keyproxyd`** — Rust daemon. Runs the HTTPS proxy (TCP listener + CONNECT routing + TLS interception via rustls + blind-tunnel fallback). State is fully runtime-injected over a Unix socket. Holds no persistent data.
- **`KeyProxy.app`** — SwiftUI menu bar app. Owns the Keychain, the rule config file, CA trust prompts, and system proxy configuration. Spawns and talks to `keyproxyd` over a line-delimited JSON socket protocol. Fails closed: when the app quits, the daemon exits and the system proxy is cleared.

```
┌─────────────────────────────┐      spawns / IPC      ┌──────────────────────┐
│  KeyProxy.app (SwiftUI)     │ ──────────────────────▶│  keyproxyd (Rust)    │
│  · Keychain (API keys + CA) │   Unix socket 0600     │  · TCP :7777         │
│  · config.json              │   line-delimited JSON  │  · CONNECT routing   │
│  · networksetup control     │                        │  · TLS terminate     │
│  · CA trust dialog          │                        │  · blind tunnel      │
└─────────────────────────────┘                        └──────────────────────┘
```

See the spec at the top of the conversation for the end-to-end problem/solution framing.

## Requirements

- macOS 14+ (Sonoma)
- Rust stable (1.77+)
- Swift 5.9+ (Xcode 15 CLT or Xcode 15+)

## Build & run

```bash
./app/scripts/build-app.sh                 # release build
CONFIG=debug ./app/scripts/build-app.sh    # debug build

open build/KeyProxy.app
```

The build script:
1. Runs `swift build` for the app.
2. Runs `cargo build` for the daemon.
3. Assembles `build/KeyProxy.app` with the daemon at `Contents/Resources/keyproxyd`.

### First launch

1. App generates a local CA (by asking the daemon) and stores the private key in your login Keychain.
2. Open Settings → General → **Trust CA** — enter your admin password when prompted.
3. Add a rule: domain, header name, credential. Credential is write-only from the UI (it goes straight to Keychain).
4. Click **Start** in the menu bar popover. System HTTPS proxy is set to `127.0.0.1:7777`.

### Verify

```bash
# succeeds — credentials injected by the proxy
curl -v https://api.anthropic.com/v1/messages

# passes through untouched — blind tunnel, no interception
curl -v https://example.com
```

## File layout

```
key-proxy/
├── daemon/                         # Rust daemon
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                 # entry + socket path
│       ├── ipc.rs                  # JSON IPC protocol
│       ├── state.rs                # runtime rule/CA state
│       ├── stats.rs                # log entry shape
│       └── proxy/
│           ├── mod.rs              # listener + shutdown
│           ├── connect.rs          # CONNECT routing
│           ├── intercept.rs        # TLS terminate + header inject + forward
│           ├── tunnel.rs           # copy_bidirectional
│           └── cert.rs             # CA gen, per-domain leaf cert cache
└── app/                            # SwiftUI app
    ├── Package.swift
    ├── Resources/Info.plist        # LSUIElement = menu bar app
    ├── scripts/build-app.sh        # assembles .app from SwiftPM binary + daemon
    └── Sources/KeyProxy/
        ├── KeyProxyApp.swift       # @main, MenuBarExtra + Settings scene
        ├── Models/Rule.swift
        ├── Services/
        │   ├── KeychainStore.swift     # SecItem wrapper (device-only access)
        │   ├── ConfigStore.swift       # ~/Library/Application Support/KeyProxy/config.json
        │   ├── NetworkProxy.swift      # networksetup + CA trust via osascript
        │   ├── DaemonClient.swift      # NWConnection unix socket + IPC
        │   └── ProxyController.swift   # top-level ObservableObject
        └── Views/
            ├── MenuBarView.swift
            ├── SettingsView.swift
            ├── RulesTab.swift
            ├── GeneralTab.swift
            ├── StatsTab.swift
            └── AddRuleSheet.swift
```

## IPC protocol (daemon)

Line-delimited JSON on `~/Library/Application Support/KeyProxy/daemon.sock` (mode 0600).

**Client → daemon:**

```json
{"id": 1, "cmd": "generate_ca"}
{"id": 2, "cmd": "set_ca", "key_pem": "...", "cert_pem": "..."}
{"id": 3, "cmd": "set_rules", "rules": [{"domain": "api.anthropic.com", "header_name": "x-api-key", "credential": "..."}]}
{"id": 4, "cmd": "start", "port": 7777}
{"id": 5, "cmd": "stop"}
{"id": 6, "cmd": "subscribe_logs"}
{"id": 7, "cmd": "ping"}
```

**Daemon → client:**

```json
{"type": "ok", "id": 1}
{"type": "error", "id": 2, "message": "..."}
{"type": "ca", "id": 1, "key_pem": "...", "cert_pem": "..."}
{"type": "pong", "id": 7}
{"type": "log", "timestamp": "2026-04-15T12:34:56Z", "domain": "api.anthropic.com", "status": 200, "latency_ms": 12, "intercepted": true, "error": null}
```

## Security properties

| Property | Mechanism |
|---|---|
| Keys never on disk as plaintext | macOS Keychain (`kSecAttrAccessibleWhenUnlockedThisDeviceOnly`) |
| Daemon holds secrets in memory only | `keyproxyd` receives rules/CA at startup, never touches disk |
| IPC socket not world-readable | Unix socket with 0600 permissions |
| Non-whitelisted traffic untouched | Blind TCP tunnel — no TLS termination |
| No persistent request logging | In-memory only; cleared on daemon exit |
| Proxy fails closed | Daemon shuts down when Swift side disconnects; system proxy cleared on app quit |

## Recovering from a stuck proxy

If something goes wrong and HTTPS on the whole machine stops working (dead daemon, force-quit, uninstalled app with proxy still set), run the daemon binary directly with `--unset`:

```bash
./build/KeyProxy.app/Contents/Resources/keyproxyd --unset
# or if installed in /Applications:
/Applications/KeyProxy.app/Contents/Resources/keyproxyd --unset
```

This disables the system HTTPS proxy on every network interface and exits. Always safe to run. The app also does this automatically on launch if it detects a leftover proxy from a previous crash, and on quit via `applicationWillTerminate`.

## Notes

- The `.app` currently isn't code-signed. For a signed build, add a signing step after `cp` in `scripts/build-app.sh` (`codesign --sign "Developer ID Application: …" --deep build/KeyProxy.app`).
- The menu bar uses an SF Symbol (`key` / `key.fill`). Replace with a custom template image by bundling a PNG and using `Image(nsImage:)` if you want your own icon.
- Only HTTPS is supported (CONNECT tunnels). Plain HTTP isn't a goal — credentials shouldn't travel over cleartext anyway.
