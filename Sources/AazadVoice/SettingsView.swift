import SwiftUI
import ServiceManagement

struct SettingsView: View {
    var body: some View {
        TabView {
            GeneralTab().tabItem { Label("General", systemImage: "gearshape") }
            DictionaryTab().tabItem { Label("Dictionary", systemImage: "character.book.closed") }
            HistoryTab().tabItem { Label("History", systemImage: "clock.arrow.circlepath") }
            AboutTab().tabItem { Label("About", systemImage: "info.circle") }
        }
        .frame(width: 560, height: 460)
    }
}

private struct GeneralTab: View {
    @ObservedObject var settings = AppSettings.shared
    @ObservedObject var state = AppState.shared
    @State private var launchAtLogin = SMAppService.mainApp.status == .enabled

    var body: some View {
        Form {
            Section("Shortcut") {
                Picker("Trigger key", selection: $settings.triggerKey) {
                    ForEach(TriggerKey.allCases) { Text($0.label).tag($0) }
                }
                Picker("Behaviour", selection: $settings.recordMode) {
                    ForEach(RecordMode.allCases) { Text($0.label).tag($0) }
                }
                if settings.triggerKey == .fn {
                    Text("Tip: set System Settings → Keyboard → “Press 🌐 key to” → Do Nothing.")
                        .font(.caption).foregroundStyle(.secondary)
                }
                Text("Press Esc at any time to cancel a dictation.")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Section("Output") {
                Toggle("Add a space after each dictation", isOn: $settings.addTrailingSpace)
                Toggle("Voice commands (“new line”, “new paragraph”)", isOn: $settings.voiceCommands)
                Toggle("Restore my clipboard after pasting", isOn: $settings.restoreClipboard)
            }
            Section("App") {
                Toggle("Sound effects", isOn: $settings.playSounds)
                Toggle("Keep dictation history on this Mac", isOn: $settings.keepHistory)
                Toggle("Launch at login", isOn: $launchAtLogin)
                    .onChange(of: launchAtLogin) { _, on in
                        do { on ? try SMAppService.mainApp.register() : try SMAppService.mainApp.unregister() }
                        catch { launchAtLogin = SMAppService.mainApp.status == .enabled }
                    }
            }
            Section("Status") {
                StatusRow(title: "Microphone", ok: state.micGranted)
                StatusRow(title: "Accessibility (paste into apps)", ok: state.accessibilityGranted)
                HStack {
                    Text("Speech model — Parakeet TDT 0.6B v3")
                    Spacer()
                    ModelStatusBadge(state: state.model)
                }
            }
        }
        .formStyle(.grouped)
    }
}

struct StatusRow: View {
    let title: String
    let ok: Bool
    var body: some View {
        HStack {
            Text(title)
            Spacer()
            Label(ok ? "Granted" : "Not granted", systemImage: ok ? "checkmark.circle.fill" : "exclamationmark.circle.fill")
                .foregroundStyle(ok ? .green : .orange)
                .labelStyle(.titleAndIcon)
        }
    }
}

struct ModelStatusBadge: View {
    let state: Transcriber.State
    var body: some View {
        switch state {
        case .notLoaded: Text("Not loaded").foregroundStyle(.secondary)
        case .downloading(let p): HStack { ProgressView(value: p).frame(width: 90); Text("\(Int(p * 100))%").monospacedDigit() }
        case .loading: HStack { ProgressView().controlSize(.small); Text("Preparing…") }
        case .ready: Label("Ready", systemImage: "checkmark.circle.fill").foregroundStyle(.green)
        case .failed: Label("Failed", systemImage: "xmark.octagon.fill").foregroundStyle(.red)
        }
    }
}

private struct DictionaryTab: View {
    @ObservedObject var settings = AppSettings.shared
    @State private var from = ""
    @State private var to = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Fix words the model gets wrong, or create spoken shortcuts.")
                .foregroundStyle(.secondary)
            HStack {
                TextField("When I say…", text: $from)
                Image(systemName: "arrow.right").foregroundStyle(.secondary)
                TextField("Write…", text: $to)
                Button("Add") {
                    settings.replacements.append(Replacement(from: from, to: to))
                    from = ""; to = ""
                }
                .disabled(from.trimmingCharacters(in: .whitespaces).isEmpty)
                .keyboardShortcut(.defaultAction)
            }
            List {
                ForEach(settings.replacements) { r in
                    HStack {
                        Text(r.from)
                        Image(systemName: "arrow.right").foregroundStyle(.secondary)
                        Text(r.to).fontWeight(.medium)
                        Spacer()
                        Button(role: .destructive) {
                            settings.replacements.removeAll { $0.id == r.id }
                        } label: { Image(systemName: "trash") }.buttonStyle(.borderless)
                    }
                }
            }
            .overlay {
                if settings.replacements.isEmpty {
                    Text("No entries yet. Example: “aazad” → “Aazad”").foregroundStyle(.tertiary)
                }
            }
        }
        .padding(20)
    }
}

private struct HistoryTab: View {
    @ObservedObject var history = HistoryStore.shared

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("\(history.items.count) dictations · \(history.totalWords) words")
                    .foregroundStyle(.secondary)
                Spacer()
                Button("Clear history", role: .destructive) { history.clear() }
                    .disabled(history.items.isEmpty)
            }
            List(history.items) { item in
                VStack(alignment: .leading, spacing: 4) {
                    Text(item.text).textSelection(.enabled).lineLimit(4)
                    HStack {
                        Text(item.date, style: .relative) + Text(" ago")
                        if let app = item.app { Text("· \(app)") }
                        Spacer()
                        Button {
                            NSPasteboard.general.clearContents()
                            NSPasteboard.general.setString(item.text, forType: .string)
                        } label: { Image(systemName: "doc.on.doc") }.buttonStyle(.borderless).help("Copy")
                    }
                    .font(.caption).foregroundStyle(.secondary)
                }
                .padding(.vertical, 2)
            }
            .overlay {
                if history.items.isEmpty { Text("Nothing dictated yet.").foregroundStyle(.tertiary) }
            }
        }
        .padding(20)
    }
}

private struct AboutTab: View {
    var body: some View {
        VStack(spacing: 14) {
            Image(nsImage: NSApp.applicationIconImage).resizable().frame(width: 84, height: 84)
            Text("Aazad Voice").font(.title.bold())
            Text("Version \(Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "dev")")
                .foregroundStyle(.secondary)
            Text("Private, offline dictation for every app on your Mac.\nYour voice never leaves this computer.")
                .multilineTextAlignment(.center)
            Divider().padding(.horizontal, 60)
            VStack(spacing: 4) {
                Text("Speech model: NVIDIA Parakeet TDT 0.6B v3 — CC-BY-4.0")
                Text("CoreML runtime: FluidAudio by Fluid Inference — Apache-2.0")
            }
            .font(.caption).foregroundStyle(.secondary)
        }
        .padding(30)
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
