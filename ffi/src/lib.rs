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
use std::ffi::{CStr, CString, c_char};
use std::path::Path;
use std::ptr;

use telaradio_core::Recipe;
use telaradio_core::audio::WavBuffer;
use telaradio_core::generator::Generator;
use telaradio_dsp::{Envelope, apply_am};
use telaradio_model_adapter::SubprocessGenerator;

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
