- Add a package-owned Flutter `MosaicHost` that loads the shared Rust browser
  session through Dart FFI, mounts Cairo-rendered RGBA pixels as a native
  Flutter `RawImage`, and forwards wheel, hover, and tap input without
  duplicating the Mosaic-authored chrome.
