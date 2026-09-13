# UI78: Per-Entry History State Restoration

## Goal

Venture restores browser-owned state for the exact session-history entry being
traversed, even when multiple entries have the same URL. Native and web hosts
only repaint the shared result.

## Contract

- Every fresh navigation receives a stable session-local entry identifier.
- Back and Forward preserve entry identifiers; redirect replacement changes the
  URL without changing identity.
- The session stores bounded public control/custom-element state and the shared
  logical scroll offset against the departing entry identifier.
- Back and Forward restore the target entry before repaint and clamp its offset
  to current layout geometry.
- Repeated URLs remain independent. Passwords and file payloads retain the
  existing public-snapshot privacy boundary.
- Same-document fragment traversal remains fetch-free and restores the entry's
  prior offset rather than rerunning anchor scrolling.

## Diagnostics And Acceptance

`BrowserHistoryRestorationState` publishes the restored entry identifier,
state categories, and resulting offset. Navigation tests ratchet stable IDs
through duplicate URLs and redirects. Session acceptance covers distinct form
values and offsets for repeated URLs, while fragment acceptance proves exact
fetch-free Back/Forward restoration.
