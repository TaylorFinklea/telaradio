// Telaradio.swift
//
// Idiomatic Swift wrapper around the Rust C ABI. Hides the unsafe pointer
// wrangling and translates the FFI's null-with-last-error convention into
// throwing functions.

import Foundation
import TelaradioFFI

private let aceStepTotalBytes: UInt64 = tr_ace_step_total_bytes()

/// Errors surfaced by the FFI layer.
enum TelaradioError: Error, CustomStringConvertible {
    case ffi(String)
    case nullPointer(String)

    var description: String {
        switch self {
        case .ffi(let msg): return "Telaradio FFI: \(msg)"
        case .nullPointer(let where_): return "Telaradio FFI returned null at \(where_)"
        }
    }
}

/// Read the current FFI thread-local error string, or a placeholder if none.
private func lastFFIError(_ where_: String) -> TelaradioError {
    guard let raw = tr_last_error() else {
        return .nullPointer(where_)
    }
    let message = String(cString: raw)
    return .ffi("\(where_): \(message)")
}

/// Modulation envelope, mirrored to the FFI's integer encoding.
enum Envelope: UInt32 {
    case square = 0
    case sine = 1
    case triangle = 2
}

/// Owned audio buffer. Frees its underlying Rust allocation on `deinit`.
final class WavBuffer {
    fileprivate let pointer: OpaquePointer

    fileprivate init(takingOwnership pointer: OpaquePointer) {
        self.pointer = pointer
    }

    deinit {
        // Cast OpaquePointer back into an UnsafeMutablePointer<TrWavBuffer>
        // for the FFI call. tr_wavbuffer_free handles null safely.
        tr_wavbuffer_free(pointer)
    }

    var sampleRate: UInt32 {
        return tr_wavbuffer_sample_rate(pointer)
    }

    var channels: UInt8 {
        return tr_wavbuffer_channels(pointer)
    }

    /// Number of samples (interleaved across channels).
    var sampleCount: Int {
        return Int(tr_wavbuffer_len(pointer))
    }

    /// Borrow the contiguous samples. Pointer is valid until this buffer is freed.
    func withSamples<R>(_ body: (UnsafeBufferPointer<Float>) -> R) -> R {
        guard let raw = tr_wavbuffer_samples(pointer) else {
            return body(UnsafeBufferPointer(start: nil, count: 0))
        }
        let buf = UnsafeBufferPointer(start: raw, count: sampleCount)
        return body(buf)
    }
}

/// Parsed recipe handle. Owns the underlying Rust allocation.
final class Recipe {
    fileprivate let pointer: OpaquePointer

    fileprivate init(takingOwnership pointer: OpaquePointer) {
        self.pointer = pointer
    }

    deinit {
        tr_recipe_free(pointer)
    }
}

/// Static FFI surface. Functions throw on failure rather than returning null.
enum Telaradio {
    /// Parse a recipe JSON document.
    static func parseRecipe(_ json: String) throws -> Recipe {
        // C signature: TrRecipe *tr_recipe_parse(const char *).
        // Swift imports pointers to forward-declared structs as OpaquePointer.
        let result: OpaquePointer? = json.withCString { tr_recipe_parse($0) }
        guard let ptr = result else {
            throw lastFFIError("parseRecipe")
        }
        return Recipe(takingOwnership: ptr)
    }

    /// Run the mock 440 Hz sine subprocess.
    static func generateMock(
        scriptPath: String,
        prompt: String,
        seed: UInt64,
        durationSeconds: UInt32
    ) throws -> WavBuffer {
        let result: OpaquePointer? = scriptPath.withCString { sp in
            prompt.withCString { p in
                tr_generate_mock(sp, p, seed, durationSeconds)
            }
        }
        guard let ptr = result else {
            throw lastFFIError("generateMock")
        }
        return WavBuffer(takingOwnership: ptr)
    }

    /// Apply amplitude modulation. Returns a new buffer; `input` is unchanged.
    static func applyAM(
        to input: WavBuffer,
        rateHz: Double,
        depth: Double,
        envelope: Envelope
    ) throws -> WavBuffer {
        guard let raw = tr_apply_am(
            input.pointer,
            rateHz,
            depth,
            envelope.rawValue
        ) else {
            throw lastFFIError("applyAM")
        }
        return WavBuffer(takingOwnership: raw)
    }

    /// Download ACE-Step model artifacts into `installDir`, resuming any
    /// partial download. `progress` fires on the main actor with a clamped
    /// fraction in [0.0, 1.0]. Returns the resolved install directory URL.
    static func ensureModelDownload(
        installDir: URL,
        progress: @escaping (Double) -> Void
    ) async throws -> URL {
        try await withCheckedThrowingContinuation { continuation in
            Task.detached(priority: .userInitiated) {
                let ctx = ProgressContext(
                    onProgress: progress,
                    totalBytes: aceStepTotalBytes
                )
                // Retained here; released after the FFI call returns.
                let ctxPtr = Unmanaged.passRetained(ctx).toOpaque()

                let token = tr_cancel_token_new()
                defer { tr_cancel_token_free(token) }

                let result = installDir.path(percentEncoded: false).withCString { installCStr in
                    tr_ensure_model_download(
                        installCStr,
                        { rawCtx, bytesWritten in
                            guard let rawCtx else { return }
                            let ctx = Unmanaged<ProgressContext>.fromOpaque(rawCtx)
                                .takeUnretainedValue()
                            let fraction = min(
                                1.0,
                                Double(bytesWritten) / Double(ctx.totalBytes)
                            )
                            // Download fires on a background thread; marshal to main actor
                            // before touching @Published properties on any ObservableObject.
                            Task { @MainActor in ctx.onProgress(fraction) }
                        },
                        ctxPtr,
                        token
                    )
                }

                // Balance the passRetained above.
                Unmanaged<ProgressContext>.fromOpaque(ctxPtr).release()

                if let cstr = result {
                    defer { tr_string_free(cstr) }
                    let path = String(cString: cstr)
                    continuation.resume(returning: URL(fileURLWithPath: path))
                } else {
                    continuation.resume(throwing: lastFFIError("ensureModelDownload"))
                }
            }
        }
    }

    /// Copy model weights from an existing local directory into `installDir`,
    /// validating sha256 checksums. Returns the resolved install directory URL.
    static func ensureModelUseExisting(
        installDir: URL,
        sourceDir: URL
    ) async throws -> URL {
        try await Task.detached(priority: .userInitiated) {
            let result = installDir.path(percentEncoded: false).withCString { installCStr in
                sourceDir.path(percentEncoded: false).withCString { sourceCStr in
                    tr_ensure_model_use_existing(installCStr, sourceCStr)
                }
            }
            guard let cstr = result else {
                throw lastFFIError("ensureModelUseExisting")
            }
            defer { tr_string_free(cstr) }
            let path = String(cString: cstr)
            return URL(fileURLWithPath: path)
        }.value
    }

    /// Run ACE-Step generation from a resolved model directory. Spawns the
    /// Python subprocess, generates audio, and drops the subprocess. Callers
    /// should surface a clear error when `.venv` is absent — the FFI reports
    /// the spawn failure via `tr_last_error`.
    static func generateAceStep(
        modelDir: URL,
        prompt: String,
        seed: UInt64,
        durationSeconds: UInt32
    ) async throws -> WavBuffer {
        try await Task.detached(priority: .userInitiated) {
            let result: OpaquePointer? = modelDir.path(percentEncoded: false).withCString { mdirCStr in
                prompt.withCString { promptCStr in
                    tr_generate_ace_step(mdirCStr, promptCStr, seed, durationSeconds)
                }
            }
            guard let ptr = result else {
                throw lastFFIError("generateAceStep")
            }
            return WavBuffer(takingOwnership: ptr)
        }.value
    }
}

// MARK: - Private helpers

/// Carries the Swift progress closure and the expected total bytes across
/// the C callback boundary. Held alive via Unmanaged retain for the
/// duration of tr_ensure_model_download.
private final class ProgressContext {
    let onProgress: (Double) -> Void
    let totalBytes: UInt64

    init(onProgress: @escaping (Double) -> Void, totalBytes: UInt64) {
        self.onProgress = onProgress
        self.totalBytes = totalBytes
    }
}
