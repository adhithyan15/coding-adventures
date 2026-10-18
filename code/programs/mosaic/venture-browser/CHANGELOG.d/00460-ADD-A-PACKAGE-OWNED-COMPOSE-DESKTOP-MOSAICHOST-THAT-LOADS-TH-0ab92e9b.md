- Add a package-owned Compose Desktop `MosaicHost` that loads the shared Rust
  browser session through JNA, mounts Cairo-rendered RGBA pixels as a native
  Compose `Image`, and forwards native wheel, hover, and pointer activation.
