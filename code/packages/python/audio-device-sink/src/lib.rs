//! Python bridge for the Rust audio device sink stack.
//!
//! Hardened against VER15 FFI targets:
//! - Panic safety: `std::panic::catch_unwind` wraps the entire body of every
//!   `extern "C"` entry point (`py_play_samples`, `PyInit_audio_device_sink`)
//!   and sets a Python `RuntimeError` on panic before returning `NULL`.
//! - CWE-476 (NULL Pointer Dereference): explicit null checks on every CPython
//!   argument, tuple, iterator, class lookup, and constructed return object.
//! - CWE-401 (Reference Count Safety): builtin exception/type classes are
//!   cached once in `OnceLock` (zero per-call ref leaks), `exc_base` is
//!   decref'd after `PyErr_NewException`, and `report_to_py` decrefs any
//!   unconsumed tuple items if `PyTuple_SetItem` fails.
//! - CWE-190 / CWE-400 / CWE-20: rejects `str`/`bytes`/`bytearray` directly in
//!   `parse_samples`, uses `PyObject_Size` for O(1) length checks while
//!   propagating `OverflowError` / `MemoryError` / `KeyboardInterrupt`
//!   untouched, checks `PcmFormat::max_samples`, and bounds error messages to
//!   `MAX_ERROR_MESSAGE_BYTES`.

use std::ffi::{c_char, c_int, c_longlong, c_ulonglong, CStr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;
use std::sync::OnceLock;

use audio_device_coreaudio::CoreAudioSink;
use audio_device_sink::{
    AudioSink, PcmFormat, PcmPlaybackBuffer, PlaybackReport, MAX_CHANNELS, MAX_SAMPLE_RATE_HZ,
    MIN_SAMPLE_RATE_HZ, SUPPORTED_BIT_DEPTH, SUPPORTED_CHANNEL_COUNT,
};
use python_bridge::*;

const MAX_ERROR_MESSAGE_BYTES: usize = 256;

#[allow(non_snake_case)]
extern "C" {
    fn PyLong_AsLongLong(obj: PyObjectPtr) -> c_longlong;
    fn PyLong_FromUnsignedLongLong(v: c_ulonglong) -> PyObjectPtr;
    fn PyErr_Occurred() -> PyObjectPtr;
    fn PyErr_ExceptionMatches(exc: PyObjectPtr) -> c_int;
    fn PyObject_IsInstance(inst: PyObjectPtr, cls: PyObjectPtr) -> c_int;
    fn PyObject_Size(o: PyObjectPtr) -> isize;
    fn PyTuple_Size(p: PyObjectPtr) -> isize;
}

static AUDIO_DEVICE_ERROR_CLASS: OnceLock<usize> = OnceLock::new();
static BOOL_CLASS: OnceLock<usize> = OnceLock::new();
static STR_CLASS: OnceLock<usize> = OnceLock::new();
static BYTES_CLASS: OnceLock<usize> = OnceLock::new();
static BYTEARRAY_CLASS: OnceLock<usize> = OnceLock::new();
static TYPE_ERROR_CLASS: OnceLock<usize> = OnceLock::new();
static VALUE_ERROR_CLASS: OnceLock<usize> = OnceLock::new();
static RUNTIME_ERROR_CLASS: OnceLock<usize> = OnceLock::new();

fn bound_error_message(message: &str) -> &str {
    if message.len() <= MAX_ERROR_MESSAGE_BYTES {
        return message;
    }
    let mut end = MAX_ERROR_MESSAGE_BYTES;
    while end > 0 && !message.is_char_boundary(end) {
        end -= 1;
    }
    &message[..end]
}

unsafe fn lookup_builtin(name: &CStr) -> PyObjectPtr {
    let builtins = PyImport_ImportModule(c"builtins".as_ptr());
    if builtins.is_null() {
        PyErr_Clear();
        return ptr::null_mut();
    }
    let attr = PyObject_GetAttrString(builtins, name.as_ptr());
    Py_DecRef(builtins);
    if attr.is_null() {
        PyErr_Clear();
        return ptr::null_mut();
    }
    attr
}

unsafe fn cached_builtin(slot: &OnceLock<usize>, name: &CStr) -> PyObjectPtr {
    if let Some(&ptr_val) = slot.get() {
        return ptr_val as PyObjectPtr;
    }
    let cls = lookup_builtin(name);
    if cls.is_null() {
        return ptr::null_mut();
    }
    if slot.set(cls as usize).is_err() {
        Py_DecRef(cls);
        return slot.get().copied().unwrap_or(0) as PyObjectPtr;
    }
    cls
}

unsafe fn type_error_cls() -> PyObjectPtr {
    cached_builtin(&TYPE_ERROR_CLASS, c"TypeError")
}

unsafe fn value_error_cls() -> PyObjectPtr {
    cached_builtin(&VALUE_ERROR_CLASS, c"ValueError")
}

unsafe fn runtime_error_cls() -> PyObjectPtr {
    cached_builtin(&RUNTIME_ERROR_CLASS, c"RuntimeError")
}

unsafe fn audio_device_error_cls() -> PyObjectPtr {
    AUDIO_DEVICE_ERROR_CLASS
        .get()
        .map(|value| *value as PyObjectPtr)
        .unwrap_or_else(|| runtime_error_cls())
}

unsafe fn set_bounded_error(exc_type: PyObjectPtr, message: &str) {
    if exc_type.is_null() {
        return;
    }
    set_error(exc_type, bound_error_message(message));
}

unsafe fn set_audio_device_error(error: impl Into<String>) -> PyObjectPtr {
    let msg = error.into();
    set_bounded_error(audio_device_error_cls(), &msg);
    ptr::null_mut()
}

unsafe fn py_is_instance(obj: PyObjectPtr, cls: PyObjectPtr) -> Option<bool> {
    if obj.is_null() || cls.is_null() {
        return Some(false);
    }
    let result = PyObject_IsInstance(obj, cls);
    if result < 0 {
        // Propagate any active Python exception rather than masking it.
        None
    } else {
        Some(result == 1)
    }
}

unsafe fn py_is_bool(obj: PyObjectPtr) -> bool {
    if obj.is_null() {
        return false;
    }
    let cls = cached_builtin(&BOOL_CLASS, c"bool");
    if cls.is_null() {
        return false;
    }
    let result = PyObject_IsInstance(obj, cls);
    if result < 0 {
        PyErr_Clear();
        false
    } else {
        result == 1
    }
}

unsafe fn py_i64(obj: PyObjectPtr) -> Option<i64> {
    if obj.is_null() || py_is_bool(obj) {
        return None;
    }
    PyErr_Clear();
    let value = PyLong_AsLongLong(obj);
    if !PyErr_Occurred().is_null() {
        PyErr_Clear();
        return None;
    }
    Some(value)
}

unsafe fn parse_arg_i64(args: PyObjectPtr, index: isize) -> Option<i64> {
    if args.is_null() {
        return None;
    }
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        PyErr_Clear();
        return None;
    }
    py_i64(arg)
}

enum SampleParseError {
    InvalidType,
    InvalidValue,
    PythonExceptionSet,
}

unsafe fn parse_samples(
    obj: PyObjectPtr,
    max_samples: usize,
) -> Result<Vec<i16>, SampleParseError> {
    if obj.is_null() {
        return Err(SampleParseError::InvalidType);
    }

    for cls in [
        cached_builtin(&STR_CLASS, c"str"),
        cached_builtin(&BYTES_CLASS, c"bytes"),
        cached_builtin(&BYTEARRAY_CLASS, c"bytearray"),
    ] {
        match py_is_instance(obj, cls) {
            Some(true) => return Err(SampleParseError::InvalidType),
            Some(false) => {}
            None => return Err(SampleParseError::PythonExceptionSet),
        }
    }

    // O(1) pre-flight length check for sized containers.
    // Only TypeError ("object has no len()") is cleared to allow unsized iterators;
    // OverflowError (e.g., range(1 << 100), oversized __len__) and all other
    // exceptions propagate immediately without clearing or iterating.
    let known_size = PyObject_Size(obj);
    if known_size < 0 {
        if !PyErr_Occurred().is_null() {
            let te = type_error_cls();
            if !te.is_null() && PyErr_ExceptionMatches(te) != 0 {
                PyErr_Clear();
            } else {
                return Err(SampleParseError::PythonExceptionSet);
            }
        }
    } else if (known_size as usize) > max_samples {
        return Err(SampleParseError::InvalidValue);
    }

    let iter = PyObject_GetIter(obj);
    if iter.is_null() {
        if !PyErr_Occurred().is_null() {
            return Err(SampleParseError::PythonExceptionSet);
        }
        return Err(SampleParseError::InvalidType);
    }

    let mut samples = Vec::new();
    loop {
        let item = PyIter_Next(iter);
        if item.is_null() {
            if !PyErr_Occurred().is_null() {
                Py_DecRef(iter);
                return Err(SampleParseError::PythonExceptionSet);
            }
            break;
        }

        let Some(value) = py_i64(item) else {
            Py_DecRef(item);
            Py_DecRef(iter);
            return Err(SampleParseError::InvalidValue);
        };
        Py_DecRef(item);

        if value < i16::MIN as i64 || value > i16::MAX as i64 {
            Py_DecRef(iter);
            return Err(SampleParseError::InvalidValue);
        }
        if samples.len() >= max_samples {
            Py_DecRef(iter);
            return Err(SampleParseError::InvalidValue);
        }
        samples.push(value as i16);
    }

    Py_DecRef(iter);
    Ok(samples)
}

unsafe fn report_to_py(report: PlaybackReport) -> PyObjectPtr {
    let tuple = PyTuple_New(5);
    if tuple.is_null() {
        return ptr::null_mut();
    }
    let frames = PyLong_FromUnsignedLongLong(report.frames_played as c_ulonglong);
    let rate = PyLong_FromUnsignedLongLong(report.sample_rate_hz as c_ulonglong);
    let channels = PyLong_FromUnsignedLongLong(report.channel_count as c_ulonglong);
    let duration = f64_to_py(report.duration_seconds);
    let backend = str_to_py(report.backend_name);
    let items = [frames, rate, channels, duration, backend];
    if items.iter().any(|item| item.is_null()) {
        for item in items {
            if !item.is_null() {
                Py_DecRef(item);
            }
        }
        Py_DecRef(tuple);
        return ptr::null_mut();
    }
    for (index, item) in items.iter().copied().enumerate() {
        if PyTuple_SetItem(tuple, index as isize, item) != 0 {
            // PyTuple_SetItem steals `item` on index; unconsumed tail items must be decref'd.
            for remaining in items.iter().copied().skip(index + 1) {
                Py_DecRef(remaining);
            }
            Py_DecRef(tuple);
            return ptr::null_mut();
        }
    }
    tuple
}

unsafe fn py_play_samples_inner(args: PyObjectPtr) -> PyObjectPtr {
    if args.is_null() {
        set_bounded_error(
            type_error_cls(),
            "_play_samples() requires 3 arguments (samples, sample_rate_hz, channel_count)",
        );
        return ptr::null_mut();
    }
    let arg_count = PyTuple_Size(args);
    if arg_count != 3 {
        if arg_count < 0 {
            PyErr_Clear();
        }
        set_bounded_error(
            type_error_cls(),
            "_play_samples() requires 3 arguments (samples, sample_rate_hz, channel_count)",
        );
        return ptr::null_mut();
    }

    let sample_rate_hz = match parse_arg_i64(args, 1) {
        Some(value)
            if value >= i64::from(MIN_SAMPLE_RATE_HZ) && value <= i64::from(MAX_SAMPLE_RATE_HZ) =>
        {
            value as u32
        }
        _ => {
            set_bounded_error(
                type_error_cls(),
                "_play_samples() requires a positive integer sample_rate_hz within V1 bounds",
            );
            return ptr::null_mut();
        }
    };
    let channel_count = match parse_arg_i64(args, 2) {
        Some(value)
            if value == i64::from(SUPPORTED_CHANNEL_COUNT) && value <= i64::from(MAX_CHANNELS) =>
        {
            value as u16
        }
        _ => {
            set_bounded_error(
                type_error_cls(),
                "_play_samples() only supports mono channel_count=1",
            );
            return ptr::null_mut();
        }
    };

    let format = match PcmFormat::new(sample_rate_hz, channel_count, SUPPORTED_BIT_DEPTH) {
        Ok(fmt) => fmt,
        Err(error) => return set_audio_device_error(error.to_string()),
    };
    let max_samples = match format.max_samples() {
        Ok(cap) => cap,
        Err(error) => return set_audio_device_error(error.to_string()),
    };

    let sample_arg = PyTuple_GetItem(args, 0);
    if sample_arg.is_null() {
        PyErr_Clear();
        set_bounded_error(type_error_cls(), "_play_samples() requires samples");
        return ptr::null_mut();
    }
    let samples = match parse_samples(sample_arg, max_samples) {
        Ok(value) => value,
        Err(SampleParseError::PythonExceptionSet) => return ptr::null_mut(),
        Err(SampleParseError::InvalidType) => {
            set_bounded_error(
                type_error_cls(),
                "_play_samples() samples must be an iterable of signed 16-bit PCM integers",
            );
            return ptr::null_mut();
        }
        Err(SampleParseError::InvalidValue) => {
            set_bounded_error(
                value_error_cls(),
                "_play_samples() samples must be signed 16-bit PCM integers within the V1 size limit",
            );
            return ptr::null_mut();
        }
    };

    let buffer = match PcmPlaybackBuffer::new(samples, format) {
        Ok(buf) => buf,
        Err(error) => return set_audio_device_error(error.to_string()),
    };
    match CoreAudioSink::new().play_blocking(&buffer) {
        Ok(report) => report_to_py(report),
        Err(error) => set_audio_device_error(error.to_string()),
    }
}

unsafe extern "C" fn py_play_samples(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    match catch_unwind(AssertUnwindSafe(|| py_play_samples_inner(args))) {
        Ok(result) => result,
        Err(_) => {
            if PyErr_Occurred().is_null() {
                let rt = runtime_error_cls();
                if !rt.is_null() {
                    PyErr_SetString(rt, c"internal panic in audio_device_sink".as_ptr());
                }
            }
            ptr::null_mut()
        }
    }
}

/// CPython extension module entry point for `audio_device_sink`.
///
/// # Safety
///
/// Must be called only by the CPython interpreter during module import with the
/// GIL held; the full body is wrapped in `catch_unwind`.
#[no_mangle]
pub unsafe extern "C" fn PyInit_audio_device_sink() -> PyObjectPtr {
    let init_result = catch_unwind(AssertUnwindSafe(|| {
        // Warm cached exception classes once so error paths never allocate/leak refs.
        let _ = runtime_error_cls();
        let _ = type_error_cls();
        let _ = value_error_cls();

        let methods: &'static mut [PyMethodDef; 2] = Box::leak(Box::new([
            PyMethodDef {
                ml_name: c"_play_samples".as_ptr(),
                ml_meth: Some(py_play_samples),
                ml_flags: METH_VARARGS,
                ml_doc: c"Play mono signed 16-bit PCM samples through the default audio sink."
                    .as_ptr(),
            },
            method_def_sentinel(),
        ]));

        let def: &'static mut PyModuleDef = Box::leak(Box::new(PyModuleDef {
            m_base: PyModuleDef_Base {
                ob_base: [0u8; std::mem::size_of::<usize>() * 2],
                m_init: None,
                m_index: 0,
                m_copy: ptr::null_mut(),
            },
            m_name: c"audio_device_sink".as_ptr(),
            m_doc: c"Python bridge for Rust audio device sinks.".as_ptr(),
            m_size: -1,
            m_methods: methods.as_mut_ptr(),
            m_slots: ptr::null_mut(),
            m_traverse: ptr::null_mut(),
            m_clear: ptr::null_mut(),
            m_free: ptr::null_mut(),
        }));

        let module = PyModule_Create2(def as *mut PyModuleDef, PYTHON_API_VERSION);
        if module.is_null() {
            return ptr::null_mut();
        }

        let exc_base = lookup_builtin(c"Exception");
        if exc_base.is_null() {
            Py_DecRef(module);
            return ptr::null_mut();
        }

        let audio_device_error = PyErr_NewException(
            c"audio_device_sink.AudioDeviceError".as_ptr(),
            exc_base,
            ptr::null_mut(),
        );
        Py_DecRef(exc_base);
        if audio_device_error.is_null() {
            Py_DecRef(module);
            return ptr::null_mut();
        }

        if PyModule_AddObject(module, c"AudioDeviceError".as_ptr(), audio_device_error) != 0 {
            Py_DecRef(audio_device_error);
            Py_DecRef(module);
            return ptr::null_mut();
        }
        Py_IncRef(audio_device_error);
        let _ = AUDIO_DEVICE_ERROR_CLASS.set(audio_device_error as usize);
        let _ = c_char::default();
        module
    }));

    match init_result {
        Ok(module) => module,
        Err(_) => {
            if PyErr_Occurred().is_null() {
                let rt = runtime_error_cls();
                if !rt.is_null() {
                    PyErr_SetString(
                        rt,
                        c"internal panic initializing audio_device_sink".as_ptr(),
                    );
                }
            }
            ptr::null_mut()
        }
    }
}
