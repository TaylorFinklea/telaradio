// swift-tools-version:5.9
//
// Telaradio macOS app (Phase 1d MVL).
//
// Two targets:
//   - TelaradioFFI: a system library wrapping the cbindgen-generated
//     C header that exposes the Rust workspace's C ABI. The header
//     itself is auto-regenerated whenever telaradio-ffi is built; do
//     not edit it by hand.
//   - Telaradio: the SwiftUI macOS executable that consumes the FFI.
//
// Build flow (run from workspace root):
//
//   make ffi      → cargo build -p telaradio-ffi (writes header + .a)
//   make swift    → swift build (links against ../../target/.../libtelaradio_ffi.a)
//   make app-run  → both, then launches the executable
//
// The Rust static lib is found via the unsafe linker flag
// `-L../../target/debug` (relative to apple/Telaradio/). For release
// builds, swap to `-L../../target/release`.
import PackageDescription

let package = Package(
    name: "Telaradio",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "Telaradio", targets: ["Telaradio"]),
    ],
    targets: [
        .systemLibrary(
            name: "TelaradioFFI",
            path: "Sources/TelaradioFFI"
        ),
        .executableTarget(
            name: "Telaradio",
            dependencies: ["TelaradioFFI"],
            path: "Sources/Telaradio",
            linkerSettings: [
                .unsafeFlags([
                    "-L../../target/debug",
                ]),
            ]
        ),
    ]
)
