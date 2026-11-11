import AppKit
import SwiftUI
import Combine

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate, NSMenuDelegate {
    private var statusItem: NSStatusItem!
    private let controller = DictationController()
    private var settingsWindow: NSWindow?
    private var onboardingWindow: NSWindow?
    private var bag = Set<AnyCancellable>()

    func applicationDidFinishLaunching(_ notification: Notification) {
        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        let menu = NSMenu()
        menu.delegate = self
        statusItem.menu = menu
        updateIcon()

        AppState.shared.$isRecording.combineLatest(AppState.shared.$model)
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in self?.updateIcon() }
            .store(in: &bag)

        controller.start()

        if !AppSettings.shared.hasCompletedOnboarding
            || !AudioRecorder.permissionGranted || !TextInserter.accessibilityGranted {
            showOnboarding()
        }
    }

    private func updateIcon() {
        let state = AppState.shared
        let symbol: String
        switch (state.isRecording, state.model) {
        case (true, _): symbol = "waveform.circle.fill"
        case (_, .ready): symbol = "waveform"
        case (_, .failed): symbol = "exclamationmark.triangle"
        default: symbol = "arrow.down.circle"
        }
        let image = NSImage(systemSymbolName: symbol, accessibilityDescription: "Aazad Voice")
        image?.isTemplate = true
        statusItem.button?.image = image
    }

    // Rebuild the menu each time it opens so it always reflects live state.
    func menuNeedsUpdate(_ menu: NSMenu) {
        menu.removeAllItems()
        let state = AppState.shared
        let settings = AppSettings.shared

        let status = NSMenuItem(title: state.model == .ready
                                ? "Ready — hold \(settings.triggerKey.shortLabel) to dictate"
                                : controller.modelStatusMessage, action: nil, keyEquivalent: "")
        status.isEnabled = false
        menu.addItem(status)

        if !state.micGranted || !state.accessibilityGranted {
            menu.addItem(item("⚠︎ Finish setup…", #selector(showOnboarding)))
        }
        if case .failed = state.model {
            menu.addItem(item("Retry model download", #selector(retryModel)))
        }
        menu.addItem(.separator())
        menu.addItem(item(state.isRecording ? "Stop dictation" : "Start dictation (hands-free)", #selector(toggleDictation)))

        let recent = HistoryStore.shared.items.prefix(5)
        if !recent.isEmpty {
            menu.addItem(.separator())
            let header = NSMenuItem(title: "Recent — click to copy", action: nil, keyEquivalent: "")
            header.isEnabled = false
            menu.addItem(header)
            for h in recent {
                let title = h.text.count > 48 ? String(h.text.prefix(48)) + "…" : h.text
                let mi = item(title, #selector(copyHistory(_:)))
                mi.representedObject = h.text
                menu.addItem(mi)
            }
        }

        menu.addItem(.separator())
        menu.addItem(item("Settings…", #selector(showSettings), key: ","))
        menu.addItem(item("Quit Aazad Voice", #selector(quit), key: "q"))
    }

    private func item(_ title: String, _ action: Selector, key: String = "") -> NSMenuItem {
        let mi = NSMenuItem(title: title, action: action, keyEquivalent: key)
        mi.target = self
        return mi
    }

    @objc private func toggleDictation() { controller.toggleFromMenu() }
    @objc private func retryModel() { controller.loadModel() }
    @objc private func quit() { NSApp.terminate(nil) }

    @objc private func copyHistory(_ sender: NSMenuItem) {
        guard let text = sender.representedObject as? String else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    @objc func showSettings() {
        AppState.shared.refreshPermissions()
        if settingsWindow == nil {
            settingsWindow = makeWindow(title: "Aazad Voice Settings", view: SettingsView())
        }
        present(settingsWindow!)
    }

    @objc func showOnboarding() {
        if onboardingWindow == nil {
            onboardingWindow = makeWindow(title: "Welcome", view: OnboardingView { [weak self] in
                self?.onboardingWindow?.close()
            })
            onboardingWindow?.titlebarAppearsTransparent = true
        }
        present(onboardingWindow!)
    }

    private func makeWindow<V: View>(title: String, view: V) -> NSWindow {
        let w = NSWindow(contentViewController: NSHostingController(rootView: view))
        w.title = title
        w.styleMask = [.titled, .closable]
        w.isReleasedWhenClosed = false
        w.center()
        return w
    }

    private func present(_ window: NSWindow) {
        NSApp.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
    }
}
