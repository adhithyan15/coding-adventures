//! # `classes` — JS `Graph` and `Runtime` classes for the addon
//!
//! **MX07 Phase 2b.**  Exposes the class-based API described in
//! [MX07 §"The binding surface"](../../../specs/MX07-matrix-rust-napi.md):
//!
//! ```javascript
//! const m = require("./matrix_rust_napi.node");
//!
//! // Graph — wraps a Rust `matrix_ir::Graph` parsed from its JSON
//! // wire format.  Holds a Box<Graph> behind the scenes via napi_wrap.
//! const graph = new m.Graph(jsonString);        // direct ctor
//! //  or:    m.Graph.fromJson(jsonString)       // static-method sugar
//! const json = graph.toJson();                  // re-serialise
//! const summary = graph.describe();             // "Graph(tensors=4, ops=3, ...)"
//!
//! // Runtime — owns the CPU executor (and, when registered, GPU
//! // backends via matrix-runtime's planner).
//! const rt = new m.Runtime();                   // direct ctor
//! //  or:    m.Runtime.create()                 // static-method sugar
//! const outputs = rt.run(graph, [buf1, buf2]);  // Buffer[] in -> Buffer[] out
//! ```
//!
//! ## Why this exists alongside the Phase 1/2 string-only API
//!
//! Phase 1 (`graphRoundTripJson`) and Phase 2 (`runGraphOnCpu` JSON
//! envelope) were string-in / string-out functions — perfectly fine
//! for proving the pipeline works, but with two costs:
//!
//! * **JSON re-parsing on every call.**  A real consumer that runs a
//!   given graph 1000 times in a hot loop pays the JSON parse cost
//!   1000 times.  Wrapping the parsed `Graph` in a JS handle pays it
//!   once.
//! * **Hex-encoded bytes are 2× the wire cost.**  `Buffer[]` carries
//!   raw bytes; no encoding overhead, no copy through a string.
//!
//! Phase 2b addresses both: `new Graph(json)` pays the parse cost
//! once, and `rt.run(graph, Buffer[])` exchanges raw bytes via the
//! workspace `node-bridge` Buffer helpers added in PR #3529.
//!
//! ## Class registration pattern
//!
//! `napi_define_class` creates a JS constructor function whose body
//! is the `graph_ctor` / `runtime_ctor` callback below.  We store
//! the constructor `napi_value` in a static `AtomicUsize` so static
//! methods like `Graph.fromJson` can later look it up and pass it to
//! `napi_new_instance` — mirroring the font-parser-node pattern
//! (which fixed a Worker-thread race in its Finding 3.1 by using
//! the same atomic-storage approach).

use std::ffi::CString;
use std::os::raw::c_void;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

use matrix_ir::Graph;
use matrix_ir_json::{decode, encode};
use node_bridge::{
    create_function, define_class, get_cb_info, method_property, napi_callback_info,
    napi_create_reference, napi_env, napi_get_reference_value, napi_new_instance, napi_ref,
    napi_set_named_property, napi_unwrap, napi_value, napi_wrap, set_named_property,
    str_from_js, str_to_js, throw_error, undefined, vec_buf_from_js, vec_buf_to_js, NAPI_OK,
};

use crate::exec::run_graph_on_cpu;

// ─────────────────────────────────────────────────────────────────────────────
// SECURITY: type-tag discriminator for napi_wrap'd payloads.
//
// `napi_unwrap` is type-agnostic by N-API design — it returns whichever
// raw pointer was stored by *any* previous `napi_wrap` call in the env,
// regardless of which JS class the object belongs to.  Without a
// software-level type tag, a JS caller could do:
//
//     rt.run(rt, [])             // pass Runtime where Graph is expected
//     graph.toJson.call(rt)      // call Graph method on Runtime
//
// `unwrap_graph` would then return a pointer to a `Box<WrappedRuntime>`
// cast as `&Graph`.  Reading `graph.tensors.len()` reads (ptr, len, cap)
// out of bounds — immediate UB, near-guaranteed crash or worse.
//
// Fix: every napi_wrap'd payload in this crate starts with a 16-byte
// `tag: [u64; 2]` prefix.  Each class has its own constant tag value;
// `unwrap_graph` / `unwrap_runtime` read the first 16 bytes and reject
// the call if the tag doesn't match before dereferencing the rest as
// `&Graph` / `&WrappedRuntime`.  Both `WrappedGraph` and `WrappedRuntime`
// share the `[u64; 2]` prefix layout, so reading 16 bytes from any
// pointer we ourselves stored is safe.  For cross-addon wraps (a JS
// caller passing in an object wrapped by a different addon in the same
// env), the 128-bit tag collision probability is ~2^-128 — effectively
// zero — and even a colliding tag would still fail because the rest of
// the layout wouldn't match a real Graph.
//
// The long-term clean answer is `napi_type_tag_object` /
// `napi_check_object_type_tag` (N-API v8+).  Adopting those requires
// extending node-bridge with new extern declarations + safe wrappers;
// deferred to a follow-up PR.  For now the tagged-enum approach fully
// defends against the reachable Runtime↔Graph confusion path while
// staying contained within matrix-rust-napi.

const GRAPH_TAG: [u64; 2] = [0x4D58_4952_4750_4831, 0x436F_6469_6E67_4156];
const RUNTIME_TAG: [u64; 2] = [0x4D58_4952_5254_4D31, 0x436F_6469_6E67_4156];

/// What we actually store behind every `Graph` JS object's `napi_wrap`.
/// `#[repr(C)]` pins the field order so `unwrap_graph` can safely read
/// the leading tag bytes.
#[repr(C)]
struct WrappedGraph {
    tag: [u64; 2],
    inner: Graph,
}

/// What we actually store behind every `Runtime` JS object's
/// `napi_wrap`.  Currently no real state — kept as a `[u64; 2]` so
/// `unwrap_runtime` can validate the tag with the same 16-byte read
/// shape that `unwrap_graph` uses.
#[repr(C)]
struct WrappedRuntime {
    tag: [u64; 2],
}

// ─────────────────────────────────────────────────────────────────────────────
// Static storage for the class constructor references.
//
// **Why `napi_ref` and not `napi_value`?**
//
// `napi_value` is a "local reference" — valid only inside the current
// handle scope (typically the duration of one N-API callback).
// Storing a `napi_value` in a `static` and dereferencing it from a
// later callback uses a stale handle; `napi_new_instance` returns
// `napi_invalid_arg` (status 1) because the `napi_value` no longer
// points at a live JS function.  This bit us on the first run of the
// MX07 Phase 4 smoke tests — `Runtime.create()` returned `undefined`
// because the constructor handle stored from `napi_register_module_v1`
// was already invalid by the time JS first called the static method.
//
// `napi_ref` is the persistent equivalent: created with refcount 1
// via `napi_create_reference`, it keeps the wrapped JS value alive
// across handle scopes until explicitly deleted via
// `napi_delete_reference`.  We use that for the class constructors so
// `Graph.fromJson` and `Runtime.create` can resolve them from any
// later JS-triggered callback.
//
// Worker threads still load addons concurrently, so we keep the
// `AtomicUsize` Release/Acquire publish-once pattern from
// font-parser-node's Finding 3.1 fix.  Storing `napi_ref` (also a
// pointer-sized opaque) instead of `napi_value` is the only change.
//
// **What about the lesson for node-bridge?**
//
// font-parser-node uses the `napi_value` AtomicUsize pattern too.
// As far as we can tell, no existing test in that crate actually
// exercises the failure mode (`fp.load` rejects every input we tried
// before reaching `napi_new_instance`), but the bug is latent there.
// `lessons.md` is updated to call this out so the next napi addon
// gets it right from day one.
// ─────────────────────────────────────────────────────────────────────────────

static GRAPH_CTOR_REF: AtomicUsize = AtomicUsize::new(0);
static RUNTIME_CTOR_REF: AtomicUsize = AtomicUsize::new(0);

fn store_graph_ctor_ref(r: napi_ref) {
    GRAPH_CTOR_REF.store(r as usize, Ordering::Release);
}
fn load_graph_ctor_ref() -> napi_ref {
    GRAPH_CTOR_REF.load(Ordering::Acquire) as napi_ref
}
fn store_runtime_ctor_ref(r: napi_ref) {
    RUNTIME_CTOR_REF.store(r as usize, Ordering::Release);
}
fn load_runtime_ctor_ref() -> napi_ref {
    RUNTIME_CTOR_REF.load(Ordering::Acquire) as napi_ref
}

/// Resolve a stored class-constructor `napi_ref` into a `napi_value`
/// valid for the current handle scope.  Returns `None` (with a JS
/// exception thrown) if the reference is null (registration never
/// ran) or `napi_get_reference_value` fails.
unsafe fn resolve_ctor(env: napi_env, ctor_ref: napi_ref, class_name: &str) -> Option<napi_value> {
    if ctor_ref.is_null() {
        throw_error(
            env,
            &format!(
                "{}: class not initialised (napi_register_module_v1 didn't run?)",
                class_name
            ),
        );
        return None;
    }
    let mut value: napi_value = ptr::null_mut();
    let status = napi_get_reference_value(env, ctor_ref, &mut value);
    if status != NAPI_OK || value.is_null() {
        throw_error(
            env,
            &format!(
                "{}: napi_get_reference_value failed (status {})",
                class_name, status
            ),
        );
        return None;
    }
    Some(value)
}

// ─────────────────────────────────────────────────────────────────────────────
// Graph class
// ─────────────────────────────────────────────────────────────────────────────

/// GC finalizer: called by Node when the Graph JS object is collected.
/// Drops the boxed `matrix_ir::Graph` to free the dataflow + constant
/// buffers.
///
/// # Safety
///
/// `data` is the `Box<Graph>` pointer we stored via `Box::into_raw`
/// in `graph_ctor`.  Node invokes us exactly once per object, after
/// the JS engine has guaranteed nothing can dereference the wrap
/// pointer anymore.
unsafe extern "C" fn finalize_graph(_env: napi_env, data: *mut c_void, _hint: *mut c_void) {
    if !data.is_null() {
        let _ = Box::from_raw(data as *mut WrappedGraph);
    }
}

/// Constructor for `new Graph(jsonString)`.
///
/// 1. Validate arg count (exactly 1).
/// 2. Extract the JS string into an owned `String`.
/// 3. Parse via `matrix-ir-json::decode`.
/// 4. Box it and attach via `napi_wrap` with the finalizer above.
///
/// On any error, throw a JS `Error` and return `this` (the bare
/// instance, unwrapped — calling any instance method on it will throw
/// via the unwrap-failure path).
///
/// # Safety
///
/// Invoked by Node as part of `new Graph(...)`; `env` and `info` are
/// valid for the duration of the call.
unsafe extern "C" fn graph_ctor(env: napi_env, info: napi_callback_info) -> napi_value {
    // Request 2 slots so over-arity is detected (same hardening as
    // graphRoundTripJson — see Phase 1 security review).
    let (this, args) = get_cb_info(env, info, 2);
    if args.len() != 1 {
        throw_error(
            env,
            &format!(
                "Graph constructor: expected exactly 1 argument (jsonString), got {}",
                args.len()
            ),
        );
        return this;
    }
    let json = match str_from_js(env, args[0]) {
        Some(s) => s,
        None => {
            throw_error(env, "Graph constructor: argument 0 must be a string");
            return this;
        }
    };
    let graph = match decode(&json) {
        Ok(g) => g,
        Err(e) => {
            throw_error(env, &format!("Graph constructor: parse failed: {:?}", e));
            return this;
        }
    };

    // SECURITY (mirroring font-parser-node Finding 3.5): check
    // `napi_wrap` status BEFORE letting `Box::into_raw` leak.  If
    // wrap fails the box would never see a finalizer.
    //
    // Note we wrap a `WrappedGraph` (tag + inner) — see the
    // `GRAPH_TAG` / `unwrap_graph` security note at the top of the
    // module.
    let boxed = Box::into_raw(Box::new(WrappedGraph {
        tag: GRAPH_TAG,
        inner: graph,
    }));
    let status = napi_wrap(
        env,
        this,
        boxed as *mut c_void,
        Some(finalize_graph),
        ptr::null_mut(),
        ptr::null_mut(),
    );
    if status != NAPI_OK {
        // Wrap failed — drop the box manually to avoid leaking.
        let _ = Box::from_raw(boxed);
        throw_error(env, "Graph constructor: napi_wrap failed");
    }
    this
}

/// Helper: unwrap the `&Graph` from a JS object.  Validates the
/// type tag before dereferencing so callers can't trigger UB by
/// passing a `Runtime` (or any other wrapped object) where a `Graph`
/// is expected — see the module-level SECURITY note.
///
/// Returns `None` (with a JS exception set) on:
///   * object not napi_wrap'd in this env
///   * wrap pointer null
///   * type tag does not match `GRAPH_TAG`
///
/// # Safety
///
/// Caller guarantees `obj` is in the current call frame.  The
/// returned reference is valid only for the duration of the call —
/// we hand back `'static` because the pointer lives behind a
/// napi_wrap finalizer-owned `Box` whose lifetime is tied to the JS
/// object, which is itself live for at least the current call.
unsafe fn unwrap_graph(env: napi_env, obj: napi_value) -> Option<&'static Graph> {
    let mut p: *mut c_void = ptr::null_mut();
    let status = napi_unwrap(env, obj, &mut p);
    if status != NAPI_OK || p.is_null() {
        throw_error(env, "argument is not a wrapped Graph");
        return None;
    }
    // SAFETY: every napi_wrap'd payload in this crate starts with a
    // `tag: [u64; 2]` prefix (`WrappedGraph` and `WrappedRuntime`
    // both `#[repr(C)]` with that leading field).  Reading the first
    // 16 bytes from any pointer we ourselves stored is therefore
    // sound.  For wrapped objects from other addons in the same env,
    // collision probability with `GRAPH_TAG` is ~2^-128.
    let tag = (p as *const [u64; 2]).read();
    if tag != GRAPH_TAG {
        throw_error(env, "argument is not a Graph (type tag mismatch)");
        return None;
    }
    // Now safe to view the full WrappedGraph and return a reference
    // to its `inner` field.
    Some(&(*(p as *const WrappedGraph)).inner)
}

/// `graph.toJson(): string` — re-serialise the wrapped Graph through
/// `matrix-ir-json::encode`.  Always succeeds (encode is infallible
/// per the matrix-ir-json crate's contract).
unsafe extern "C" fn graph_to_json(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let graph = match unwrap_graph(env, this) {
        Some(g) => g,
        None => return undefined(env),
    };
    let json = encode(graph);
    str_to_js(env, &json)
}

/// `graph.describe(): string` — return a short human-readable
/// summary.  Useful for logging and debugging without dumping the
/// full JSON.
unsafe extern "C" fn graph_describe(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, _args) = get_cb_info(env, info, 0);
    let graph = match unwrap_graph(env, this) {
        Some(g) => g,
        None => return undefined(env),
    };
    let summary = format!(
        "Graph(tensors={}, ops={}, inputs={}, outputs={}, constants={})",
        graph.tensors.len(),
        graph.ops.len(),
        graph.inputs.len(),
        graph.outputs.len(),
        graph.constants.len(),
    );
    str_to_js(env, &summary)
}

/// `Graph.fromJson(jsonString): Graph` — static-method sugar.  Just
/// calls the constructor; semantically identical to
/// `new Graph(jsonString)`.
unsafe extern "C" fn graph_from_json_static(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 2);
    if args.len() != 1 {
        throw_error(
            env,
            &format!(
                "Graph.fromJson: expected exactly 1 argument (jsonString), got {}",
                args.len()
            ),
        );
        return undefined(env);
    }
    let ctor = match resolve_ctor(env, load_graph_ctor_ref(), "Graph.fromJson") {
        Some(c) => c,
        None => return undefined(env),
    };
    let mut instance: napi_value = ptr::null_mut();
    // Forward args[0] verbatim to the constructor — it does all the
    // parsing + wrapping work.
    let status = napi_new_instance(env, ctor, 1, args.as_ptr(), &mut instance);
    if status != NAPI_OK {
        throw_error(
            env,
            &format!(
                "Graph.fromJson: napi_new_instance failed (status {})",
                status
            ),
        );
        return undefined(env);
    }
    if instance.is_null() {
        // If status was OK but instance is null, the constructor
        // itself threw and we should let the pending exception
        // propagate.
        return undefined(env);
    }
    instance
}

// ─────────────────────────────────────────────────────────────────────────────
// Runtime class
// ─────────────────────────────────────────────────────────────────────────────
//
// In v0.x the Runtime is stateless (each call to `run` constructs a
// fresh `matrix_runtime::Runtime` + `CpuExecutor` internally).  We
// still expose it as a class so the API surface matches MX07's
// eventual shape — once we add things like Runtime-level option flags
// or a long-lived executor pool, the JS surface won't have to change.
//
// The constructor wraps a `()` marker; the finalizer drops it.  No
// runtime state actually lives on the JS side yet.

unsafe extern "C" fn finalize_runtime(_env: napi_env, data: *mut c_void, _hint: *mut c_void) {
    if !data.is_null() {
        let _ = Box::from_raw(data as *mut WrappedRuntime);
    }
}

unsafe extern "C" fn runtime_ctor(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 1);
    if !args.is_empty() {
        throw_error(
            env,
            &format!(
                "Runtime constructor: expected 0 arguments, got {}",
                args.len()
            ),
        );
        return this;
    }
    let boxed = Box::into_raw(Box::new(WrappedRuntime { tag: RUNTIME_TAG }));
    let status = napi_wrap(
        env,
        this,
        boxed as *mut c_void,
        Some(finalize_runtime),
        ptr::null_mut(),
        ptr::null_mut(),
    );
    if status != NAPI_OK {
        let _ = Box::from_raw(boxed);
        throw_error(env, "Runtime constructor: napi_wrap failed");
    }
    this
}

/// Helper: validate that `obj` is a wrapped Runtime by checking its
/// type tag.  Same shape as `unwrap_graph` — see the module-level
/// SECURITY note for the rationale.
unsafe fn unwrap_runtime(env: napi_env, obj: napi_value) -> Option<&'static WrappedRuntime> {
    let mut p: *mut c_void = ptr::null_mut();
    let status = napi_unwrap(env, obj, &mut p);
    if status != NAPI_OK || p.is_null() {
        throw_error(env, "argument is not a wrapped Runtime");
        return None;
    }
    let tag = (p as *const [u64; 2]).read();
    if tag != RUNTIME_TAG {
        throw_error(env, "argument is not a Runtime (type tag mismatch)");
        return None;
    }
    Some(&*(p as *const WrappedRuntime))
}

/// `runtime.run(graph: Graph, inputs: Buffer[]): Buffer[]` — the
/// headline method.  Marshals across the napi boundary, dispatches
/// to the pure-Rust `run_graph_on_cpu`, packages outputs as JS
/// Buffers.
///
/// All bytes are copied at the boundary (no shared-memory views) for
/// the detachment-safety reasons documented in
/// `node-bridge::buffer_from_js`.  This is the right default; if
/// profiling ever shows the copies matter for a real workload, the
/// follow-up is to use `napi_create_external_buffer` for outputs and
/// `napi_get_buffer_info` directly for inputs with explicit lifetime
/// management.
unsafe extern "C" fn runtime_run(env: napi_env, info: napi_callback_info) -> napi_value {
    let (this, args) = get_cb_info(env, info, 3);
    if args.len() != 2 {
        throw_error(
            env,
            &format!(
                "runtime.run: expected exactly 2 arguments (graph, inputsArray), got {}",
                args.len()
            ),
        );
        return undefined(env);
    }
    // Establish that `this` is a wrapped Runtime.  The type-tag check
    // inside `unwrap_runtime` defends against being called as
    // `m.Runtime.prototype.run.call(someGraph, ...)` — without the
    // tag, that would slip past a bare `napi_unwrap` ok-status check
    // and the next line would read a `Box<WrappedGraph>` as if it
    // were a Runtime.
    if unwrap_runtime(env, this).is_none() {
        // unwrap_runtime already threw a precise error.
        return undefined(env);
    }

    // arg 0: the Graph instance.
    let graph = match unwrap_graph(env, args[0]) {
        Some(g) => g,
        None => return undefined(env), // unwrap_graph already threw
    };

    // arg 1: Array<Buffer> of inputs.
    let inputs = match vec_buf_from_js(env, args[1]) {
        Some(xs) => xs,
        None => {
            throw_error(
                env,
                "runtime.run: argument 1 must be an Array of Buffer (each tensor's LE bytes)",
            );
            return undefined(env);
        }
    };

    // Execute.
    let outputs = match run_graph_on_cpu(graph, &inputs) {
        Ok(out) => out,
        Err(msg) => {
            throw_error(env, &format!("runtime.run: {}", msg));
            return undefined(env);
        }
    };

    // Marshal outputs back as Array<Buffer>.
    vec_buf_to_js(env, &outputs)
}

unsafe extern "C" fn runtime_create_static(
    env: napi_env,
    info: napi_callback_info,
) -> napi_value {
    let (_this, args) = get_cb_info(env, info, 1);
    if !args.is_empty() {
        throw_error(
            env,
            &format!(
                "Runtime.create: expected 0 arguments, got {}",
                args.len()
            ),
        );
        return undefined(env);
    }
    let ctor = match resolve_ctor(env, load_runtime_ctor_ref(), "Runtime.create") {
        Some(c) => c,
        None => return undefined(env),
    };
    let mut instance: napi_value = ptr::null_mut();
    let no_args: [napi_value; 0] = [];
    let status = napi_new_instance(env, ctor, 0, no_args.as_ptr(), &mut instance);
    if status != NAPI_OK {
        throw_error(
            env,
            &format!(
                "Runtime.create: napi_new_instance failed (status {})",
                status
            ),
        );
        return undefined(env);
    }
    if instance.is_null() {
        return undefined(env);
    }
    instance
}

/// Define both classes, attach their static-method sugar, and bind
/// them onto the addon's `exports` object as `Graph` and `Runtime`.
///
/// # Safety
///
/// Called exactly once at module load, with a valid `env` and
/// `exports`.
pub unsafe fn register(env: napi_env, exports: napi_value) {
    // ── Graph ──────────────────────────────────────────────────
    let graph_class = define_class(
        env,
        "Graph",
        Some(graph_ctor),
        &[
            method_property("toJson", Some(graph_to_json)),
            method_property("describe", Some(graph_describe)),
        ],
    );
    // SAFETY: napi_value is a local handle valid only in this scope.
    // Wrap it in a persistent napi_ref (refcount 1) so static-method
    // callbacks fired from later JS calls can still resolve the
    // class constructor.  See the GRAPH_CTOR_REF docs above for the
    // full rationale.
    let mut graph_ref: napi_ref = ptr::null_mut();
    let st = napi_create_reference(env, graph_class, 1, &mut graph_ref);
    if st != NAPI_OK {
        throw_error(
            env,
            &format!(
                "Graph: napi_create_reference failed (status {})",
                st
            ),
        );
        return;
    }
    store_graph_ctor_ref(graph_ref);

    // Attach the static-method sugar `Graph.fromJson` on the class
    // itself (JS classes are functions; you can hang properties off
    // them).  We use the raw napi_set_named_property here because
    // `node-bridge::set_named_property` takes &str and we already
    // have a CString lying around for clarity.
    let from_json = create_function(env, "fromJson", Some(graph_from_json_static));
    let key = CString::new("fromJson").expect("name has no NUL");
    napi_set_named_property(env, graph_class, key.as_ptr(), from_json);

    set_named_property(env, exports, "Graph", graph_class);

    // ── Runtime ────────────────────────────────────────────────
    let runtime_class = define_class(
        env,
        "Runtime",
        Some(runtime_ctor),
        &[method_property("run", Some(runtime_run))],
    );
    // SAFETY: same as for graph_ref above — wrap the local handle in
    // a persistent napi_ref so Runtime.create() can resolve it later.
    let mut runtime_ref: napi_ref = ptr::null_mut();
    let st = napi_create_reference(env, runtime_class, 1, &mut runtime_ref);
    if st != NAPI_OK {
        throw_error(
            env,
            &format!(
                "Runtime: napi_create_reference failed (status {})",
                st
            ),
        );
        return;
    }
    store_runtime_ctor_ref(runtime_ref);

    let create = create_function(env, "create", Some(runtime_create_static));
    let key = CString::new("create").expect("name has no NUL");
    napi_set_named_property(env, runtime_class, key.as_ptr(), create);

    set_named_property(env, exports, "Runtime", runtime_class);
}

// ─────────────────────────────────────────────────────────────────────────────
// Public registration — call once from napi_register_module_v1.
// ─────────────────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────────────────
// Pure-Rust tests
//
// The N-API surface itself isn't exercisable without a Node runtime
// (Phase 4 lands `node --test` smoke); here we verify the two
// invariants the type-tag defence relies on:
//
//   1. `GRAPH_TAG` and `RUNTIME_TAG` are distinct.  If a future
//      change ever made them equal, the cross-class confusion check
//      would silently fail.
//   2. `WrappedGraph` and `WrappedRuntime` start with `tag: [u64; 2]`
//      at offset 0 with the right size.  If a future change broke
//      the field order or alignment, `unwrap_*`'s 16-byte read
//      would either read padding (false-negative tag check) or
//      stray into the next field (UB).
//
// These are compile-time invariants but the unit tests make them
// explicit so the next reader knows they're load-bearing.
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_and_runtime_tags_are_distinct() {
        assert_ne!(
            GRAPH_TAG, RUNTIME_TAG,
            "type-tag defence relies on distinct class tags"
        );
    }

    #[test]
    fn wrapped_graph_starts_with_tag_at_offset_zero() {
        use std::mem::offset_of;
        assert_eq!(
            offset_of!(WrappedGraph, tag),
            0,
            "unwrap_graph reads 16 bytes from offset 0 — tag must live there"
        );
    }

    #[test]
    fn wrapped_runtime_starts_with_tag_at_offset_zero() {
        use std::mem::offset_of;
        assert_eq!(
            offset_of!(WrappedRuntime, tag),
            0,
            "unwrap_runtime reads 16 bytes from offset 0 — tag must live there"
        );
    }

    #[test]
    fn tag_size_matches_unwrap_read_size() {
        use std::mem::size_of;
        assert_eq!(
            size_of::<[u64; 2]>(),
            16,
            "unwrap_* helpers read exactly 16 bytes via (p as *const [u64; 2]).read()"
        );
    }
}
