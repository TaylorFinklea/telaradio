// TelaradioApp.swift
//
// SwiftUI macOS app entry point. Hosts a single PlayerView in a window.

import AppKit
import SwiftUI

@main
struct TelaradioApp: App {
    init() {
        // SwiftPM executables launch as command-line tools, so the window
        // never gains focus by default. Promote to a regular app and force
        // activation so `swift run` actually surfaces the window.
        NSApplication.shared.setActivationPolicy(.regular)
        NSApplication.shared.activate(ignoringOtherApps: true)
    }

    var body: some Scene {
        WindowGroup("Telaradio") {
            PlayerView()
        }
        .windowResizability(.contentSize)
    }
}
