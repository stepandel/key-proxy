# KeyProxy

Credential-injecting HTTPS proxy for macOS. Stores API keys in the Keychain and injects them into outbound requests to whitelisted domains — transparently, for every process on the machine.

## How it works

1. KeyProxy listens on `127.0.0.1:7777` and registers itself as the macOS system HTTPS proxy.
2. Every outbound HTTPS connection on the machine arrives as a `CONNECT` request.
3. If the destination host matches an **enabled rule**, KeyProxy:
   - TLS-terminates the inbound connection using a leaf cert signed by its local CA
   - Reads request headers and injects the credential from Keychain (e.g. `Authorization: Bearer …`)
   - Opens a fresh TLS connection to the real upstream, forwards the request, streams the response back
4. Everything else passes through as a **blind TCP tunnel** — never decrypted, never inspected.

The calling process never sees the proxy. It dials `api.anthropic.com:443` and gets a real response with credentials silently attached.

## Requirements

- macOS (Apple Silicon or Intel)
- Rust stable (1.77+)
- Node 18+ and npm
- Tauri CLI: `npm install` then `npm run tauri`

## Build & run (dev)

```
npm install
npm run tauri dev
```

First launch:
1. App generates a local CA and stores the private key in Keychain.
2. Click **Trust CA Certificate** in Settings → General. You'll be prompted for your admin password.
3. Add a rule (Settings → Rules → Add rule). Enter the domain, header name, and credential.
4. Click **Start** in the menu bar panel. System proxy is now set.

Verify:
```
curl -v https://api.anthropic.com/v1/messages
# Should succeed without ANTHROPIC_API_KEY in your environment.

curl -v https://example.com
# Should pass through untouched (blind tunnel).
```

## Build release

```
npm run tauri build
```

Generates `.app` and `.dmg` in `src-tauri/target/release/bundle/`.

## File layout

- `src-tauri/src/proxy/` — listener, CONNECT routing, TLS intercept, blind tunnel, CA + leaf cert cache
- `src-tauri/src/keychain.rs` — secret storage via `security-framework`
- `src-tauri/src/config.rs` — non-secret rule metadata (JSON in `~/Library/Application Support/KeyProxy/`)
- `src-tauri/src/network.rs` — `networksetup` calls to set/unset system proxy
- `src-tauri/src/stats.rs` — in-memory ring buffer (never persisted)
- `src-tauri/src/commands.rs` — Tauri IPC surface
- `src/` — React UI (menu bar panel + settings window)

## Security properties

| Property | Mechanism |
|---|---|
| Keys never on disk as plaintext | macOS Keychain |
| CA private key hardware-bound, device-only | `kSecAttrAccessibleWhenUnlockedThisDeviceOnly` via Keychain |
| Non-whitelisted traffic untouched | Blind TCP tunnel — no TLS termination |
| No persistent request logging | In-memory ring buffer only, cleared on app quit |
| Proxy fails closed | System proxy disabled on exit |

## Notes

- The included icons are 1×1 transparent placeholders. Replace `src-tauri/icons/*.png` with a real icon set before shipping, or run `npx @tauri-apps/cli icon path/to/source.png`.
- Only HTTPS interception is supported (CONNECT tunnels). Plain HTTP is not a goal — credentials shouldn't travel over cleartext anyway.
- The CA trust uses `security add-trusted-cert` via an admin `osascript` prompt. Remove trust at any time from Settings → General → Remove Trust.
