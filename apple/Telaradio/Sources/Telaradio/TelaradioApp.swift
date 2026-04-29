// TelaradioApp.swift
//
// SwiftUI macOS app entry point. Hosts a single PlayerView in a window.

import SwiftUI

@main
struct TelaradioApp: App {
    var body: some Scene {
        WindowGroup("Telaradio") {
            PlayerView()
        }
        .windowResizability(.contentSize)
    }
}
