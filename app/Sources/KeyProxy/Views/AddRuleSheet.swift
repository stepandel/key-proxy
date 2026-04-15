import SwiftUI

struct AddRuleSheet: View {
    let initial: Rule?
    let onSave: (Rule, String?) -> Void

    @Environment(\.dismiss) private var dismiss
    @State private var domain: String
    @State private var label: String
    @State private var headerName: String
    @State private var credential: String = ""

    init(initial: Rule?, onSave: @escaping (Rule, String?) -> Void) {
        self.initial = initial
        self.onSave = onSave
        _domain = State(initialValue: initial?.domain ?? "")
        _label = State(initialValue: initial?.label ?? "")
        _headerName = State(initialValue: initial?.headerName ?? "Authorization")
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text(initial == nil ? "Add rule" : "Edit rule").font(.headline)

            Form {
                TextField("api.anthropic.com", text: $domain)
                    .textFieldStyle(.roundedBorder)
                    .font(.system(.body, design: .monospaced))
                TextField("Label (e.g. Anthropic)", text: $label)
                    .textFieldStyle(.roundedBorder)
                TextField("Header name", text: $headerName)
                    .textFieldStyle(.roundedBorder)
                SecureField(initial == nil ? "Bearer sk-..." : "leave blank to keep existing",
                            text: $credential)
                    .textFieldStyle(.roundedBorder)
            }

            HStack {
                Spacer()
                Button("Cancel") { dismiss() }
                Button("Save") {
                    let rule = Rule(
                        id: initial?.id ?? UUID(),
                        domain: domain.trimmingCharacters(in: .whitespaces),
                        label: label,
                        headerName: headerName,
                        enabled: initial?.enabled ?? true
                    )
                    onSave(rule, credential.isEmpty ? nil : credential)
                    dismiss()
                }
                .buttonStyle(.borderedProminent)
                .disabled(domain.isEmpty || headerName.isEmpty)
            }
        }
        .padding(20)
        .frame(width: 360)
    }
}
