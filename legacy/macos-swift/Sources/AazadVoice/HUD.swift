import AppKit
import SwiftUI

@MainActor
final class HUDModel: ObservableObject {
    enum Phase: Equatable { case hidden, listening, handsFree, transcribing, message(String) }
    @Published var phase: Phase = .hidden
    @Published var levels: [CGFloat] = Array(repeating: 0, count: 28)
    @Published var startedAt = Date()

    func push(level: Float) {
        levels.removeFirst()
        levels.append(CGFloat(level))
    }

    func resetLevels() { levels = Array(repeating: 0, count: levels.count) }
}

/// Floating, click-through pill shown near the bottom of the active screen.
@MainActor
final class HUDController {
    let model = HUDModel()
    private var panel: NSPanel?
    private var hideWork: DispatchWorkItem?

    func show(_ phase: HUDModel.Phase) {
        hideWork?.cancel()
        if model.phase == .hidden || phase == .listening { model.startedAt = Date() }
        if phase == .listening { model.resetLevels() }
        withAnimation(.spring(response: 0.35, dampingFraction: 0.8)) { model.phase = phase }
        ensurePanel()
        position()
        panel?.orderFrontRegardless()
        if case .message = phase { hide(after: 2.2) }
    }

    func hide(after delay: TimeInterval = 0) {
        hideWork?.cancel()
        let work = DispatchWorkItem { [weak self] in
            guard let self else { return }
            withAnimation(.easeOut(duration: 0.2)) { self.model.phase = .hidden }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) {
                if self.model.phase == .hidden { self.panel?.orderOut(nil) }
            }
        }
        hideWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + delay, execute: work)
    }

    private func ensurePanel() {
        guard panel == nil else { return }
        let p = NSPanel(contentRect: NSRect(x: 0, y: 0, width: 360, height: 90),
                        styleMask: [.nonactivatingPanel, .borderless],
                        backing: .buffered, defer: false)
        p.isOpaque = false
        p.backgroundColor = .clear
        p.hasShadow = false
        p.level = .statusBar
        p.ignoresMouseEvents = true
        p.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .stationary, .ignoresCycle]
        p.contentView = NSHostingView(rootView: HUDView(model: model))
        panel = p
    }

    private func position() {
        guard let panel else { return }
        let mouse = NSEvent.mouseLocation
        let screen = NSScreen.screens.first { NSMouseInRect(mouse, $0.frame, false) } ?? NSScreen.main
        guard let frame = screen?.visibleFrame else { return }
        let size = panel.frame.size
        panel.setFrameOrigin(NSPoint(x: frame.midX - size.width / 2, y: frame.minY + 28))
    }
}

struct HUDView: View {
    @ObservedObject var model: HUDModel

    var body: some View {
        ZStack {
            if model.phase != .hidden {
                pill.transition(.scale(scale: 0.85).combined(with: .opacity))
            }
        }
        .frame(width: 360, height: 90)
    }

    private var pill: some View {
        HStack(spacing: 12) {
            indicator
            content
        }
        .padding(.horizontal, 16)
        .frame(height: 46)
        .background(
            Capsule().fill(.ultraThinMaterial)
                .overlay(Capsule().fill(Color.black.opacity(0.55)))
                .overlay(Capsule().strokeBorder(Color.white.opacity(0.12), lineWidth: 1))
        )
        .shadow(color: .black.opacity(0.35), radius: 18, y: 8)
        .environment(\.colorScheme, .dark)
    }

    @ViewBuilder private var indicator: some View {
        switch model.phase {
        case .listening, .handsFree:
            PulsingDot()
        case .transcribing:
            ProgressView().controlSize(.small).tint(.white)
        default:
            Image(systemName: "waveform").foregroundStyle(.white.opacity(0.8))
        }
    }

    @ViewBuilder private var content: some View {
        switch model.phase {
        case .listening, .handsFree:
            Waveform(levels: model.levels)
                .frame(width: 150, height: 22)
            TimelineView(.periodic(from: .now, by: 1)) { ctx in
                Text(elapsed(ctx.date))
                    .font(.system(size: 12, weight: .medium, design: .monospaced))
                    .foregroundStyle(.white.opacity(0.7))
            }
            if model.phase == .handsFree {
                Text("hands-free")
                    .font(.system(size: 10, weight: .semibold))
                    .padding(.horizontal, 6).padding(.vertical, 2)
                    .background(Capsule().fill(Color.white.opacity(0.15)))
                    .foregroundStyle(.white.opacity(0.85))
            }
        case .transcribing:
            Text("Transcribing…").font(.system(size: 13, weight: .medium)).foregroundStyle(.white.opacity(0.9))
        case .message(let msg):
            Text(msg).font(.system(size: 13, weight: .medium)).foregroundStyle(.white.opacity(0.9)).lineLimit(1)
        case .hidden:
            EmptyView()
        }
    }

    private func elapsed(_ now: Date) -> String {
        let s = max(0, Int(now.timeIntervalSince(model.startedAt)))
        return String(format: "%d:%02d", s / 60, s % 60)
    }
}

private struct Waveform: View {
    let levels: [CGFloat]
    var body: some View {
        HStack(alignment: .center, spacing: 3) {
            ForEach(Array(levels.enumerated()), id: \.offset) { _, level in
                Capsule()
                    .fill(LinearGradient(colors: [Color(red: 0.55, green: 0.75, blue: 1), Color(red: 0.75, green: 0.55, blue: 1)],
                                         startPoint: .bottom, endPoint: .top))
                    .frame(width: 2.5, height: max(3, level * 22))
            }
        }
        .animation(.easeOut(duration: 0.08), value: levels)
    }
}

private struct PulsingDot: View {
    @State private var on = false
    var body: some View {
        Circle().fill(Color(red: 1, green: 0.27, blue: 0.3))
            .frame(width: 9, height: 9)
            .shadow(color: .red.opacity(0.8), radius: on ? 6 : 1)
            .scaleEffect(on ? 1.0 : 0.8)
            .onAppear { withAnimation(.easeInOut(duration: 0.7).repeatForever()) { on = true } }
    }
}
