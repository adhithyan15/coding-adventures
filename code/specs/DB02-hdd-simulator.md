# DB02 — Deterministic HDD simulator

Parent: DB00; backlog: DB-005. All native lanes implement the same contract.

## Model

Start with a deliberately idealized disk: one surface, fixed sectors per track,
fixed sector size, and integer simulation ticks per sector. State includes sector
bytes, current track, current time and spindle phase. Inputs have explicit caps,
checked arithmetic and bounds before allocation. It is a teaching model, not a
calibrated prediction for a vendor drive. Real LBA mapping need not expose CHS.

For a single-sector request to LBA `l`:

1. Track = floor(l / sectors_per_track); sector = l mod sectors_per_track.
2. Seek ticks = absolute track distance times configured ticks per track.
3. Advance spindle phase by seek ticks modulo revolution ticks.
4. Rotational wait = (sector start phase - current phase) modulo revolution.
5. Transfer takes ticks_per_sector; advance time and phase through that transfer.
6. Read returns exactly the selected sector. Write replaces it without an erase.

The initial profile serializes requests with no cache and returns an event
breakdown: seek, rotational wait, transfer, start and completion tick. Invalid
requests leave both timing and data unchanged. Arithmetic overflow is an error.
Callers control idle time explicitly; no wall clock or sleeping is used.

## Acceptance

- Exact hand-derived fixtures for same-track access, seek while platter rotates,
  wraparound, repeated-sector reads and sequential cross-track access.
- Exhaustive small-geometry/phase/address transition tests with a separate
  angular-position oracle; bounded history tests and seeded random workloads.
- Data overwrite, adjacent-sector preservation, returned-buffer isolation,
  invalid size/address/configuration rejection and time monotonicity.
- Mutation controls for ignoring rotation during seek, forgetting modulo,
  advancing phase incorrectly and requiring flash-style erasure.
- Equal event traces and final bytes in every native language.

## Later profiles

Multiple heads, track/head switch time, zoned capacity, nonlinear seek profiles,
request scheduling, write caches, flush ordering, retries, bad sectors and torn
writes are separate additions. Baseline model memory is not a durability claim.
The shared block-device fault model supplies volatile/stable bytes and crashes.
