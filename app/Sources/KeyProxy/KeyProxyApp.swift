import SwiftUI

@main
struct KeyProxyApp: App {
    @StateObject private var controller = ProxyController()

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
