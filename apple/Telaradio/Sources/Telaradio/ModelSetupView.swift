// ModelSetupView.swift
//
// First-launch sheet. Lets the user choose a model source once; the choice
// persists in UserDefaults so subsequent launches skip this sheet entirely.

import AppKit
import SwiftUI

struct ModelSetupView: View {
    @ObservedObject var settings: ModelSettings

    @State private var phase: Phase = .idle
    @State private var downloadProgress: Double = 0
    @State private var errorMessage: String?

    private enum Phase { case idle, downloading, installing, picking }

    var body: some View {
        VStack(spacing: 16) {
            Text("Set up Telaradio")
                .font(.title2)
                .bold()

            Text("Telaradio needs a music-generation model. Choose how to get one:")
                .multilineTextAlignment(.center)
                .padding(.horizontal)

            switch phase {
            case .idle:
                Button("Download (~5 GB) from Hugging Face") { downloadTapped() }
                Button("Use existing folder…") { pickFolderTapped() }
                Button("Use mock for now") { useMockTapped() }

            case .downloading:
                ProgressView(value: downloadProgress)
                Text("\(Int(downloadProgress * 100))%")

            case .installing:
                ProgressView()
                Text("Validating folder…")

            case .picking:
                ProgressView()
            }

            if let msg = errorMessage {
                Text(msg)
                    .foregroundColor(.red)
                    .font(.caption)
                    .multilineTextAlignment(.center)
            }
        }
        .padding(24)
        .frame(width: 420)
    }

    // MARK: - Actions

    private func downloadTapped() {
        phase = .downloading
        downloadProgress = 0
        errorMessage = nil
        Task {
            do {
                let installDir = ModelSettings.defaultInstallDir
                try createParentDirectory(for: installDir)
                let dir = try await Telaradio.ensureModelDownload(
                    installDir: installDir,
                    progress: { fraction in
                        // ensureModelDownload already dispatches to @MainActor before
                        // calling this closure, so no extra dispatch needed.
                        downloadProgress = fraction
                    }
                )
                settings.modelDir = dir
                settings.backend = .aceStep
            } catch {
                errorMessage = "\(error)"
                phase = .idle
            }
        }
    }

    private func pickFolderTapped() {
        // NSOpenPanel must run on the main thread; it's synchronous here so the
        // .picking spinner only flashes briefly before the panel appears.
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = false
        guard panel.runModal() == .OK, let src = panel.url else { return }

        phase = .installing
        errorMessage = nil
        Task {
            do {
                let installDir = ModelSettings.defaultInstallDir
                try createParentDirectory(for: installDir)
                let dir = try await Telaradio.ensureModelUseExisting(
                    installDir: installDir,
                    sourceDir: src
                )
                settings.modelDir = dir
                settings.backend = .aceStep
            } catch {
                errorMessage = "\(error)"
                phase = .idle
            }
        }
    }

    private func useMockTapped() {
        // Synchronous — just flip the backend and let the Binding dismiss the sheet.
        settings.backend = .mock
    }

    // MARK: - Helpers

    private func createParentDirectory(for url: URL) throws {
        let parent = url.deletingLastPathComponent()
        try FileManager.default.createDirectory(
            at: parent,
            withIntermediateDirectories: true
        )
    }
}
