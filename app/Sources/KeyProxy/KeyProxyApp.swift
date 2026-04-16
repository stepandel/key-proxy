import SwiftUI

@main
struct KeyProxyApp: App {
    @StateObject private var controller = ProxyController()
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    init() {
        NSApplication.shared.setActivationPolicy(.accessory)
    }

    var body: some Scene {
        MenuBarExtra {
            MenuBarView()
                .environmentObject(controller)
                .task {
                    await controller.onAppLaunch()
                }
        } label: {
            Image(systemName: controller.daemon.running ? "key.fill" : "key")
                .symbolRenderingMode(.hierarchical)
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView()
                .environmentObject(controller)
        }
    }
}

/// AppKit delegate for termination hooks. SwiftUI's `.onDisappear` isn't
/// reliable for app quit, so we wire AppKit notifications directly.
final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationWillTerminate(_ notification: Notification) {
        // Runs synchronously on the main thread before the process exits.
        // Keep this strictly sync — async tasks won't complete.
        NetworkProxy.disable()
    }
}
