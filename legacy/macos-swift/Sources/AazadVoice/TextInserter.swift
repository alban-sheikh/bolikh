import AppKit
import ApplicationServices

/// Inserts text into whatever text field currently has keyboard focus, in any app.
/// Uses the clipboard + ⌘V (most compatible approach), then restores the user's clipboard.
enum TextInserter {
    static var accessibilityGranted: Bool { AXIsProcessTrusted() }

    static func promptForAccessibility() {
        let key = kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String
        _ = AXIsProcessTrustedWithOptions([key: true] as CFDictionary)
    }

    static func openAccessibilitySettings() {
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        }
    }

    @MainActor
    static func insert(_ text: String, restoreClipboard: Bool) {
        guard !text.isEmpty else { return }
        let pasteboard = NSPasteboard.general
        let saved = restoreClipboard ? snapshot(pasteboard) : nil

        pasteboard.clearContents()
        pasteboard.setString(text, forType: .string)
        // Tell clipboard managers this is transient (don't record it in history).
        pasteboard.setString("", forType: NSPasteboard.PasteboardType("org.nspasteboard.TransientType"))
        let changeCount = pasteboard.changeCount

        pressCommandV()

        if let saved {
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.6) {
                // Only restore if nobody else touched the clipboard in the meantime.
                guard pasteboard.changeCount == changeCount else { return }
                pasteboard.clearContents()
                if !saved.isEmpty { pasteboard.writeObjects(saved) }
            }
        }
    }

    private static func pressCommandV() {
        let source = CGEventSource(stateID: .combinedSessionState)
        // Ignore the user's physically-held modifiers (e.g. the trigger key).
        source?.setLocalEventsFilterDuringSuppressionState(
            [.permitLocalMouseEvents, .permitSystemDefinedEvents], state: .eventSuppressionStateSuppressionInterval)
        let vKey: CGKeyCode = 9
        let down = CGEvent(keyboardEventSource: source, virtualKey: vKey, keyDown: true)
        let up = CGEvent(keyboardEventSource: source, virtualKey: vKey, keyDown: false)
        down?.flags = .maskCommand
        up?.flags = .maskCommand
        down?.post(tap: .cghidEventTap)
        up?.post(tap: .cghidEventTap)
    }

    private static func snapshot(_ pb: NSPasteboard) -> [NSPasteboardItem] {
        (pb.pasteboardItems ?? []).map { item in
            let copy = NSPasteboardItem()
            for type in item.types {
                if let data = item.data(forType: type) { copy.setData(data, forType: type) }
            }
            return copy
        }
    }
}
