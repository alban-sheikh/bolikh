import AppKit

/// Watches a single modifier key system-wide and turns presses into start/stop/cancel intents.
@MainActor
final class HotkeyMonitor {
    var onStart: (() -> Void)?
    var onStop: (() -> Void)?
    var onCancel: (() -> Void)?

    var triggerKey: TriggerKey = .rightOption
    var mode: RecordMode = .smart

    /// Whether a recording session is active (owned by the controller).
    var isActive: () -> Bool = { false }

    private var monitors: [Any] = []
    private var keyIsDown = false
    private var pressedAt: Date?
    private var startedThisPress = false
    private var otherKeyDuringPress = false

    /// Holding longer than this means push-to-talk; shorter means a tap.
    private let tapThreshold: TimeInterval = 0.3

    func start() {
        stop()
        let flagsHandler: (NSEvent) -> Void = { [weak self] e in self?.handleFlags(e) }
        let keyHandler: (NSEvent) -> Void = { [weak self] e in self?.handleKeyDown(e) }
        if let m = NSEvent.addGlobalMonitorForEvents(matching: .flagsChanged, handler: flagsHandler) { monitors.append(m) }
        if let m = NSEvent.addGlobalMonitorForEvents(matching: .keyDown, handler: keyHandler) { monitors.append(m) }
        if let m = NSEvent.addLocalMonitorForEvents(matching: .flagsChanged, handler: { flagsHandler($0); return $0 }) { monitors.append(m) }
        if let m = NSEvent.addLocalMonitorForEvents(matching: .keyDown, handler: { keyHandler($0); return $0 }) { monitors.append(m) }
    }

    func stop() {
        monitors.forEach { NSEvent.removeMonitor($0) }
        monitors.removeAll()
    }

    private func handleFlags(_ event: NSEvent) {
        guard event.keyCode == triggerKey.keyCode else { return }
        let down = event.modifierFlags.contains(triggerKey.flag)
        if down && !keyIsDown {
            keyIsDown = true
            pressedAt = Date()
            otherKeyDuringPress = false
            keyPressed()
        } else if !down && keyIsDown {
            keyIsDown = false
            keyReleased(heldFor: Date().timeIntervalSince(pressedAt ?? Date()))
        }
    }

    private func handleKeyDown(_ event: NSEvent) {
        // Escape cancels any active dictation.
        if event.keyCode == 53, isActive() { onCancel?(); return }
        // Another key while the trigger is held = user is typing a shortcut (e.g. ⌥+E). Back off.
        if keyIsDown {
            otherKeyDuringPress = true
            if startedThisPress, isActive() { onCancel?() }
        }
    }

    private func keyPressed() {
        startedThisPress = false
        switch mode {
        case .hold, .smart:
            if isActive() {
                // In smart mode a tap while hands-free stops it.
                onStop?()
            } else {
                startedThisPress = true
                onStart?()
            }
        case .toggle:
            if isActive() { onStop?() } else { startedThisPress = true; onStart?() }
        }
    }

    private func keyReleased(heldFor: TimeInterval) {
        guard startedThisPress, !otherKeyDuringPress else { return }
        switch mode {
        case .hold:
            if isActive() { onStop?() }
        case .smart:
            // Long hold → push-to-talk, stop on release. Quick tap → stay recording (hands-free).
            if heldFor >= tapThreshold, isActive() { onStop?() }
        case .toggle:
            break
        }
    }
}
