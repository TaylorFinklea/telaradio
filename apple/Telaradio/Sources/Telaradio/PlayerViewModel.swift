// PlayerViewModel.swift
//
// Drives the SwiftUI view. Orchestrates the generate → modulate → play
// pipeline against a hardcoded example recipe. Branches on the configured
// backend: mock (440 Hz sine) or ACE-Step (real model generation).

import AVFoundation
import Foundation
import SwiftUI

@MainActor
final class PlayerViewModel: ObservableObject {
    enum Status {
        case ready
        case generating
        case modulating
        case playing
        case paused
        case error(String)

        var label: String {
            switch self {
            case .ready: return "Ready"
            case .generating: return "Generating audio…"
            case .modulating: return "Applying modulation…"
            case .playing: return "Playing"
            case .paused: return "Paused"
            case .error(let msg): return "Error: \(msg)"
            }
        }
    }

    @Published private(set) var status: Status = .ready

    private let engine = AVAudioEngine()
    private let player = AVAudioPlayerNode()
    private var pcmBuffer: AVAudioPCMBuffer?
    private let settings: ModelSettings

    init(settings: ModelSettings) {
        self.settings = settings
        engine.attach(player)
    }

    /// Load the hardcoded example recipe, generate (mock or ACE-Step), modulate,
    /// schedule on the audio engine, and start playback.
    func playExample() async {
        do {
            // Resolve paths relative to the workspace root. The executable
            // lives at apple/Telaradio/.build/.../debug/Telaradio. Walk up
            // from the binary location to the workspace root.
            let workspaceRoot = Self.findWorkspaceRoot()
            let recipePath = workspaceRoot
                .appendingPathComponent("recipes/example-foggy-lofi.json")
            let scriptPath = workspaceRoot
                .appendingPathComponent("model-adapter/python/telaradio_subprocess.py")

            let json = try String(contentsOf: recipePath, encoding: .utf8)
            _ = try Telaradio.parseRecipe(json) // validate it parses; we don't use the parsed value yet

            status = .generating

            let raw: WavBuffer
            switch settings.backend {
            case .mock:
                // Run the heavy work off the main actor.
                raw = try await Task.detached {
                    try Telaradio.generateMock(
                        scriptPath: scriptPath.path,
                        prompt: "Foggy lofi for deep work",
                        seed: 1_893_421,
                        durationSeconds: 5 // short clip for the MVL demo
                    )
                }.value

            case .aceStep:
                let modelDir = settings.modelDir!
                raw = try await Telaradio.generateAceStep(
                    modelDir: modelDir,
                    prompt: "Foggy lofi for deep work",
                    seed: 1_893_421,
                    durationSeconds: 30
                )
            }

            status = .modulating
            let modulated = try await Task.detached {
                try Telaradio.applyAM(to: raw, rateHz: 16.0, depth: 0.5, envelope: .square)
            }.value

            // AVAudioEngine wants its own buffer type; copy the FFI samples in.
            let buffer = Self.makePCMBuffer(from: modulated)
            pcmBuffer = buffer

            try startPlayback(buffer)
        } catch {
            status = .error(String(describing: error))
        }
    }

    func pause() {
        player.pause()
        if engine.isRunning {
            engine.pause()
        }
        status = .paused
    }

    func resume() {
        do {
            try engine.start()
            player.play()
            status = .playing
        } catch {
            status = .error("resume: \(error)")
        }
    }

    func stop() {
        player.stop()
        engine.stop()
        pcmBuffer = nil
        status = .ready
    }

    // MARK: - Internals

    private func startPlayback(_ buffer: AVAudioPCMBuffer) throws {
        // Wire the player to the main mixer with the buffer's format.
        engine.disconnectNodeOutput(player)
        engine.connect(player, to: engine.mainMixerNode, format: buffer.format)

        try engine.start()

        player.scheduleBuffer(buffer, at: nil, options: .interrupts) { [weak self] in
            Task { @MainActor [weak self] in
                self?.status = .ready
            }
        }
        player.play()
        status = .playing
    }

    private static func makePCMBuffer(from buffer: WavBuffer) -> AVAudioPCMBuffer {
        let sampleRate = Double(buffer.sampleRate)
        let channels = AVAudioChannelCount(buffer.channels)
        let format = AVAudioFormat(
            commonFormat: .pcmFormatFloat32,
            sampleRate: sampleRate,
            channels: channels,
            interleaved: false
        )!

        // Frame count = total samples / channels.
        let frameCount = AVAudioFrameCount(buffer.sampleCount / Int(channels))
        let pcm = AVAudioPCMBuffer(pcmFormat: format, frameCapacity: frameCount)!
        pcm.frameLength = frameCount

        // FFI buffer is interleaved; AVAudioPCMBuffer with `interleaved: false`
        // wants each channel in its own array. Demux.
        buffer.withSamples { samples in
            let channelPointers = pcm.floatChannelData!
            for ch in 0..<Int(channels) {
                let dst = channelPointers[ch]
                for frame in 0..<Int(frameCount) {
                    dst[frame] = samples[frame * Int(channels) + ch]
                }
            }
        }
        return pcm
    }

    /// Walk up from the executable to find the workspace root (the dir
    /// containing `Cargo.toml`). For `swift run` invocations the binary
    /// is at `.build/.../debug/Telaradio` so we walk up enough levels.
    private static func findWorkspaceRoot() -> URL {
        // Bundle.main.bundleURL points at the executable's parent dir.
        var dir = Bundle.main.bundleURL
        let fm = FileManager.default
        // Walk up until we find Cargo.toml or hit "/".
        for _ in 0..<10 {
            let candidate = dir.appendingPathComponent("Cargo.toml")
            if fm.fileExists(atPath: candidate.path) {
                return dir
            }
            let parent = dir.deletingLastPathComponent()
            if parent.path == dir.path {
                break
            }
            dir = parent
        }
        // Fallback: return Bundle.main.bundleURL parent and let downstream
        // file ops report a clear error.
        return Bundle.main.bundleURL.deletingLastPathComponent()
    }
}
