import SwiftUI

struct StatsTab: View {
    @EnvironmentObject var controller: ProxyController

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 20) {
                stat(label: "Requests", value: "\(controller.daemon.logs.count)")
                stat(label: "Errors",
                     value: "\(controller.daemon.logs.filter { $0.error != nil || ($0.status ?? 0) >= 400 }.count)")
                Spacer()
            }
            Divider()
            Table(controller.daemon.logs) {
                TableColumn("Time") { Text($0.timestamp.formatted(date: .omitted, time: .standard)) }
                    .width(80)
                TableColumn("Domain") { Text($0.domain).font(.system(.body, design: .monospaced)) }
                TableColumn("Mode") {
                    Text($0.intercepted ? "intercept" : "tunnel")
                        .foregroundStyle($0.intercepted ? .blue : .secondary)
                }
                .width(80)
                TableColumn("Status") { entry in
                    if let err = entry.error {
                        Text(err).foregroundStyle(.red).lineLimit(1)
                    } else if let s = entry.status {
                        Text("\(s)")
                    } else {
                        Text("—").foregroundStyle(.secondary)
                    }
                }
                .width(80)
                TableColumn("Latency") { Text("\($0.latencyMs) ms") }
                    .width(80)
            }
        }
        .padding(20)
    }

    private func stat(label: String, value: String) -> some View {
        VStack(alignment: .leading) {
            Text(label).font(.caption).foregroundStyle(.secondary)
            Text(value).font(.title2).bold()
        }
    }
}
