import SwiftUI

struct OnboardingView: View {
    @ObservedObject var state = AppState.shared
    @ObservedObject var settings = AppSettings.shared
    var onDone: () -> Void
    private let timer = Timer.publish(every: 1, on: .main, in: .common).autoconnect()

    var body: some View {
        VStack(spacing: 0) {
            header
            VStack(spacing: 14) {
                StepCard(number: 1, title: "Microphone", detail: "So Aazad Voice can hear you. Audio stays in memory and is never saved or sent.",
                         done: state.micGranted, action: "Allow") {
                    Task { _ = await AudioRecorder.requestPermission(); state.refreshPermissions() }
                }
                StepCard(number: 2, title: "Accessibility", detail: "Lets Aazad Voice type into the app you're using. Enable it in System Settings, then come back.",
                         done: state.accessibilityGranted, action: "Open Settings") {
                    TextInserter.promptForAccessibility()
                    TextInserter.openAccessibilitySettings()
                }
                modelCard
            }
            .padding(24)
            Spacer(minLength: 0)
            footer
        }
        .frame(width: 520, height: 600)
        .onReceive(timer) { _ in state.refreshPermissions() }
    }

    private var header: some View {
        VStack(spacing: 8) {
            Image(nsImage: NSApp.applicationIconImage).resizable().frame(width: 72, height: 72)
            Text("Welcome to Aazad Voice").font(.system(size: 24, weight: .bold))
            Text("Speak anywhere on your Mac. 100% on-device.").foregroundStyle(.secondary)
        }
        .padding(.top, 30)
    }

    private var modelCard: some View {
        HStack(alignment: .top, spacing: 14) {
            StepBadge(number: 3, done: state.model == .ready)
            VStack(alignment: .leading, spacing: 6) {
                Text("Speech model").font(.headline)
                Text("Downloads NVIDIA Parakeet v3 (~0.5 GB) once. After that, works fully offline.")
                    .font(.callout).foregroundStyle(.secondary)
                ModelStatusBadge(state: state.model)
                if case .failed(let e) = state.model {
                    Text(e).font(.caption).foregroundStyle(.red).lineLimit(3)
                }
            }
            Spacer()
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color.primary.opacity(0.04)))
    }

    private var footer: some View {
        VStack(spacing: 10) {
            Divider()
            HStack {
                Text("Then hold **\(settings.triggerKey.shortLabel)** and talk. Release to type.")
                    .foregroundStyle(.secondary)
                Spacer()
                Button(state.allReady ? "Start dictating" : "Finish later") {
                    settings.hasCompletedOnboarding = true
                    onDone()
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)
            }
            .padding(.horizontal, 24).padding(.bottom, 16)
        }
    }
}

private struct StepCard: View {
    let number: Int
    let title: String
    let detail: String
    let done: Bool
    let action: String
    let perform: () -> Void

    var body: some View {
        HStack(alignment: .top, spacing: 14) {
            StepBadge(number: number, done: done)
            VStack(alignment: .leading, spacing: 4) {
                Text(title).font(.headline)
                Text(detail).font(.callout).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
            }
            Spacer()
            if !done { Button(action, action: perform).controlSize(.regular) }
        }
        .padding(14)
        .background(RoundedRectangle(cornerRadius: 12).fill(Color.primary.opacity(0.04)))
    }
}

private struct StepBadge: View {
    let number: Int
    let done: Bool
    var body: some View {
        ZStack {
            Circle().fill(done ? Color.green : Color.accentColor.opacity(0.15)).frame(width: 28, height: 28)
            if done { Image(systemName: "checkmark").font(.system(size: 13, weight: .bold)).foregroundStyle(.white) }
            else { Text("\(number)").font(.system(size: 13, weight: .bold)).foregroundStyle(Color.accentColor) }
        }
    }
}
