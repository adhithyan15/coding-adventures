### Changed — `from_pipeline` signature

The fourth argument changed from `manifest: Option<&()>` (a stub from
PR-1) to `registry: Option<&ComponentRegistry>`. Callers that don't
need component references continue to pass `None`; the behaviour for
them is identical to PR-4.

