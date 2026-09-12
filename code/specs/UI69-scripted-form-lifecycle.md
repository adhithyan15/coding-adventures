# UI69: Scripted Form Lifecycle Dispatch

## Status

Implemented for Venture's shared native/web form pipeline.

## Contract

Form lifecycle policy belongs to `browser-form-submission`, not a generated
toolkit. A dispatch transaction resolves the associated form and optional
submitter, runs shared constraint validation, emits synchronous lifecycle
events, exposes ordered mutable form data, validates its final bounds, and only
then constructs a navigation request.

The public event order is:

1. one cancelable `invalid` event per failing native or attached custom control;
2. a cancelable `submit` event when validation succeeds or is bypassed;
3. a mutable, non-cancelable `formdata` event after successful controls are
   collected in document order;
4. navigation after the final form-data snapshot is revalidated and encoded.

Reset dispatch emits one cancelable `reset` event before restoring shared
native and custom state. Preventing submit or reset returns a no-op activation;
preventing invalid suppresses interactive presentation but never makes an
invalid form valid.

`requestSubmit` accepts a form ordinal and optional submitter key. The
submitter must be an enabled submit/image control associated with that form;
its action, method, encoding, validation override, name/value, and deterministic
keyboard image coordinates remain authoritative. Omitting it does not invent a
default submitter. Implicit Enter and pointer/accessibility activation enter the
same transaction through their existing submitter rules.

`checkValidity` and `reportValidity` share diagnostics and cancelable invalid
events. Check mode has no presentation side effect. Report and submit modes let
`BrowserSession` focus and annotate only diagnostics whose invalid event was
not prevented, while navigation remains blocked by every validity failure.

## Bounds And Hosts

Form-data listeners may reorder, replace, delete, or append text and opaque
file values. The final entry count and URL-encoded or multipart payload limits
are checked after listeners return. Host paths never enter the data model.

`BrowserSession` retains emitted events for deterministic draining and commits
transport/history only after dispatch completes. SwiftUI, WinUI, Qt, Flutter,
Compose, HTML, Web Component, React, and Electron adapters therefore translate
input or script calls without owning event order, cancellation, validation,
serialization, or navigation policy.
