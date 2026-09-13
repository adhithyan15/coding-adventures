# UI77: Shared Fragment Navigation

## Goal

Venture owns same-document URL fragments in its host-neutral session rather
than asking native or web shells to fetch, locate, or scroll anchor targets.

## Contract

- Document transport receives the canonical URL without its fragment.
- Successful full navigation reattaches the requested fragment to history and
  resolves it only after shared layout is available.
- Navigate, Back, and Forward between URLs that differ only by fragment keep
  the retained document, controls, resources, and navigation generation.
- Targets match the first positioned element in document order whose `id` or
  legacy `a[name]` equals the percent-decoded fragment.
- Empty fragments scroll to the document start. Missing nonempty targets retain
  the current offset and report `target_found = false`.
- Shared scroll bounds clamp resolved target geometry. Hosts forward navigation
  commands and render the resulting viewport; they do not own target policy.

## Diagnostics And Acceptance

`FragmentNavigationState` publishes the committed URL, decoded fragment,
resolution status, and actual clamped scroll offset. Parser, Layout IR, and
session tests cover named-anchor preservation, fragment-free fetches, decoded
IDs, missing targets, empty fragments, and fetch-free history traversal.
