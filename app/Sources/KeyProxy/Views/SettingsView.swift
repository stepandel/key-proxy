import SwiftUI

struct SettingsView: View {
    var body: some View {
        TabView {
            RulesTab()
                .tabItem { Label("Rules", systemImage: "list.bullet") }
            GeneralTab()
                .tabItem { Label("General", systemImage: "gearshape") }
            StatsTab()
                .tabItem { Label("Stats", systemImage: "chart.bar") }
        }
        .frame(width: 640, height: 460)
    }
}
