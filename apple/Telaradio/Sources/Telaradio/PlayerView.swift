// PlayerView.swift
//
// SwiftUI player view. Phase 1d2: adds first-launch model-setup sheet.

import SwiftUI

struct PlayerView: View {
    // Both objects are siblings owned by this view. PlayerViewModel holds a
    // strong reference to settings; no retain cycle because the view owns both.
    @StateObject private var modelSettings: ModelSettings
    @StateObject private var viewModel: PlayerViewModel

    init() {
        let settings = ModelSettings()
        _modelSettings = StateObject(wrappedValue: settings)
        _viewModel = StateObject(wrappedValue: PlayerViewModel(settings: settings))
    }

    var body: some View {
        VStack(spacing: 24) {
            Text("Telaradio")
                .font(.largeTitle)
                .fontWeight(.semibold)

            Text("Focus music — 16 Hz AM modulation")
                .foregroundStyle(.secondary)

            Text(viewModel.status.label)
                .font(.body.monospaced())
                .padding(.vertical, 4)

            HStack(spacing: 16) {
                Button {
                    Task { await viewModel.playExample() }
                } label: {
                    Label("Play", systemImage: "play.fill")
                }

                Button {
                    viewModel.pause()
                } label: {
                    Label("Pause", systemImage: "pause.fill")
                }

                Button {
                    viewModel.stop()
                } label: {
                    Label("Stop", systemImage: "stop.fill")
                }
            }
            .controlSize(.large)
        }
        .padding(32)
        .frame(minWidth: 420, minHeight: 220)
        .sheet(
            isPresented: Binding(
                get: { !modelSettings.isConfigured },
                // Sheet dismisses itself when the user makes a choice and
                // isConfigured becomes true; no explicit set path needed.
                set: { _ in }
            )
        ) {
            ModelSetupView(settings: modelSettings)
                // Prevent accidental swipe-to-dismiss; the user must pick a backend.
                .interactiveDismissDisabled()
        }
    }
}

#Preview {
    PlayerView()
}
