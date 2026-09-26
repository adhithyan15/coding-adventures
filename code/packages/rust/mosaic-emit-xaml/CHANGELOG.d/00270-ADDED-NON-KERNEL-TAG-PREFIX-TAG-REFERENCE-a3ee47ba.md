### Added — Non-kernel tag → `<{prefix}:{Tag} ... />` reference

When a layout node's tag isn't in the UI29 kernel:

- **With a registry** AND the tag is registered → emits
  `<{prefix}:{Tag} ... />` with the registered xmlns prefix. The
  matching `xmlns:{prefix}="{value}"` declaration lands on the
  `<UserControl>` root tag.
- **With a registry** AND the tag is NOT registered →
  `PipelineEmitError::UnknownComponent(tag)` (the spec's intended
  error for missing manifest dependency).
- **Without a registry** → `PipelineEmitError::UnsupportedPrimitive(tag)`
  (preserves pre-PR-5 behaviour for callers that don't use packages).

Kernel primitives ALWAYS win over registry entries — if a registry
happens to define an entry for `Box` / `Text` / etc., the kernel
emitter is used and the registry entry is ignored. This protects
against accidental shadowing.

