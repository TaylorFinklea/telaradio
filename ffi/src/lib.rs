//! Telaradio FFI: C-ABI shim around `telaradio-core`, `telaradio-dsp`, and
//! `telaradio-model-adapter`. Designed to be consumed from Swift (via a
//! cbindgen-generated header + `module.modulemap`) but works with any
//! C-ABI caller.
//!
//! ## Ownership
//!
//! Functions returning `*mut T` transfer ownership to the caller. The
//! caller MUST eventually pass the pointer to the matching `*_free`
//! function. Functions returning `*const T` return a non-owning borrow
//! valid until the owning object is freed.
//!
//! ## Errors
//!
//! Failures return a null pointer (or 0 / a sentinel) and set a
//! thread-local error string accessible via [`tr_last_error`]. Successful
//! calls clear the error.

#![allow(clippy::missing_safety_doc)]

use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char, c_void};
use std::path::Path;
use std::ptr;

use telaradio_core::Recipe;
use telaradio_core::audio::WavBuffer;
use telaradio_core::generator::Generator;
use telaradio_dsp::{Envelope, apply_am};
use telaradio_model_adapter::{
    ACE_STEP_TOTAL_BYTES, AceStepGenerator, CancellationToken, InstallMode, SubprocessGenerator,
    ace_step_artifacts, ensure_model,
};

/// Opaque cancel token exposed to C callers. Wraps a [`CancellationToken`]
/// which is `Arc<AtomicBool>`-backed and cheap to clone.
pub struct TrCancelToken(CancellationToken);

/// A `*mut c_void` context pointer is not `Send` by default because Rust
/// cannot verify the pointed-to data is safe to share across threads.
/// Here the pointer is opaque to Rust — we never dereference it, we only
/// pass it back to the caller's C callback on whatever thread the download
/// happens to run on. The caller is responsible for ensuring the pointed-to
/// data is thread-safe (e.g. a Swift actor-isolated object captured with
/// `Unmanaged.passRetained`). We document this contract on the public FFI
/// functions that accept it.
#[derive(Copy, Clone)]
struct CtxPtr(*mut c_void);

// SAFETY: see the WHY comment on `CtxPtr` above — opaque, never dereferenced.
unsafe impl Send for CtxPtr {}

/// Call the optional C progress callback. Keeping the extraction of the raw
/// pointer in a dedicated function prevents the closure that captures
/// `ctx_wrapped: CtxPtr` from having a `*mut c_void` as a direct capture
/// (which would make the closure non-`Send`).
#[allow(clippy::missing_safety_doc)]
unsafe fn call_progress_cb(cb: unsafe extern "C" fn(*mut c_void, u64), ctx: CtxPtr, bytes: u64) {
    let CtxPtr(raw) = ctx;
    // SAFETY: caller guarantees `raw` is valid for the duration of the call.
    unsafe { cb(raw, bytes) };
}

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_error(message: impl Into<String>) {
    let raw = message.into();
    let cleaned = CString::new(raw)
        .unwrap_or_else(|_| CString::new("error message contained NUL byte").unwrap());
    LAST_ERROR.with(|cell| *cell.borrow_mut() = Some(cleaned));
}

fn clear_error() {
    LAST_ERROR.with(|cell| *cell.borrow_mut() = None);
}

/// Returns the last error message set by an FFI call on this thread, or
/// null if no error is pending. The pointer is owned by the FFI; do not
/// free. Successful calls clear this; subsequent reads will see null
/// until the next failure.
#[unsafe(no_mangle)]
pub extern "C" fn tr_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| cell.borrow().as_ref().map_or(ptr::null(), |s| s.as_ptr()))
}

/// Parse a recipe from a NUL-terminated UTF-8 JSON string.
/// Returns an owned pointer or null on failure.
///
/// # Safety
/// `json` must be a valid NUL-terminated C string or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_recipe_parse(json: *const c_char) -> *mut Recipe {
    if json.is_null() {
        set_error("tr_recipe_parse: json pointer is null");
        return ptr::null_mut();
    }
    let cstr = unsafe { CStr::from_ptr(json) };
    let s = match cstr.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("tr_recipe_parse: invalid UTF-8: {e}"));
            return ptr::null_mut();
        }
    };
    match Recipe::parse(s) {
        Ok(recipe) => {
            clear_error();
            Box::into_raw(Box::new(recipe))
        }
        Err(e) => {
            set_error(format!("tr_recipe_parse: {e}"));
            ptr::null_mut()
        }
    }
}

/// Free a `Recipe` previously returned by [`tr_recipe_parse`]. Null is a no-op.
///
/// # Safety
/// `recipe` must be a pointer returned by `tr_recipe_parse` and not yet freed,
/// or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_recipe_free(recipe: *mut Recipe) {
    if !recipe.is_null() {
        drop(unsafe { Box::from_raw(recipe) });
    }
}

/// Construct a new `WavBuffer` from a samples array. Samples are copied;
/// the caller may free their input array after this call returns.
///
/// # Safety
/// `samples` must be a valid pointer to `len` consecutive f32 values, or
/// null with `len == 0`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_wavbuffer_new(
    samples: *const f32,
    len: usize,
    sample_rate: u32,
    channels: u8,
) -> *mut WavBuffer {
    if samples.is_null() && len > 0 {
        set_error("tr_wavbuffer_new: samples is null but len > 0");
        return ptr::null_mut();
    }
    let copied: Vec<f32> = if len == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(samples, len) }.to_vec()
    };
    clear_error();
    Box::into_raw(Box::new(WavBuffer {
        sample_rate,
        channels,
        samples: copied,
    }))
}

/// Run the mock 440 Hz sine subprocess to produce a `WavBuffer`. Spawns
/// `python3 <script_path>`, sends one NDJSON request, reads the response,
/// reads the WAV the subprocess writes, and returns the buffer.
///
/// # Safety
/// `script_path` and `prompt` must be valid NUL-terminated UTF-8 C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_generate_mock(
    script_path: *const c_char,
    prompt: *const c_char,
    seed: u64,
    duration_seconds: u32,
) -> *mut WavBuffer {
    if script_path.is_null() || prompt.is_null() {
        set_error("tr_generate_mock: null argument");
        return ptr::null_mut();
    }
    let script = match unsafe { CStr::from_ptr(script_path) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!(
                "tr_generate_mock: invalid UTF-8 in script_path: {e}"
            ));
            return ptr::null_mut();
        }
    };
    let prompt_str = match unsafe { CStr::from_ptr(prompt) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!("tr_generate_mock: invalid UTF-8 in prompt: {e}"));
            return ptr::null_mut();
        }
    };
    let generator = match SubprocessGenerator::spawn(Path::new(script)) {
        Ok(g) => g,
        Err(e) => {
            set_error(format!("tr_generate_mock: spawn: {e}"));
            return ptr::null_mut();
        }
    };
    match generator.generate(prompt_str, seed, duration_seconds) {
        Ok(buf) => {
            clear_error();
            Box::into_raw(Box::new(buf))
        }
        Err(e) => {
            set_error(format!("tr_generate_mock: generate: {e}"));
            ptr::null_mut()
        }
    }
}

/// Apply amplitude modulation. Returns a new owned buffer; the input
/// buffer is unchanged. `envelope_kind`: 0 = Square, 1 = Sine, 2 = Triangle.
///
/// # Safety
/// `input` must be a valid `WavBuffer` pointer (returned by another
/// `tr_*` function and not yet freed) or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_apply_am(
    input: *const WavBuffer,
    rate_hz: f64,
    depth: f64,
    envelope_kind: u32,
) -> *mut WavBuffer {
    if input.is_null() {
        set_error("tr_apply_am: input is null");
        return ptr::null_mut();
    }
    let envelope = match envelope_kind {
        0 => Envelope::Square,
        1 => Envelope::Sine,
        2 => Envelope::Triangle,
        n => {
            set_error(format!(
                "tr_apply_am: unknown envelope_kind {n} (expected 0/1/2)"
            ));
            return ptr::null_mut();
        }
    };
    let input_buf = unsafe { &*input };
    let modulated = apply_am(input_buf, rate_hz, depth, envelope);
    clear_error();
    Box::into_raw(Box::new(modulated))
}

/// Free a `WavBuffer` previously returned by an FFI function. Null is a no-op.
///
/// # Safety
/// `buffer` must be a pointer returned by `tr_wavbuffer_new`,
/// `tr_generate_mock`, or `tr_apply_am`, and not yet freed; or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_wavbuffer_free(buffer: *mut WavBuffer) {
    if !buffer.is_null() {
        drop(unsafe { Box::from_raw(buffer) });
    }
}

/// Borrow the contiguous samples of a `WavBuffer`. Pointer is valid until
/// the buffer is freed.
///
/// # Safety
/// `buffer` must be a valid `WavBuffer` pointer or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_wavbuffer_samples(buffer: *const WavBuffer) -> *const f32 {
    if buffer.is_null() {
        return ptr::null();
    }
    unsafe { (*buffer).samples.as_ptr() }
}

/// Sample count (interleaved across channels) of a `WavBuffer`. Returns 0 if null.
///
/// # Safety
/// `buffer` must be a valid `WavBuffer` pointer or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_wavbuffer_len(buffer: *const WavBuffer) -> usize {
    if buffer.is_null() {
        return 0;
    }
    unsafe { (*buffer).samples.len() }
}

/// Sample rate of a `WavBuffer` in Hz. Returns 0 if null.
///
/// # Safety
/// `buffer` must be a valid `WavBuffer` pointer or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_wavbuffer_sample_rate(buffer: *const WavBuffer) -> u32 {
    if buffer.is_null() {
        return 0;
    }
    unsafe { (*buffer).sample_rate }
}

/// Channel count of a `WavBuffer`. Returns 0 if null.
///
/// # Safety
/// `buffer` must be a valid `WavBuffer` pointer or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_wavbuffer_channels(buffer: *const WavBuffer) -> u8 {
    if buffer.is_null() {
        return 0;
    }
    unsafe { (*buffer).channels }
}

// ── Cancel token ─────────────────────────────────────────────────────────────

/// Allocate a new cancellation token. Never returns null.
///
/// The returned pointer is owned by the caller; free with
/// [`tr_cancel_token_free`].
#[unsafe(no_mangle)]
pub extern "C" fn tr_cancel_token_new() -> *mut TrCancelToken {
    Box::into_raw(Box::new(TrCancelToken(CancellationToken::new())))
}

/// Signal cancellation on `token`. Safe to call on null (no-op).
///
/// # Safety
/// `token` must be a valid pointer returned by [`tr_cancel_token_new`]
/// and not yet freed, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_cancel_token_cancel(token: *mut TrCancelToken) {
    if !token.is_null() {
        unsafe { (*token).0.cancel() };
    }
}

/// Free a cancel token. Safe to call on null (no-op).
///
/// # Safety
/// `token` must be a pointer returned by [`tr_cancel_token_new`] and not
/// yet freed, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_cancel_token_free(token: *mut TrCancelToken) {
    if !token.is_null() {
        drop(unsafe { Box::from_raw(token) });
    }
}

// ── Model install ─────────────────────────────────────────────────────────────

/// Total bytes the ACE-Step model footprint occupies once
/// [`tr_ensure_model_download`] finishes. UI code can use this as the
/// denominator for a download progress bar.
#[unsafe(no_mangle)]
pub extern "C" fn tr_ace_step_total_bytes() -> u64 {
    ACE_STEP_TOTAL_BYTES
}

/// Free a C string previously returned by [`tr_ensure_model_download`] or
/// [`tr_ensure_model_use_existing`]. Safe to call on null (no-op).
///
/// **The returned strings from `tr_ensure_model_*` MUST be freed via this
/// function.** Do not pass them to C's `free()` — Rust's allocator must
/// reclaim them.
///
/// # Safety
/// `ptr` must be a `*mut c_char` returned by one of the `tr_ensure_model_*`
/// functions and not yet freed, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}

/// Download the ACE-Step model artifacts into `install_dir`, resuming any
/// partial download. Returns the install directory as a NUL-terminated C
/// string on success; the caller must free it with [`tr_string_free`].
/// Returns null on failure; call [`tr_last_error`] for the reason.
///
/// **Thread-safety contract for `progress_cb`:** the callback fires on
/// whatever OS thread the blocking download runs on — *not* the calling
/// thread. Swift callers must marshal back to the main actor inside the
/// callback (e.g. `Task { @MainActor in … }`). The `ctx` pointer is passed
/// through opaquely; Rust never dereferences it, but it must remain valid
/// for the duration of the call.
///
/// # Safety
/// - `install_dir` must be a valid NUL-terminated UTF-8 C string.
/// - `ctx` is caller-managed; see thread-safety note above.
/// - `cancel` must be a valid [`TrCancelToken`] pointer or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_ensure_model_download(
    install_dir: *const c_char,
    progress_cb: Option<unsafe extern "C" fn(*mut c_void, u64)>,
    ctx: *mut c_void,
    cancel: *const TrCancelToken,
) -> *mut c_char {
    if install_dir.is_null() {
        set_error("tr_ensure_model_download: install_dir is null");
        return ptr::null_mut();
    }
    let dir_str = match unsafe { CStr::from_ptr(install_dir) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!(
                "tr_ensure_model_download: invalid UTF-8 in install_dir: {e}"
            ));
            return ptr::null_mut();
        }
    };

    let ctx_wrapped = CtxPtr(ctx);
    let progress: Option<telaradio_model_adapter::hf_download::ProgressCallback> =
        progress_cb.map(|cb| {
            // Capture `ctx_wrapped: CtxPtr` (which is `Send`) so the closure
            // satisfies `Box<dyn FnMut(u64) + Send>`. We call a helper that
            // unwraps it in a way that stays within the `CtxPtr` type boundary.
            let f: telaradio_model_adapter::hf_download::ProgressCallback =
                Box::new(move |bytes: u64| {
                    // SAFETY: ctx_wrapped is opaque; we only hand it back to
                    // the C callback. The caller owns the pointed-to memory.
                    unsafe { call_progress_cb(cb, ctx_wrapped, bytes) };
                });
            f
        });

    let cancel_token = if cancel.is_null() {
        None
    } else {
        // Clone the token so the caller's token outlives this call.
        Some(unsafe { (*cancel).0.clone() })
    };

    let artifacts = ace_step_artifacts();
    match ensure_model(
        Path::new(dir_str),
        artifacts,
        InstallMode::Download(progress, cancel_token),
    ) {
        Ok(path) => {
            let Some(path_str) = path.to_str() else {
                set_error("tr_ensure_model_download: install path is not valid UTF-8");
                return ptr::null_mut();
            };
            match CString::new(path_str) {
                Ok(cs) => {
                    clear_error();
                    cs.into_raw()
                }
                Err(e) => {
                    set_error(format!(
                        "tr_ensure_model_download: path contains NUL byte: {e}"
                    ));
                    ptr::null_mut()
                }
            }
        }
        Err(e) => {
            set_error(format!("tr_ensure_model_download: {e}"));
            ptr::null_mut()
        }
    }
}

/// Copy model weights from an existing local directory into `install_dir`,
/// validating sha256 checksums. Returns the install directory as a
/// NUL-terminated C string on success; the caller must free it with
/// [`tr_string_free`]. Returns null on failure; call [`tr_last_error`] for
/// the reason.
///
/// # Safety
/// `install_dir` and `source_dir` must be valid NUL-terminated UTF-8 C
/// strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_ensure_model_use_existing(
    install_dir: *const c_char,
    source_dir: *const c_char,
) -> *mut c_char {
    if install_dir.is_null() || source_dir.is_null() {
        set_error("tr_ensure_model_use_existing: null argument");
        return ptr::null_mut();
    }
    let dir_str = match unsafe { CStr::from_ptr(install_dir) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!(
                "tr_ensure_model_use_existing: invalid UTF-8 in install_dir: {e}"
            ));
            return ptr::null_mut();
        }
    };
    let src_str = match unsafe { CStr::from_ptr(source_dir) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!(
                "tr_ensure_model_use_existing: invalid UTF-8 in source_dir: {e}"
            ));
            return ptr::null_mut();
        }
    };

    let artifacts = ace_step_artifacts();
    match ensure_model(
        Path::new(dir_str),
        artifacts,
        InstallMode::UseExisting(Path::new(src_str).to_owned()),
    ) {
        Ok(path) => {
            let Some(path_str) = path.to_str() else {
                set_error("tr_ensure_model_use_existing: install path is not valid UTF-8");
                return ptr::null_mut();
            };
            match CString::new(path_str) {
                Ok(cs) => {
                    clear_error();
                    cs.into_raw()
                }
                Err(e) => {
                    set_error(format!(
                        "tr_ensure_model_use_existing: path contains NUL byte: {e}"
                    ));
                    ptr::null_mut()
                }
            }
        }
        Err(e) => {
            set_error(format!("tr_ensure_model_use_existing: {e}"));
            ptr::null_mut()
        }
    }
}

// ── ACE-Step generation ───────────────────────────────────────────────────────

/// Run ACE-Step generation from a resolved model directory. Spawns the
/// ACE-Step Python subprocess, generates audio, and drops the subprocess.
///
/// Yes, this incurs subprocess startup per call (~few seconds). Phase 1e
/// (background buffer queue) will add a persistent subprocess; this
/// spawn-use-drop pattern is the minimal correct path for Phase 1d2.
///
/// Returns an owned `WavBuffer`; the caller must free it with
/// [`tr_wavbuffer_free`]. Returns null on failure; call [`tr_last_error`].
///
/// # Safety
/// `model_dir` and `prompt` must be valid NUL-terminated UTF-8 C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tr_generate_ace_step(
    model_dir: *const c_char,
    prompt: *const c_char,
    seed: u64,
    duration_seconds: u32,
) -> *mut WavBuffer {
    if model_dir.is_null() || prompt.is_null() {
        set_error("tr_generate_ace_step: null argument");
        return ptr::null_mut();
    }
    let dir_str = match unsafe { CStr::from_ptr(model_dir) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!(
                "tr_generate_ace_step: invalid UTF-8 in model_dir: {e}"
            ));
            return ptr::null_mut();
        }
    };
    let prompt_str = match unsafe { CStr::from_ptr(prompt) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_error(format!(
                "tr_generate_ace_step: invalid UTF-8 in prompt: {e}"
            ));
            return ptr::null_mut();
        }
    };
    let generator = match AceStepGenerator::spawn(Path::new(dir_str)) {
        Ok(g) => g,
        Err(e) => {
            set_error(format!("tr_generate_ace_step: spawn: {e}"));
            return ptr::null_mut();
        }
    };
    match generator.generate(prompt_str, seed, duration_seconds) {
        Ok(buf) => {
            clear_error();
            Box::into_raw(Box::new(buf))
        }
        Err(e) => {
            set_error(format!("tr_generate_ace_step: generate: {e}"));
            ptr::null_mut()
        }
    }
}
