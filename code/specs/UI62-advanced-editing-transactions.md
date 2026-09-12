# UI62: Advanced Editing Transactions

## Status

Implemented by `browser-form-controls` and `venture-browser-core`; native host
adapters translate platform key and accessibility gestures into this contract.

## Selection Contract

Selection offsets remain Unicode-scalar indexes at the public boundary so they
are deterministic across FFI hosts. Character movement and forward/backward
deletion use the generated Unicode 17 UAX #29 grapheme segmentation from
`text-flow`, and therefore never stop inside a user-perceived character. Word
movement and double-click selection share one Unicode-aware letter, numeric,
whitespace, and punctuation policy. Triple click selects the containing line.

Pointer drags retain the initial anchor. When a drag leaves an editable
viewport, the shared reducer advances horizontal and multiline vertical scroll
offsets by a bounded distance derived from explicit text metrics. Toolkits do
not calculate selection or autoscroll policy.

## Transaction Contract

Each editable control owns independent undo and redo stacks. A value mutation
records value and selection together, a new mutation clears redo, reset clears
both stacks, and each stack retains at most 100 snapshots. Composition commit,
cut, paste, text input, accessibility replacement, and grapheme deletion all
enter the same transaction path. Disabled, read-only, and password clipboard
rules remain authoritative in the shared model.

Clipboard exchange uses `ControlClipboardPayload`. Copy supplies both escaped
`text/plain` and `text/html` representations; paste prefers plain text and
reduces an HTML-only fragment to non-executable text. Platform accessibility
adapters use `ControlAccessibilityAction` for movement by grapheme, word, line,
or document, explicit selection/replacement, select-all, undo, and redo.

## Determinism

Tests use fixed Unicode strings, click coordinates, viewport metrics, and
transaction counts. Core acceptance proves the same action contract reflows a
retained page, while native host command routing uses the shared semantic key
names (`word-left`, `word-right`, `select-all`, `undo`, and `redo`).
