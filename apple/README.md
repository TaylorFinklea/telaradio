# apple/

Native Swift / SwiftUI clients.

- macOS app — Phase 1 (the listening surface for solo development)
- iOS app — Phase 2 (parity with macOS, plus mobile-first UX polish)

Both apps talk to the Rust backend over HTTP/gRPC (decision deferred to
Phase 1 first commit). Audio playback uses AVFoundation so background
audio, lock-screen controls, and Now Playing integration come for free.

Phase 3 picks up the Apple Watch heart-rate adaptation in this directory.
