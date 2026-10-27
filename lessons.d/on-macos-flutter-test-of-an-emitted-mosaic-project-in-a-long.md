---
category: Mosaic compiler pipeline
---

# On macOS, flutter test of an emitted Mosaic project in a long path needs a runtime dylib linked with -headerpad_max_install_names

Running `flutter test` on Journal's emitted Flutter project from the session
scratchpad (a path of about 150 characters) failed before any test ran:
`install_name_tool: changing install names or rpaths can't be redone ...
because larger updated load commands do not fit`. Flutter's native-assets
hook rewrites the bundled `libmosaic_app.dylib`'s install name to its own
long build path, and a dylib linked by default has no spare header room for
it.

**Fix:** build the runtime with header padding, used only for the local
test:

```bash
RUSTFLAGS="-C link-arg=-Wl,-headerpad_max_install_names" \
  cargo build -p journal-mosaic-app
```

Then pass that dylib to `mosaic-compile ... --runtime-library`. Using a
separate `CARGO_TARGET_DIR` keeps the different `RUSTFLAGS` from
invalidating the normal build. A shorter output path also works.
