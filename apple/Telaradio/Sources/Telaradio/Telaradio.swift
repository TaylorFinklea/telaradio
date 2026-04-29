// Telaradio.swift
//
// Idiomatic Swift wrapper around the Rust C ABI. Hides the unsafe pointer
// wrangling and translates the FFI's null-with-last-error convention into
// throwing functions.

import Foundation
import TelaradioFFI

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
}
