## Unreleased — declare the Unix reactor Windows build boundary

Add a Windows BUILD override that reports unsupported Unix file descriptors.
Keep all Rust source, Cargo dependencies, resource/safety controls and the full
Unix cargo test BUILD unchanged. This repairs the observed Windows build-plan
selection failure; Windows reactor runtime support is not claimed. Hosted CI
acceptance remains pending. Miri DEFERRED/PENDING, never passed.
