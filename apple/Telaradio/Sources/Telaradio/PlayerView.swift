// PlayerView.swift
//
// SwiftUI player view. Phase 1d MVL: status text + Play / Pause / Stop.

import SwiftUI

struct PlayerView: View {
    @StateObject private var viewModel = PlayerViewModel()

    var body: some View {
        VStack(spacing: 24) {
            Text("Telaradio")
                .font(.largeTitle)
                .fontWeight(.semibold)

            Text("Focus music — modulated 440 Hz mock (Phase 1d MVL)")
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
    }
}

#Preview {
    PlayerView()
}
