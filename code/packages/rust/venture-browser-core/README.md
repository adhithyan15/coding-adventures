# venture-browser-core

The host-neutral orchestration layer for Venture, the educational Mosaic-era
browser described by BR01.

```text
requested URL
  -> BrowserResourceFetcher
  -> final fetched URL + HTML bytes
  -> html-parser
  -> ordered stylesheet plans + computed author/UA cascade
  -> html-to-layout -> html-to-paint
  -> ordered BrowserSubresourceRequest effects
  -> incremental CSS/GIF/JPEG completions and repaint
  -> BrowserPage { document, source, links, controls, scene, stylesheet_resources, image_resources }
```

`BrowserPagePipeline::load` uses the final fetched document URL as the base for
relative links, stylesheets, and images. `begin_execute` commits the retained
document before subresources and emits typed requests in deterministic
stylesheet-then-image order. Hosts dispatch those
effects through `BrowserSubresourceScheduler`; each completion recomposes only
from retained document/resource state, reports whether repaint is required,
and exposes newly discovered import requests for the same scheduler.
Navigation emits cancellation effects, and generation IDs make late or
duplicate completions harmless. Stylesheets remain blocked in parser-defined
document order even when completions arrive out of order; failed or
media-inactive sheets unblock later author rules without discarding the
retained document. Imports use append-only stable request ordinals while the
cascade walks them depth-first before their parent; ancestor cycles become
diagnostics rather than fetch loops. Link, import, and rule media all evaluate
against the pipeline's logical viewport. Image failures remain recoverable Mosaic-style bordered
`alt` text.

The default `HttpBrowserFetcher` adapts `http1-client`, but tests and platform
hosts can inject any transport. Font measurement, shaping, metrics, resolution,
and the final paint backend also remain caller-owned.

`BrowserSession` routes submit/reset activation and implicit Enter submission
through `browser-form-submission`. Invalid forms retain bounded diagnostics;
valid GET and URL-encoded POST plans use the same transactional load, history,
visited-link, control-state, and subresource lifecycle as ordinary navigation.

The session also owns editor presentation. It appends stable backend-neutral
overlay groups for selection, caret, composition, and invalid feedback after
every retained reflow; routes pointer placement and drag selection; exposes
explicit copy/cut/paste methods; advances caret blink from deterministic host
ticks; and returns an IME candidate rectangle without handing editing state to
a native toolkit. It also routes grapheme/word movement, click-count selection,
drag autoscroll, bounded undo/redo, plain/HTML clipboard negotiation, and typed
accessibility actions through the same retained reflow. Password selection
never crosses the clipboard boundary.

Contenteditable hosts now reuse that semantic input and presentation seam while
remaining outside form-control ownership. `ContentEditableModel` retains a
bounded value, selection, composition, undo/redo history, accessibility state,
and per-navigation-entry snapshot; accepted mutations synchronize the retained
render tree before shared reflow. Plaintext and rich-text modes stay explicit,
HTML-only clipboard input is reduced to bounded text, and every host continues
to forward the same keys, pointer coordinates, clipboard flavors, ticks, and
IME updates without owning DOM mutation or range policy.

Typed inputs retain that single-owner design. The session exposes
`ControlValueState` for live validity and accessibility projection, routes
Arrow Up/Down and SetValue/Increment/Decrement through shared numeric
min/max/step policy, and reflows after each accepted mutation. Native and web
hosts do not parse values, enforce `maxlength`, or duplicate selection rules.
Date, month, ISO week, time, datetime-local, and color inputs extend that same
contract with normalized accessibility value text and scalar stepping. Session
reflow follows every accepted semantic key or accessibility value action;
platform adapters remain translators rather than owners of picker state.
File controls follow the same boundary: activation yields a path-free picker
request, selected metadata and bounded bytes return through the shared model,
retained reflow sees only sanitized names, and multipart navigation is planned
before the common transport receives a request.
Image submitters pass control-local pointer coordinates into that planner;
keyboard and accessibility activation use deterministic zero coordinates.
The session also preserves live Unicode `dirname` direction fields beside
their owning successful controls without exposing this policy to hosts.
Form-associated custom elements use the same boundary: hosts attach retained
internals, set bounded submission/restoration values and validity, drain typed
association/disabled/reset/restore callbacks, and consume reusable label and
accessibility projections. Native and web adapters receive one shared API;
they do not serialize values or infer lifecycle policy.
Native activation and scripted form APIs also converge before transport.
`BrowserSession` exposes `requestSubmit`, cancelable submit/reset dispatch,
non-interactive `checkValidity`, interactive `reportValidity`, and mutable
`formdata` entries. It records the resulting lifecycle events and commits
navigation only after every synchronous handler returns, so generated hosts do
not own cancellation, event ordering, diagnostics, or serialization policy.
Disabled-fieldset inheritance and explicit/implicit label activation also stay
inside this boundary; host adapters forward page coordinates without deriving
form ownership, effective disabledness, or activation behavior.
Form values now survive Back/Forward through bounded session-owned snapshots.
Those snapshots are keyed by stable history-entry identity rather than URL, so
repeated visits restore independent public form and custom-element state. The
same bounded entry snapshot restores the shared logical scroll offset before
native/web hosts repaint, including fetch-free fragment traversal.
The session exposes dirty/default state, grouped autofill descriptors, explicit
public-versus-credential transactions, and ordered mutation events. Passwords
are excluded from public snapshots and autofill, file payloads are never
persisted, and custom-element restoration waits for internals attachment.

`NavigationHistory` is re-exported from the reusable `browser-navigation`
package and implements the BR01 in-memory navigation model: navigate, Back,
Forward, Home, Reload, and redirect replacement. The same package owns
`VisitedLinks`, whose canonical document identity normalizes schemes, hosts,
default ports, percent escapes, dot segments, and fragments independently of
the page pipeline.

`BrowserSession` also owns same-document fragment traversal. It strips
fragments before transport, percent-decodes target identifiers, resolves both
`id` and legacy anchor `name` metadata in document order, clamps target
geometry through the shared viewport, and publishes `FragmentNavigationState`
for diagnostics. Navigate, Back, and Forward update history and scrolling
without refetching or replacing retained controls and document state.

Bookmarks are re-exported from the storage-neutral `browser-bookmarks`
package. `BrowserSession` owns the active catalog and applies add/remove
commands with save-before-commit transaction semantics, while native hosts
inject either the durable `browser-bookmarks-file` adapter or an isolated
in-memory repository for tests. Bookmark URL identity deliberately retains
fragments so separate document anchors can be saved independently.

`ScrollState` clamps vertical offsets against content and viewport geometry,
performs scroll-aware link hit-testing, and feeds `scrolled_viewport_scene`.
That function preserves the document scene beneath a group translated by the
negative scroll offset and sizes the returned scene to the visible viewport.

`BrowserViewport` binds a loaded `BrowserPage` to that scroll policy. Native
content-area hosts can resize or scroll it, hit-test links in viewport
coordinates, and request the exact viewport scene for each paint event. Page
replacement resets scroll while preserving the current viewport height.

Client-side image maps enter that same link path. The session exposes stable
area accessibility keys and names, while pointer or semantic activation reuses
the shared current/auxiliary/download planner. Native hosts never parse
coordinates or choose overlap and target policy.

Sequential focus is also session-owned. Rendered links, enabled controls,
details summaries, image-map areas, generic authored `tabindex`, and
contenteditable surfaces form one ordered list: positive values lead, ordinary
targets follow document order, negative and blocked targets are excluded, and
modal top layers contain traversal. The session exposes reusable roles, names,
and accessibility geometry, scrolls focused content into view, paints shared
non-control focus rings, and routes supported Enter/Space activation through
existing transactions.

`BrowserSession` is the reducer a native shell keeps for browser behavior. It
dispatches Navigate, Back, Forward, Home, and Reload through the page pipeline,
replaces redirect history with the final URL, and updates the viewport only
after a successful load. Pointer activation uses viewport coordinates and
follows the resolved link through the same transactional path. Successful
final URLs are committed to session visited state only after full composition;
failed loads preserve history, viewport, and visited state together. The
legacy `execute`/`load` methods synchronously drain the same lifecycle for
compatibility; native and web event loops use `begin_execute`, scheduler
effects, and `complete_subresource`. Reflow,
reload, Back, and Forward all project the retained state into blue/purple link
styling without coupling browser history to HTML layout.

`BrowserControlModel` owns focus, values, checked/radio state, select indexes,
character-indexed selection/caret state, composition text, validation feedback,
and disabled/read-only policy. Pointer, keyboard, text, and IME input
synchronize the retained render tree and reflow through the same
backend-neutral page pipeline. Failed validation focuses the first invalid
control and projects `aria-invalid` plus accessible diagnostics, so native
surfaces never instantiate toolkit-specific controls or own editing policy.

`BrowserChromeController` is the matching host-neutral reducer for the shared
Mosaic `VentureChrome` package. It preserves address edits as a draft, maps
navigation, bookmark, and View Source events to host-neutral commands, synchronizes redirects
only after a successful load, and projects one coherent `BrowserChromeProps`
snapshot for the nine MIL slots. `BrowserAuxiliaryDocument::view_source`
escapes the already-retained response text into synthetic preformatted HTML,
and `BrowserHostEventOutcome` carries the resulting platform-owned window
effect without navigation, history mutation, or a network fetch. Generated Mosaic shells expose the native
node seam on all registered backends. The SwiftUI adapter connects this reducer
to the live Metal renderer, WinUI mounts Direct2D pixels, and Qt, Flutter, and
Compose share the Cairo bridge. Platform-native integration gates exercise the
generated shells through those adapters rather than reimplementing browser
state in each toolkit.

## Development

```bash
cargo test -p venture-browser-core -- --nocapture
```
