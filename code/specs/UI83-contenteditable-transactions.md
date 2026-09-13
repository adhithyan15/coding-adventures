# UI83: Shared contenteditable transactions

## Status

Implemented for Venture's host-neutral browser session.

## Problem

The parser retained `contenteditable`, editing mode, selection, clipboard, and
input-handler descriptors, while UI82 only made editing hosts focusable. Hosts
could focus those surfaces but could not edit them without inventing a second
toolkit-owned text model.

## Contract

`ContentEditableModel` owns editing hosts separately from
`BrowserControlModel`. An editing host is not a successful form control and its
value never enters form serialization, validation, autofill, or credential
state. The model retains stable focus keys, plaintext/rich-text mode, bounded
text, character selection, pointer anchor, composition, 100-entry undo/redo,
deterministic editor geometry, and per-history-entry snapshots.

Unicode grapheme movement and deletion reuse `text-flow`. Overlay geometry
reuses `browser-form-controls::text_editor_presentation`, so controls and
arbitrary editable DOM share caret, scrolling, selection, and IME policy
without sharing form ownership.

Each host is limited to 256 KiB; a document retains at most 512 editing hosts
and 1 MiB of initial editable text. Diagnostics describe truncation or
omission. Rich-text hosts retain their mode for accessibility and future DOM
range work; this bounded phase commits textual mutations and sanitizes
HTML-only clipboard payloads to text rather than accepting host-created markup.

## Host mediation

Existing semantic `control_*` session methods dispatch to whichever shared
editor owns focus. Generated SwiftUI, WinUI, Qt, Flutter, Compose, and DOM
bridges remain unchanged: hosts forward key names, text, pointer coordinates,
clipboard flavors, elapsed blink time, and IME text, then paint the shared
scene. They do not choose an editing host, mutate DOM, calculate ranges, own
undo, or place IME candidates.

## Acceptance

Core acceptance covers mixed form/contenteditable pages, sequential focus,
text replacement, sanitized HTML clipboard paste, cut/copy, composition and
candidate geometry, undo, retained reflow, form isolation, and history-entry
restoration. Standalone reducer tests exercise bounds and shared presentation.
