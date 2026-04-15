import SwiftUI

struct GeneralTab: View {
    @EnvironmentObject var controller: ProxyController
    @State private var portText: String = "7777"
    @State private var busy = false

    var body: some View {
        Form {
            Section("Proxy") {
                LabeledContent("Port") {
                    TextField("7777", text: $portText, onCommit: commitPort)
                        .frame(width: 80)
                        .textFieldStyle(.roundedBorder)
                        .onAppear { portText = String(controller.configStore.config.port) }
                }
                LabeledContent("Status") {
                    HStack {
                        Circle()
                            .fill(controller.daemon.running ? .green : .gray)
                            .frame(width: 8, height: 8)
                        Text(controller.status)
                    }
                }
            }

            Section("Certificate Authority") {
                LabeledContent("Trust") {
                    HStack {
                        Circle()
                            .fill(controller.caTrusted ? .green : .red)
                            .frame(width: 8, height: 8)
                        Text(controller.caTrusted ? "Trusted by system" : "Not trusted")
                    }
                }
                HStack {
                    Button("Trust CA") {
                        busy = true
                        Task { await controller.trustCA(); busy = false }
                    }
                    .disabled(busy || controller.caTrusted)

                    Button("Remove Trust") {
                        busy = true
                        Task { await controller.untrustCA(); busy = false }
                    }
                    .disabled(busy || !controller.caTrusted)

                    Button("Export…") { exportCA() }
                    Button("Regenerate") {
                        guard promptConfirm("Regenerate CA? Existing trust will be invalidated.") else { return }
                        busy = true
                        Task { await controller.regenerateCA(); busy = false }
                    }
                    .disabled(busy)
                }
            }
        }
        .formStyle(.grouped)
        .padding(20)
    }

    private func commitPort() {
        guard let p = UInt16(portText) else { return }
        controller.configStore.setPort(p)
        Task { await controller.pushCurrentConfig() }
    }

    private func exportCA() {
        let panel = NSSavePanel()
        panel.nameFieldStringValue = "keyproxy-ca.pem"
        panel.allowedContentTypes = [.init(filenameExtension: "pem")!]
        if panel.runModal() == .OK, let url = panel.url {
            try? controller.exportCA(to: url)
        }
    }

    private func promptConfirm(_ message: String) -> Bool {
        let alert = NSAlert()
        alert.messageText = message
        alert.addButton(withTitle: "Yes")
        alert.addButton(withTitle: "Cancel")
        return alert.runModal() == .alertFirstButtonReturn
    }
}
