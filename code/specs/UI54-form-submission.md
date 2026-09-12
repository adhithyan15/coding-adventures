# UI54: Form Submission and Validation

## Status

Accepted for Venture's reusable browser stack.

## Ownership

`browser-form-controls` owns retained visible control state and initial reset
baselines. `browser-form-submission` owns form association, constraint
validation, successful-control collection, and request planning. The Venture
session owns transactional navigation and diagnostics. Network and UI hosts
remain replaceable adapters.

## Activation

- Pointer or keyboard activation of a submit control validates its associated
  form unless the form or submitter disables validation.
- Enter in a single-line text control activates the first enabled submitter in
  the associated form, or submits without a submitter when none exists.
- Reset restores initial values, checked states, and selected options, then
  runs the ordinary retained-document reflow without navigation.

## Successful Controls

Controls are serialized in document order. Disabled and unnamed controls are
excluded. Checkboxes and radios contribute only when checked; select controls
contribute selected option values; only the activated submitter contributes
its name and value. Text line endings are normalized before encoding.

## Validation

The first 32 failures are retained as typed diagnostics. Required controls,
radio groups, email and URL syntax, length, pattern, finite numeric min/max,
and step alignment are checked before request planning. Typed value checks
consume the same `ControlValueState` used by live editing and accessibility;
invalid forms do not mutate history or fetch.

## Navigation

GET places UTF-8 `application/x-www-form-urlencoded` data in the action query.
POST sends the same encoding as a bounded body with explicit content type and
byte length. Both use the existing transactional page pipeline: failed loads
leave page and history untouched, while successful final URLs replace the new
history entry after redirects.

## Bounds

Planning accepts at most 1,024 entries, a 1 MiB encoded payload, and 32
diagnostics. Unsupported methods or POST encodings fail with structured errors
instead of silently changing semantics.
