### Added — package handlers can answer their own kinds through the Kotlin picker (UI89 §3.11)

- `mosaicPlatformRouter(host)` returns the router installed as the host's
  handler, or null.
- `MosaicPlatformRouter.openForApp(id, accept, limit, ok)` and
  `saveForApp(id, suggestedName, bytes, accept, ok)` answer an app's own Await
  through the library's picker under `files.*`'s rules:
  - one file operation at a time;
  - deferred before anything is shown;
  - slow work on the background thread, handed back to the UI thread;
  - exactly one answer, with `failPending` still reaching an app request in
    flight;
  - the same name checks, now `mosaicCheckSaveName`, which compares
    extensions without case;
  - an executable extension refused for an app save whatever the app
    accepts.
- `files.open` and `files.save` now run on the same `mosaicAnswerOpen` and
  `mosaicAnswerSave`. Their behaviour is unchanged.
- A refusal never throws into the caller. The same in-flight id asked for
  twice is left to its own picker (ported to the Swift library too, with a
  harness check). A throw inside the posted operation still answers and
  frees the router.
- The Compose JVM harness gained three tests of the app path (33 tests).
