### Added -- the Compose host answers effects (UI47 §5.4 step 4)

The third of five host templates, after Qt and SwiftUI. `MosaicRuntimeHost`
gains `effectHandler`, `completeEffect(id, result)`, `deferEffect(id)` and the
same bounded settle loop; the JNA interface gains
`mosaic_app_complete_effect`, and the emitted protocol version moves to
`EFFECT_PROTOCOL_VERSION` alongside Qt and SwiftUI.

Unlike SwiftUI, the deferred answer is **not** hopped to a particular thread.
SwiftUI requires state mutation on the main thread; Compose writes
`mutableStateOf` through the snapshot system, which accepts writes from any
thread, so a hop here would impose a rule Compose does not have.

Three defects this found, none of which the crate's text assertions could see:

- **Three missing `kotlinx.serialization` imports.** The emitted host did not
  compile at all. Every existing test asserts on the *text* of the emission and
  passed; the first `kotlinc` invocation failed. Hence the new acceptance below.
- **A handler that throws wedged persistence permanently.** The handler runs
  inside the settle loop, so an escaping exception left the id in `awaiting`
  with nothing left to discharge it -- and the runtime refuses to `snapshot` or
  `restore` while anything is pending. It is not an exotic path:
  `toJsonElement` throws on any value it has no case for, which is what a
  handler returning the `File` a dialog gave it does on its first run. A
  throwing handler is now treated as one that did not answer, so the sweep
  still fails the effect, and the app is told which handler failed and why
  rather than being handed a bare "no host handler answered".
- **Malformed effect entries threw rather than being reported.**
  `JsonElement.jsonObject` and `.jsonArray` throw on a wrong-typed element, and
  two of the three call sites were in `failOutstanding` -- the recovery path,
  where a throw aborts the very sweep that prevents the wedge. Parsing is now
  total, matching Qt and SwiftUI, which already were. This path is defensive:
  the runtime only ever emits well-formed effects, so it is **not** exercised
  by the acceptance below.

