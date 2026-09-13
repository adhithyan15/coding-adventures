---
category: Mosaic compiler pipeline
---

# N-API `napi_unwrap` is type-agnostic — when an addon registers more than one `napi_wrap`-ed class, each unwrap helper MUST check a type tag before casting

Discovered in matrix-rust-napi Phase 2b: the `Graph` and `Runtime` classes both went through `napi_wrap`, and `unwrap_graph` only checked `napi_unwrap`'s `status == NAPI_OK` + non-null pointer. A JS caller could pass a `Runtime` instance where a `Graph` was expected (`rt.run(rt, [])` or `g.toJson.call(rt)`); the bare unwrap returned the `Box<WrappedRuntime>` pointer, which `unwrap_graph` cast to `&Graph`, causing immediate UB on the first `graph.tensors.len()` read. **Fix pattern**: prefix every wrapped payload with `#[repr(C)] struct Wrapped<T> { tag: [u64; 2], inner: T }` using a class-specific 128-bit constant for `tag`, and validate the tag in every unwrap helper before dereferencing the rest. The "right" long-term answer is `napi_type_tag_object` / `napi_check_object_type_tag` (N-API v8+) — defer until node-bridge grows those bindings. Single-wrap-class addons like `font-parser-node` are not a model for this: with only one class, the bug doesn't exist. The lesson applies the moment a second wrapped class joins.
