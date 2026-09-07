//! Scalar WebAssembly exports over the standard Mosaic runtime.
use mosaic_app_runtime::{Event, MosaicApp, MosaicRuntime, Snapshot, StartContext};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "camelCase", deny_unknown_fields)]
enum Request {
    Create { context: StartContext },
    Dispatch { handle: u32, event: Event },
    Snapshot { handle: u32 },
    Restore { handle: u32, snapshot: Snapshot },
    Destroy { handle: u32 },
}

/// One module's independent application instances and owned transport buffers.
/// Handles are never reused, so a stale host cannot address a later application.
pub struct Bridge<A: MosaicApp> {
    apps: BTreeMap<u32, MosaicRuntime<A>>,
    next_handle: u32,
    buffers: BTreeMap<usize, Box<[u8]>>,
}

impl<A: MosaicApp> Default for Bridge<A> {
    fn default() -> Self {
        Self {
            apps: BTreeMap::new(),
            next_handle: 1,
            buffers: BTreeMap::new(),
        }
    }
}

impl<A: MosaicApp> Bridge<A> {
    /// Execute JSON without exposing application pointers to the host.
    pub fn request(&mut self, bytes: &[u8], factory: impl FnOnce() -> A) -> Value {
        let result = (|| -> Result<Value, String> {
            let request: Request = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            match request {
                Request::Create { context } => {
                    let handle = self.next_handle;
                    let next = handle
                        .checked_add(1)
                        .ok_or("application handle limit reached")?;
                    let mut app = MosaicRuntime::new(factory());
                    let update = app.start(context).map_err(|e| e.to_string())?;
                    self.apps.insert(handle, app);
                    self.next_handle = next;
                    Ok(json!({ "handle": handle, "update": update }))
                }
                Request::Destroy { handle } => {
                    self.apps
                        .remove(&handle)
                        .ok_or("unknown application handle")?;
                    Ok(Value::Null)
                }
                Request::Dispatch { handle, event } => {
                    let update = self
                        .app(handle)?
                        .dispatch(event)
                        .map_err(|e| e.to_string())?;
                    Ok(json!(update))
                }
                Request::Snapshot { handle } => {
                    let snapshot = self.app(handle)?.snapshot().map_err(|e| e.to_string())?;
                    Ok(json!(snapshot))
                }
                Request::Restore { handle, snapshot } => {
                    let update = self
                        .app(handle)?
                        .restore(snapshot)
                        .map_err(|e| e.to_string())?;
                    Ok(json!(update))
                }
            }
        })();
        match result {
            Ok(value) => json!({ "ok": true, "value": value }),
            Err(error) => json!({ "ok": false, "error": error }),
        }
    }

    fn app(&mut self, handle: u32) -> Result<&mut MosaicRuntime<A>, String> {
        self.apps
            .get_mut(&handle)
            .ok_or_else(|| "unknown application handle".into())
    }

    /// Allocate zeroed input bytes. Zero is failure; allocations are bounded to 64 MiB.
    pub fn alloc(&mut self, len: usize) -> usize {
        if len == 0 || len > 64 * 1024 * 1024 {
            return 0;
        }
        self.store(vec![0; len].into_boxed_slice())
    }

    fn store(&mut self, bytes: Box<[u8]>) -> usize {
        let ptr = bytes.as_ptr() as usize;
        self.buffers.insert(ptr, bytes);
        ptr
    }

    /// Consume an owned input buffer and return an owned length-prefixed response.
    /// Unknown pointers return zero, without dereferencing host-supplied addresses.
    pub fn call(&mut self, ptr: usize, factory: impl FnOnce() -> A) -> usize {
        let Some(input) = self.buffers.remove(&ptr) else {
            return 0;
        };
        let response = self.request(&input, factory).to_string().into_bytes();
        let Ok(len) = u32::try_from(response.len()) else {
            return 0;
        };
        let mut output = Vec::with_capacity(response.len() + 4);
        output.extend_from_slice(&len.to_le_bytes());
        output.extend_from_slice(&response);
        self.store(output.into_boxed_slice())
    }

    /// Release an input or response buffer; unknown pointers are harmless.
    pub fn free(&mut self, ptr: usize) {
        self.buffers.remove(&ptr);
    }
}

/// Export only scalar arguments on wasm32; native applications keep their C ABI.
#[macro_export]
macro_rules! export_mosaic_wasm {
    ($app:ty, $factory:expr) => {
        #[cfg(target_arch = "wasm32")]
        mod mosaic_wasm_exports {
            use super::*;
            std::thread_local! {
                static BRIDGE: std::cell::RefCell<$crate::Bridge<$app>> =
                    std::cell::RefCell::new($crate::Bridge::default());
            }
            #[no_mangle]
            pub extern "C" fn mosaic_wasm_alloc(len: usize) -> usize {
                BRIDGE.with(|bridge| bridge.borrow_mut().alloc(len))
            }
            #[no_mangle]
            pub extern "C" fn mosaic_wasm_call(ptr: usize) -> usize {
                BRIDGE.with(|bridge| bridge.borrow_mut().call(ptr, || $factory))
            }
            #[no_mangle]
            pub extern "C" fn mosaic_wasm_free(ptr: usize) {
                BRIDGE.with(|bridge| bridge.borrow_mut().free(ptr));
            }
        }
    };
}
