import SwiftUI

struct RulesTab: View {
    @EnvironmentObject var controller: ProxyController
    @State private var editing: Rule?
    @State private var adding = false

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Rules").font(.title2).bold()
                Spacer()
                Button {
                    adding = true
                } label: {
                    Label("Add rule", systemImage: "plus")
                }
                .buttonStyle(.borderedProminent)
            }

            if controller.configStore.config.rules.isEmpty {
                ContentUnavailableView("No rules yet",
                                       systemImage: "lock.slash",
                                       description: Text("Add a rule to start injecting credentials into outbound requests."))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else {
                List {
                    ForEach(controller.configStore.config.rules) { rule in
                        RuleRow(rule: rule, onEdit: { editing = rule })
                    }
                }
                .listStyle(.inset)
            }
        }
        .padding(20)
        .sheet(isPresented: $adding) {
            AddRuleSheet(initial: nil) { rule, cred in
                Task { await controller.setRule(rule, credential: cred) }
            }
        }
        .sheet(item: $editing) { rule in
            AddRuleSheet(initial: rule) { r, cred in
                Task { await controller.setRule(r, credential: cred) }
            }
        }
    }
}

private struct RuleRow: View {
    let rule: Rule
    let onEdit: () -> Void
    @EnvironmentObject var controller: ProxyController

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(rule.domain).font(.system(.body, design: .monospaced))
                Text("\(rule.label.isEmpty ? "—" : rule.label) · \(rule.headerName)")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
            Spacer()
            Toggle("", isOn: Binding(
                get: { rule.enabled },
                set: { v in Task { await controller.toggleRule(rule.id, enabled: v) } }
            ))
            .toggleStyle(.switch)
            .labelsHidden()
            Button("Edit", action: onEdit)
            Button(role: .destructive) {
                Task { await controller.deleteRule(rule.id) }
            } label: {
                Image(systemName: "trash")
            }
        }
    }
}
