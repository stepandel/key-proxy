import SwiftUI

struct MenuBarView: View {
    @EnvironmentObject var controller: ProxyController
    @State private var showingAddRule = false

    var body: some View {
        VStack(spacing: 0) {
            header
            Divider()
            if controller.configStore.config.rules.isEmpty {
                emptyState
            } else {
                ruleList
            }
            Divider()
            footer
        }
        .frame(width: 320)
        .sheet(isPresented: $showingAddRule) {
            AddRuleSheet(initial: nil) { rule, cred in
                Task { await controller.setRule(rule, credential: cred) }
            }
        }
    }

    private var header: some View {
        HStack {
            Text("KeyProxy").font(.headline)
            Spacer()
            StatusDot(running: controller.daemon.running,
                      allEnabled: controller.configStore.config.rules.allSatisfy(\.enabled))
            Text(controller.status).font(.caption).foregroundStyle(.secondary)
        }
        .padding(12)
    }

    private var emptyState: some View {
        VStack(spacing: 8) {
            Text("No rules yet")
                .font(.subheadline)
                .foregroundStyle(.secondary)
            Button("Add rule") { showingAddRule = true }
                .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity)
        .padding(20)
    }

    private var ruleList: some View {
        ScrollView {
            LazyVStack(spacing: 0) {
                ForEach(controller.configStore.config.rules) { rule in
                    HStack {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(rule.domain)
                                .font(.system(.caption, design: .monospaced))
                            Text(rule.label.isEmpty ? rule.headerName : "\(rule.label) · \(rule.headerName)")
                                .font(.caption2)
                                .foregroundStyle(.secondary)
                        }
                        Spacer()
                        Toggle("", isOn: Binding(
                            get: { rule.enabled },
                            set: { v in
                                Task { await controller.toggleRule(rule.id, enabled: v) }
                            }
                        ))
                        .toggleStyle(.switch)
                        .labelsHidden()
                    }
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    Divider().opacity(0.3)
                }
            }
        }
        .frame(maxHeight: 260)
    }

    private var footer: some View {
        HStack {
            Button("+ Rule") { showingAddRule = true }
                .buttonStyle(.plain)
                .font(.caption)
            Spacer()
            Button("Settings…") {
                NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
                NSApp.activate(ignoringOtherApps: true)
            }
            .buttonStyle(.plain)
            .font(.caption)
            Button(controller.daemon.running ? "Pause" : "Start") {
                Task { await controller.toggleProxy() }
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.small)
            Button("Quit") { NSApp.terminate(nil) }
                .buttonStyle(.plain)
                .font(.caption)
        }
        .padding(10)
    }
}

struct StatusDot: View {
    let running: Bool
    let allEnabled: Bool

    var body: some View {
        Circle()
            .fill(color)
            .frame(width: 8, height: 8)
    }

    private var color: Color {
        if !running { return .gray }
        return allEnabled ? .green : .yellow
    }
}
