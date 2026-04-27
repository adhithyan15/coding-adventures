//! Python bridge for the Rust audio device sink stack.

use std::ffi::{c_char, c_int, c_longlong, CString};
use std::ptr;

use audio_device_coreaudio::CoreAudioSink;
use audio_device_sink::{AudioSink, AudioSinkError, PcmFormat, PcmPlaybackBuffer, PlaybackReport};
use python_bridge::*;

#[allow(non_snake_case)]
extern "C" {
    fn PyLong_AsLongLong(obj: PyObjectPtr) -> c_longlong;
    fn PyErr_Occurred() -> PyObjectPtr;
    fn PyObject_IsInstance(inst: PyObjectPtr, cls: PyObjectPtr) -> c_int;
}

static AUDIO_DEVICE_ERROR_CLASS: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
static BOOL_CLASS: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

unsafe fn audio_device_error_class() -> PyObjectPtr {
    AUDIO_DEVICE_ERROR_CLASS
        .get()
        .map(|value| *value as PyObjectPtr)
        .unwrap_or_else(|| runtime_error_class())
}

unsafe fn set_audio_device_error(error: impl Into<String>) -> PyObjectPtr {
    set_error(audio_device_error_class(), &error.into());
    ptr::null_mut()
}

unsafe fn parse_arg_i64(args: PyObjectPtr, index: isize) -> Option<i64> {
    let arg = PyTuple_GetItem(args, index);
    if arg.is_null() {
        PyErr_Clear();
        return None;
    }
    py_i64(arg)
}

unsafe fn py_i64(obj: PyObjectPtr) -> Option<i64> {
    if py_is_bool(obj) {
        return None;
    }
    let value = PyLong_AsLongLong(obj);
    if !PyErr_Occurred().is_null() {
        PyErr_Clear();
        return None;
    }
    Some(value as i64)
}

unsafe fn py_is_bool(obj: PyObjectPtr) -> bool {
    let cls = BOOL_CLASS.get().copied().unwrap_or_else(|| {
        let builtins_name = CString::new("builtins").expect("literal has no NUL");
        let builtins = PyImport_ImportModule(builtins_name.as_ptr());
        if builtins.is_null() {
            PyErr_Clear();
            return 0;
        }
        let bool_name = CString::new("bool").expect("literal has no NUL");
        let bool_cls = PyObject_GetAttrString(builtins, bool_name.as_ptr());
        Py_DecRef(builtins);
        if bool_cls.is_null() {
            PyErr_Clear();
            return 0;
        }
        let _ = BOOL_CLASS.set(bool_cls as usize);
        bool_cls as usize
    }) as PyObjectPtr;

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

unsafe fn parse_samples(obj: PyObjectPtr, max_samples: usize) -> Option<Vec<i16>> {
    let iter = PyObject_GetIter(obj);
    if iter.is_null() {
        PyErr_Clear();
        return None;
    }

    let mut samples = Vec::new();
    loop {
        let item = PyIter_Next(iter);
        if item.is_null() {
            if !PyErr_Occurred().is_null() {
                PyErr_Clear();
                Py_DecRef(iter);
                return None;
            }
            break;
        }

        let Some(value) = py_i64(item) else {
            Py_DecRef(item);
            Py_DecRef(iter);
            return None;
        };
        Py_DecRef(item);

        if value < i16::MIN as i64 || value > i16::MAX as i64 {
            Py_DecRef(iter);
            return None;
        }
        samples.push(value as i16);
        if samples.len() > max_samples {
            Py_DecRef(iter);
            return None;
        }
    }

    Py_DecRef(iter);
    PyErr_Clear();
    Some(samples)
}

unsafe fn report_to_py(report: PlaybackReport) -> PyObjectPtr {
    let tuple = PyTuple_New(5);
    PyTuple_SetItem(tuple, 0, usize_to_py(report.frames_played));
    PyTuple_SetItem(tuple, 1, usize_to_py(report.sample_rate_hz as usize));
    PyTuple_SetItem(tuple, 2, usize_to_py(report.channel_count as usize));
    PyTuple_SetItem(tuple, 3, f64_to_py(report.duration_seconds));
    PyTuple_SetItem(tuple, 4, str_to_py(report.backend_name));
    tuple
}

fn max_samples_for(sample_rate_hz: u32) -> usize {
    (sample_rate_hz as f64 * audio_device_sink::MAX_BLOCKING_DURATION_SECONDS) as usize
}

fn map_error(error: AudioSinkError) -> String {
    error.to_string()
}

unsafe extern "C" fn py_play_samples(_module: PyObjectPtr, args: PyObjectPtr) -> PyObjectPtr {
    let sample_rate_hz = match parse_arg_i64(args, 1) {
        Some(value) if value > 0 && value <= audio_device_sink::MAX_SAMPLE_RATE_HZ as i64 => {
            value as u32
        }
        _ => {
            set_error(
                type_error_class(),
                "_play_samples() requires a positive integer sample_rate_hz",
            );
            return ptr::null_mut();
        }
    };
    let channel_count = match parse_arg_i64(args, 2) {
        Some(value) if value == audio_device_sink::SUPPORTED_CHANNEL_COUNT as i64 => value as u16,
        _ => {
            set_error(
                type_error_class(),
                "_play_samples() only supports mono channel_count=1",
            );
            return ptr::null_mut();
        }
    };

    let sample_arg = PyTuple_GetItem(args, 0);
    if sample_arg.is_null() {
        PyErr_Clear();
        set_error(type_error_class(), "_play_samples() requires samples");
        return ptr::null_mut();
    }
    let samples = match parse_samples(sample_arg, max_samples_for(sample_rate_hz)) {
        Some(value) => value,
        None => {
            set_error(
                value_error_class(),
                "_play_samples() samples must be signed 16-bit PCM integers within the V1 size limit",
            );
            return ptr::null_mut();
        }
    };

    let format = match PcmFormat::new(sample_rate_hz, channel_count, 16) {
        Ok(value) => value,
        Err(error) => return set_audio_device_error(map_error(error)),
    };
    let buffer = match PcmPlaybackBuffer::new(samples, format) {
        Ok(value) => value,
        Err(error) => return set_audio_device_error(map_error(error)),
    };

    match CoreAudioSink::new().play_blocking(&buffer) {
        Ok(report) => report_to_py(report),
        Err(error) => set_audio_device_error(map_error(error)),
    }
}

#[no_mangle]
pub unsafe extern "C" fn PyInit_audio_device_sink() -> PyObjectPtr {
    let methods: &'static mut [PyMethodDef; 2] = Box::leak(Box::new([
        PyMethodDef {
            ml_name: b"_play_samples\0".as_ptr() as *const c_char,
            ml_meth: Some(py_play_samples),
            ml_flags: METH_VARARGS,
            ml_doc: b"Play mono signed 16-bit PCM samples through the default audio sink.\0"
                .as_ptr() as *const c_char,
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
        m_name: b"audio_device_sink\0".as_ptr() as *const c_char,
        m_doc: b"Python bridge for Rust audio device sinks.\0".as_ptr() as *const c_char,
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

    let audio_device_error =
        new_exception("audio_device_sink", "AudioDeviceError", exception_class());
    if audio_device_error.is_null() {
        return ptr::null_mut();
    }
    Py_IncRef(audio_device_error);
    let _ = AUDIO_DEVICE_ERROR_CLASS.set(audio_device_error as usize);
    module_add_object(module, "AudioDeviceError", audio_device_error);
    module
}
