//! Panic-safe C ABI support for [`mosaic_app_runtime`].
//!
//! A final application crate invokes [`export_mosaic_app!`] once. The macro emits
//! the fixed symbols declared in `include/mosaic_app.h`; all lifecycle, memory,
//! diagnostics, and panic behavior remains package-independent in this crate.

use std::ffi::c_void;

/// Maximum bytes returned for an error diagnostic, including the truncation mark.
pub const MAX_DIAGNOSTIC_BYTES: usize = 16 * 1024;

/// Borrowed caller-owned input bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MosaicBytes {
    pub ptr: *const u8,
    pub len: usize,
}

impl MosaicBytes {
    pub fn new(bytes: &[u8]) -> Self {
        Self {
            ptr: bytes.as_ptr(),
            len: bytes.len(),
        }
    }
}

/// Rust-owned bytes returned across the ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MosaicBuffer {
    pub ptr: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

impl MosaicBuffer {
    pub const fn empty() -> Self {
        Self {
            ptr: std::ptr::null_mut(),
            len: 0,
            capacity: 0,
        }
    }
}

impl Default for MosaicBuffer {
    fn default() -> Self {
        Self::empty()
    }
}

/// Opaque pointer to an application-specific runtime allocation.
pub type MosaicHandle = *mut c_void;

/// Stable status values mirrored by `include/mosaic_app.h`.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MosaicStatus {
    Ok = 0,
    InvalidArgument = 1,
    DecodeError = 2,
    ProtocolError = 3,
    ApplicationError = 4,
    EncodeError = 5,
    Panic = 6,
    Poisoned = 7,
}

/// Emit the fixed Mosaic C ABI for one concrete Rust application type.
///
/// Invoke this macro once in the final `cdylib`/`staticlib` crate. The factory
/// expression is evaluated once per `mosaic_app_create` call.
#[macro_export]
macro_rules! export_mosaic_app {
    ($app:ty, $factory:expr) => {
        #[no_mangle]
        pub unsafe extern "C" fn mosaic_app_create(
            start: $crate::MosaicBytes,
            app: *mut $crate::MosaicHandle,
            initial_update: *mut $crate::MosaicBuffer,
        ) -> $crate::MosaicStatus {
            $crate::bridge::create::<$app, _>(start, app, initial_update, || $factory)
        }

        #[no_mangle]
        pub unsafe extern "C" fn mosaic_app_dispatch(
            app: $crate::MosaicHandle,
            event: $crate::MosaicBytes,
            update: *mut $crate::MosaicBuffer,
        ) -> $crate::MosaicStatus {
            $crate::bridge::dispatch::<$app>(app, event, update)
        }

        #[no_mangle]
        pub unsafe extern "C" fn mosaic_app_snapshot(
            app: $crate::MosaicHandle,
            snapshot: *mut $crate::MosaicBuffer,
        ) -> $crate::MosaicStatus {
            $crate::bridge::snapshot::<$app>(app, snapshot)
        }

        #[no_mangle]
        pub unsafe extern "C" fn mosaic_app_restore(
            app: $crate::MosaicHandle,
            snapshot: $crate::MosaicBytes,
            update: *mut $crate::MosaicBuffer,
        ) -> $crate::MosaicStatus {
            $crate::bridge::restore::<$app>(app, snapshot, update)
        }

        #[no_mangle]
        pub unsafe extern "C" fn mosaic_buffer_free(buffer: $crate::MosaicBuffer) {
            $crate::bridge::buffer_free(buffer)
        }

        #[no_mangle]
        pub unsafe extern "C" fn mosaic_app_destroy(app: $crate::MosaicHandle) {
            $crate::bridge::destroy::<$app>(app)
        }
    };
}

/// Implementation helpers used by [`export_mosaic_app!`].
///
/// These are public so a macro expanded in another crate can call them. Hosts
/// should use the exported C symbols instead.
#[doc(hidden)]
pub mod bridge {
    use super::{MosaicBuffer, MosaicBytes, MosaicHandle, MosaicStatus, MAX_DIAGNOSTIC_BYTES};
    use mosaic_app_runtime::{
        Event, MosaicApp, MosaicRuntime, RuntimeError, Snapshot, StartContext,
    };
    use std::any::Any;
    use std::fmt::Display;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::ptr;
    use std::slice;

    struct CapiApp<A: MosaicApp> {
        runtime: MosaicRuntime<A>,
        poisoned: bool,
    }

    struct Failure {
        status: MosaicStatus,
        diagnostic: String,
    }

    impl Failure {
        fn new(status: MosaicStatus, diagnostic: impl Into<String>) -> Self {
            Self {
                status,
                diagnostic: diagnostic.into(),
            }
        }

        fn panic(payload: Box<dyn Any + Send>) -> Self {
            let detail = if let Some(message) = payload.downcast_ref::<&str>() {
                (*message).to_string()
            } else if let Some(message) = payload.downcast_ref::<String>() {
                message.clone()
            } else {
                "non-string panic payload".to_string()
            };
            Self::new(MosaicStatus::Panic, format!("Rust panic: {detail}"))
        }
    }

    fn map_runtime_error<E: Display>(error: RuntimeError<E>) -> Failure {
        let status = if matches!(&error, RuntimeError::Application(_)) {
            MosaicStatus::ApplicationError
        } else {
            MosaicStatus::ProtocolError
        };
        Failure::new(status, error.to_string())
    }

    unsafe fn read_input(input: MosaicBytes) -> Result<Vec<u8>, Failure> {
        if input.len == 0 {
            return Ok(Vec::new());
        }
        if input.ptr.is_null() {
            return Err(Failure::new(
                MosaicStatus::InvalidArgument,
                "input pointer is null with non-zero length",
            ));
        }
        Ok(slice::from_raw_parts(input.ptr, input.len).to_vec())
    }

    fn into_buffer(mut bytes: Vec<u8>) -> MosaicBuffer {
        let buffer = MosaicBuffer {
            ptr: bytes.as_mut_ptr(),
            len: bytes.len(),
            capacity: bytes.capacity(),
        };
        std::mem::forget(bytes);
        buffer
    }

    fn diagnostic_bytes(message: &str) -> Vec<u8> {
        if message.len() <= MAX_DIAGNOSTIC_BYTES {
            return message.as_bytes().to_vec();
        }

        const MARK: &str = "... [truncated]";
        let mut boundary = MAX_DIAGNOSTIC_BYTES - MARK.len();
        while !message.is_char_boundary(boundary) {
            boundary -= 1;
        }
        let mut bytes = message.as_bytes()[..boundary].to_vec();
        bytes.extend_from_slice(MARK.as_bytes());
        bytes
    }

    unsafe fn prepare_output(output: *mut MosaicBuffer) -> Result<(), Failure> {
        if output.is_null() {
            return Err(Failure::new(
                MosaicStatus::InvalidArgument,
                "output buffer pointer is null",
            ));
        }
        ptr::write(output, MosaicBuffer::empty());
        Ok(())
    }

    unsafe fn finish(output: *mut MosaicBuffer, result: Result<Vec<u8>, Failure>) -> MosaicStatus {
        match result {
            Ok(bytes) => {
                ptr::write(output, into_buffer(bytes));
                MosaicStatus::Ok
            }
            Err(failure) => {
                ptr::write(output, into_buffer(diagnostic_bytes(&failure.diagnostic)));
                failure.status
            }
        }
    }

    fn encode<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, Failure> {
        serde_json::to_vec(value).map_err(|error| {
            Failure::new(
                MosaicStatus::EncodeError,
                format!("failed to encode Mosaic output: {error}"),
            )
        })
    }

    fn decode<'a, T: serde::Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, Failure> {
        serde_json::from_slice(bytes).map_err(|error| {
            Failure::new(
                MosaicStatus::DecodeError,
                format!("failed to decode Mosaic input: {error}"),
            )
        })
    }

    /// # Safety
    /// Pointers must follow the contract in `include/mosaic_app.h`.
    pub unsafe fn create<A, F>(
        start: MosaicBytes,
        app_out: *mut MosaicHandle,
        update_out: *mut MosaicBuffer,
        factory: F,
    ) -> MosaicStatus
    where
        A: MosaicApp,
        F: FnOnce() -> A,
    {
        if !app_out.is_null() {
            ptr::write(app_out, ptr::null_mut());
        }
        if let Err(failure) = prepare_output(update_out) {
            return failure.status;
        }
        if app_out.is_null() {
            return finish(
                update_out,
                Err(Failure::new(
                    MosaicStatus::InvalidArgument,
                    "application handle pointer is null",
                )),
            );
        }
        let result = catch_unwind(AssertUnwindSafe(|| {
            let start = read_input(start)?;
            let context: StartContext = decode(&start)?;
            let mut runtime = MosaicRuntime::new(factory());
            let update = runtime.start(context).map_err(map_runtime_error)?;
            let encoded = encode(&update)?;
            Ok::<_, Failure>((runtime, encoded))
        }));

        match result {
            Ok(Ok((runtime, encoded))) => {
                let app = Box::new(CapiApp {
                    runtime,
                    poisoned: false,
                });
                ptr::write(app_out, Box::into_raw(app).cast());
                finish(update_out, Ok(encoded))
            }
            Ok(Err(failure)) => finish(update_out, Err(failure)),
            Err(payload) => finish(update_out, Err(Failure::panic(payload))),
        }
    }

    unsafe fn with_app<A, F>(
        app: MosaicHandle,
        output: *mut MosaicBuffer,
        operation: F,
    ) -> MosaicStatus
    where
        A: MosaicApp,
        F: FnOnce(&mut MosaicRuntime<A>) -> Result<Vec<u8>, Failure>,
    {
        if let Err(failure) = prepare_output(output) {
            return failure.status;
        }
        if app.is_null() {
            return finish(
                output,
                Err(Failure::new(
                    MosaicStatus::InvalidArgument,
                    "application handle is null",
                )),
            );
        }

        let app = &mut *app.cast::<CapiApp<A>>();
        if app.poisoned {
            return finish(
                output,
                Err(Failure::new(
                    MosaicStatus::Poisoned,
                    "Mosaic application handle is poisoned after a Rust panic",
                )),
            );
        }

        match catch_unwind(AssertUnwindSafe(|| operation(&mut app.runtime))) {
            Ok(result) => finish(output, result),
            Err(payload) => {
                app.poisoned = true;
                finish(output, Err(Failure::panic(payload)))
            }
        }
    }

    /// # Safety
    /// Pointers must follow the contract in `include/mosaic_app.h`.
    pub unsafe fn dispatch<A: MosaicApp>(
        app: MosaicHandle,
        event: MosaicBytes,
        update: *mut MosaicBuffer,
    ) -> MosaicStatus {
        with_app::<A, _>(app, update, |runtime| {
            let event = read_input(event)?;
            let event: Event = decode(&event)?;
            let update = runtime.dispatch(event).map_err(map_runtime_error)?;
            encode(&update)
        })
    }

    /// # Safety
    /// Pointers must follow the contract in `include/mosaic_app.h`.
    pub unsafe fn snapshot<A: MosaicApp>(
        app: MosaicHandle,
        output: *mut MosaicBuffer,
    ) -> MosaicStatus {
        with_app::<A, _>(app, output, |runtime| {
            let snapshot = runtime.snapshot().map_err(map_runtime_error)?;
            encode(&snapshot)
        })
    }

    /// # Safety
    /// Pointers must follow the contract in `include/mosaic_app.h`.
    pub unsafe fn restore<A: MosaicApp>(
        app: MosaicHandle,
        snapshot: MosaicBytes,
        update: *mut MosaicBuffer,
    ) -> MosaicStatus {
        with_app::<A, _>(app, update, |runtime| {
            let snapshot = read_input(snapshot)?;
            let snapshot: Snapshot = decode(&snapshot)?;
            let update = runtime.restore(snapshot).map_err(map_runtime_error)?;
            encode(&update)
        })
    }

    /// # Safety
    /// `buffer` must be empty or an allocation returned by this ABI, and it must
    /// not have been freed previously.
    pub unsafe fn buffer_free(buffer: MosaicBuffer) {
        if !buffer.ptr.is_null() {
            drop(Vec::from_raw_parts(buffer.ptr, buffer.len, buffer.capacity));
        }
    }

    /// # Safety
    /// `app` must be null or a live handle returned by `create::<A, _>` for the
    /// exact same `A`, and it must not have been destroyed previously.
    pub unsafe fn destroy<A: MosaicApp>(app: MosaicHandle) {
        if app.is_null() {
            return;
        }
        let _ = catch_unwind(AssertUnwindSafe(|| {
            drop(Box::from_raw(app.cast::<CapiApp<A>>()));
        }));
    }

    #[cfg(test)]
    mod tests {
        use super::super::*;
        use super::diagnostic_bytes;
        use mosaic_app_runtime::{AppUpdate, Event, MosaicApp, Platform, Snapshot, StartContext};
        use serde_json::{json, Value};
        use std::error::Error;
        use std::fmt;
        use std::ptr;
        use std::slice;

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct TestError;

        impl fmt::Display for TestError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("requested application failure")
            }
        }

        impl Error for TestError {}

        #[derive(Default)]
        struct TestApp {
            count: u64,
        }

        impl MosaicApp for TestApp {
            type Error = TestError;

            fn start(&mut self, _context: StartContext) -> Result<AppUpdate, Self::Error> {
                Ok(AppUpdate::new(json!({ "count": self.count })))
            }

            fn dispatch(&mut self, event: Event) -> Result<AppUpdate, Self::Error> {
                match event.name.as_str() {
                    "increment" => self.count += 1,
                    "fail" => return Err(TestError),
                    "panic" => panic!("requested panic"),
                    _ => {}
                }
                Ok(AppUpdate::new(json!({ "count": self.count })))
            }

            fn snapshot(&self) -> Result<Option<Snapshot>, Self::Error> {
                Ok(Some(Snapshot {
                    schema: "test-counter".to_string(),
                    version: 1,
                    bytes: self.count.to_le_bytes().to_vec(),
                }))
            }

            fn restore(&mut self, snapshot: Snapshot) -> Result<AppUpdate, Self::Error> {
                let bytes: [u8; 8] = snapshot.bytes.try_into().map_err(|_| TestError)?;
                self.count = u64::from_le_bytes(bytes);
                Ok(AppUpdate::new(json!({ "count": self.count })))
            }
        }

        export_mosaic_app!(TestApp, TestApp::default());

        fn context() -> StartContext {
            StartContext::new("en-US", Platform::Linux)
        }

        fn bytes(value: &[u8]) -> MosaicBytes {
            MosaicBytes::new(value)
        }

        unsafe fn take(buffer: MosaicBuffer) -> Vec<u8> {
            let value = slice::from_raw_parts(buffer.ptr, buffer.len).to_vec();
            mosaic_buffer_free(buffer);
            value
        }

        unsafe fn take_text(buffer: MosaicBuffer) -> String {
            String::from_utf8(take(buffer)).unwrap()
        }

        unsafe fn create_app() -> (MosaicHandle, Value) {
            let encoded = serde_json::to_vec(&context()).unwrap();
            let mut handle = ptr::null_mut();
            let mut output = MosaicBuffer::empty();
            assert_eq!(
                mosaic_app_create(bytes(&encoded), &mut handle, &mut output),
                MosaicStatus::Ok
            );
            let update = serde_json::from_slice(&take(output)).unwrap();
            (handle, update)
        }

        unsafe fn call_event(
            handle: MosaicHandle,
            sequence: u64,
            name: &str,
        ) -> (MosaicStatus, MosaicBuffer) {
            let event = serde_json::to_vec(&Event::new(sequence, name, json!({}))).unwrap();
            let mut output = MosaicBuffer::empty();
            let status = mosaic_app_dispatch(handle, bytes(&event), &mut output);
            (status, output)
        }

        #[test]
        fn exports_full_lifecycle_and_owned_buffers() {
            unsafe {
                let (handle, initial): (MosaicHandle, Value) = create_app();
                assert!(!handle.is_null());
                assert_eq!(initial["revision"], 1);
                assert_eq!(initial["props"]["count"], 0);

                let (status, output) = call_event(handle, 1, "increment");
                assert_eq!(status, MosaicStatus::Ok);
                let update: Value = serde_json::from_slice(&take(output)).unwrap();
                assert_eq!(update["revision"], 2);
                assert_eq!(update["props"]["count"], 1);

                let mut output = MosaicBuffer::empty();
                assert_eq!(mosaic_app_snapshot(handle, &mut output), MosaicStatus::Ok);
                let snapshot: Snapshot = serde_json::from_slice(&take(output)).unwrap();

                let (status, output) = call_event(handle, 2, "increment");
                assert_eq!(status, MosaicStatus::Ok);
                mosaic_buffer_free(output);

                let encoded = serde_json::to_vec(&snapshot).unwrap();
                let mut output = MosaicBuffer::empty();
                assert_eq!(
                    mosaic_app_restore(handle, bytes(&encoded), &mut output),
                    MosaicStatus::Ok
                );
                let update: Value = serde_json::from_slice(&take(output)).unwrap();
                assert_eq!(update["revision"], 4);
                assert_eq!(update["props"]["count"], 1);
                mosaic_app_destroy(handle);
            }
        }

        #[test]
        fn maps_decode_protocol_and_application_errors_and_allows_retry() {
            unsafe {
                let mut handle = ptr::null_mut();
                let mut output = MosaicBuffer::empty();
                assert_eq!(
                    mosaic_app_create(bytes(b"not-json"), &mut handle, &mut output),
                    MosaicStatus::DecodeError
                );
                assert!(handle.is_null());
                assert!(take_text(output).contains("failed to decode"));

                let (handle, _) = create_app();
                let (status, output) = call_event(handle, 2, "increment");
                assert_eq!(status, MosaicStatus::ProtocolError);
                assert!(take_text(output).contains("expected 1"));

                let (status, output) = call_event(handle, 1, "fail");
                assert_eq!(status, MosaicStatus::ApplicationError);
                assert!(take_text(output).contains("requested application failure"));

                let (status, output) = call_event(handle, 1, "increment");
                assert_eq!(status, MosaicStatus::Ok);
                let update: Value = serde_json::from_slice(&take(output)).unwrap();
                assert_eq!(update["props"]["count"], 1);
                mosaic_app_destroy(handle);
            }
        }

        #[test]
        fn contains_panics_and_poisoned_handles_cannot_continue() {
            unsafe {
                let (handle, _) = create_app();
                let (status, output) = call_event(handle, 1, "panic");
                assert_eq!(status, MosaicStatus::Panic);
                assert!(take_text(output).contains("requested panic"));

                let (status, output) = call_event(handle, 1, "increment");
                assert_eq!(status, MosaicStatus::Poisoned);
                assert!(take_text(output).contains("poisoned"));
                mosaic_app_destroy(handle);
            }
        }

        #[test]
        fn validates_null_pointers_and_accepts_null_destroy_and_empty_free() {
            unsafe {
                let encoded = serde_json::to_vec(&context()).unwrap();
                let mut output = MosaicBuffer::empty();
                assert_eq!(
                    mosaic_app_create(bytes(&encoded), ptr::null_mut(), &mut output),
                    MosaicStatus::InvalidArgument
                );
                assert!(take_text(output).contains("handle pointer is null"));

                let mut handle = ptr::null_mut();
                assert_eq!(
                    mosaic_app_create(bytes(&encoded), &mut handle, ptr::null_mut()),
                    MosaicStatus::InvalidArgument
                );
                assert!(handle.is_null());

                let invalid = MosaicBytes {
                    ptr: ptr::null(),
                    len: 1,
                };
                let mut output = MosaicBuffer::empty();
                assert_eq!(
                    mosaic_app_create(invalid, &mut handle, &mut output),
                    MosaicStatus::InvalidArgument
                );
                assert!(take_text(output).contains("input pointer is null"));

                mosaic_buffer_free(MosaicBuffer::empty());
                mosaic_app_destroy(ptr::null_mut());
            }
        }

        #[test]
        fn bounds_utf8_diagnostics() {
            let message = "界".repeat(MAX_DIAGNOSTIC_BYTES);
            let bytes = diagnostic_bytes(&message);
            assert!(bytes.len() <= MAX_DIAGNOSTIC_BYTES);
            assert!(std::str::from_utf8(&bytes).is_ok());
            assert!(bytes.ends_with(b"... [truncated]"));
        }

        #[test]
        fn c_header_tracks_the_rust_protocol_and_status_values() {
            let header = include_str!("../include/mosaic_app.h");
            assert!(header.contains(&format!(
                "#define MOSAIC_APP_PROTOCOL_VERSION {}u",
                mosaic_app_runtime::PROTOCOL_VERSION
            )));
            for (name, value) in [
                ("OK", MosaicStatus::Ok),
                ("INVALID_ARGUMENT", MosaicStatus::InvalidArgument),
                ("DECODE_ERROR", MosaicStatus::DecodeError),
                ("PROTOCOL_ERROR", MosaicStatus::ProtocolError),
                ("APPLICATION_ERROR", MosaicStatus::ApplicationError),
                ("ENCODE_ERROR", MosaicStatus::EncodeError),
                ("PANIC", MosaicStatus::Panic),
                ("POISONED", MosaicStatus::Poisoned),
            ] {
                assert!(header.contains(&format!("MOSAIC_STATUS_{name} = {}", value as u32)));
            }
        }
    }
}
