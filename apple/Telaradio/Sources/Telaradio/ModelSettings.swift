// ModelSettings.swift
//
// UserDefaults-backed observable that records which generation backend the
// user has chosen and where the model weights live. The sheet in
// ModelSetupView is shown whenever isConfigured is false.

import Foundation

enum GenerationBackend: String {
    case mock
    case aceStep
}

@MainActor
final class ModelSettings: ObservableObject {
    // Default backend is aceStep so new installs go to the setup sheet rather
    // than silently falling back to mock audio.
    @Published var backend: GenerationBackend {
        didSet { UserDefaults.standard.set(backend.rawValue, forKey: "backend") }
    }

    @Published var modelDir: URL? {
        didSet {
            if let dir = modelDir {
                UserDefaults.standard.set(dir.path(percentEncoded: false), forKey: "modelDir")
            } else {
                UserDefaults.standard.removeObject(forKey: "modelDir")
            }
        }
    }

    init() {
        let defaults = UserDefaults.standard
        self.backend = GenerationBackend(rawValue: defaults.string(forKey: "backend") ?? "") ?? .aceStep
        self.modelDir = defaults.string(forKey: "modelDir").map { URL(fileURLWithPath: $0) }
    }

    /// True once the user has made a choice: either mock mode or a resolved model dir.
    var isConfigured: Bool { backend == .mock || modelDir != nil }

    /// Standard install location used by both Download and Use Existing paths.
    static let defaultInstallDir: URL = FileManager.default
        .urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        .appendingPathComponent("Telaradio/models/ace-step-v1-3.5b")
}
