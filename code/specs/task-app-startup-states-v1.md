# TaskApp startup states v1

Issue: [#13695](https://github.com/adhithyan15/coding-adventures/issues/13695)
Parent: [#13523](https://github.com/adhithyan15/coding-adventures/issues/13523)

## The defect

The web host's `boot()` awaited four things — the WASM fetch, `WebAssembly.compile`,
the storage open, and the workspace restore — before calling `createRoot().render()`.
Until all four resolved, `#root` was empty. The call site was `void boot()`, so the
promise was floated: any rejection became an unhandled rejection and the page stayed
empty permanently, with the error visible only in the browser console.

A 404 made this worse rather than better. `fetch` resolves for a 404, and
`response.arrayBuffer()` happily returns the error page's bytes, so a missing engine
reached `WebAssembly.compile` and surfaced as a `CompileError` — which reads as a
corrupt engine rather than a missing file.

## Why startup states cannot be authored in Mosaic

The emitted TaskApp component is presentational: it takes slot values computed by
the controller, and the controller needs a live engine. While the engine is still
being fetched and compiled there is, by construction, no authored component to
render. Startup states are therefore *host chrome*, not app UI, and each host draws
its own.

This is the single deliberate exception to "never style the app outside mosstyle"
(`index.html` makes the same point about CSS). It is kept as small as possible: the
web host reads four values — ground, text, alert, font-family — verbatim from
`app-shell` and `storage-warning` in `TaskApp.{light,dark}.msl`, so the startup
states are visually continuous with the app that replaces them.

## Required behavior

These apply to **every** host, web and native.

1. **Never present an empty surface.** From the moment the host owns its window or
   root, something is displayed. A loading state is shown while the engine and the
   saved workspace initialize.
2. **A failed start is reported, not swallowed.** Initialization failure produces a
   visible message that says the app could not start and shows the underlying
   detail. It is never left to a log, a console, or a process exit code.
3. **A failed start is recoverable in place.** The failure state offers a retry that
   re-runs initialization without a manual reload or relaunch. Retry is offered
   because every failure reachable here is plausibly transient — an interrupted
   download, a cold cache, a flaky network.
4. **A failed start does not imply data loss.** Nothing has been written at this
   point, and the message says so. This is a product requirement, not a nicety:
   without it a transient network error reads as a lost workspace.
5. **Error detail is rendered as text, never as markup or a format string.** It can
   carry bytes the host did not author — a URL, a server's response text.
6. **The task-name focus handoff stays deterministic.** Once initialization
   succeeds, the app mounts and takes focus exactly as it does today; the startup
   states must not leave focus parked on their own controls.

## Web host implementation

`src/startup.tsx` holds the two states; `boot()` in `src/main.tsx` sequences them:

- resolve the theme and paint the ground, then render the loading state
  **before** the first `await`;
- run initialization inside `try`/`catch`;
- on success, replace the loading state with the app;
- on failure, render the failure state with the error detail and a retry that
  calls `boot()` again.

One React root is created for the document and reused, so retry re-renders rather
than calling `createRoot` twice on a container React already owns.

`response.ok` is now checked explicitly, so a 404 reports its status instead of
being laundered into a `CompileError`.

### Coverage

`__tests__/startup.test.tsx` covers the states themselves — what they say, their
live-region roles, that the detail is escaped, and that both themes match the
authored shell colours. `__tests__/boot.test.tsx` covers the sequencing that was
actually broken: that loading paints before initialization resolves, that a 404
and a network failure each replace the blank page with a recoverable failure, and
that retry re-runs initialization rather than only re-rendering.

The success path is not reachable from these tests — it needs a real compiled
`task_engine.wasm` — and remains covered by the presentation contract and the
live browser build.

## Native hosts

Generated native hosts do not share this code; they share the requirement. Each
already has a window before `task-mosaic-app` is ready, so requirement 1 is about
what fills that window, and requirements 2–4 are about what replaces it when the
adapter or its snapshot fails to load.

Today native hosts report a startup failure only through process and log evidence.
Closing that is **not** in this change: it needs a per-backend surface in Qt,
Flutter, Compose, SwiftUI, and WinUI, and belongs with the emitted-control contract
work rather than inside a web-host fix. The requirements above are written
host-neutral so that work has a contract to implement against rather than
re-deriving one, and it is tracked in
[#13984](https://github.com/adhithyan15/coding-adventures/issues/13984).
