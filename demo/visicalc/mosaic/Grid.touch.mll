// Grid.touch.mll — touch / mobile layout for the spreadsheet grid (UI30).
//
// UI31-L10 migration: rewritten from the legacy built-in `Grid`
// primitive to the UI31 HostTable kernel family. See
// `Grid.desktop.mll`'s top comment for the full migration rationale
// and the list of features the degraded migration drops.
//
// Why a separate touch variant at all?
// ------------------------------------
// Pre-migration, the desktop variant pinned the header row via
// `sticky-header: true` and the touch variant explicitly set
// `sticky-header: false` (so the header scrolls away on small
// screens, returning the full viewport to the user as they scroll).
// Post-migration, sticky-header is dropped from both variants (it
// belonged to the legacy `Grid` primitive's emitter, not to
// HostTable* yet), so the desktop and touch trees are byte-for-byte
// identical today.
//
// We keep this separate variant file rather than deleting it because:
//
//   1. The UI30 multi-layout convention treats per-variant `.mll`
//      files as a stable extension surface. Future work that
//      reintroduces sticky-header via a HostTable extension, or that
//      adds touch-specific cell sizing (≥44 px tap-targets per Apple
//      HIG), or that picks a smaller row-height for narrow phones,
//      slots in here without changing the build pipeline.
//   2. Deleting the file would re-merge desktop/touch into one
//      variant, which the artifact-builder discovers by absence —
//      tools wired to expect `Grid.touch.<ext>` artifacts would
//      silently fall back to `Grid.<ext>` (the variant-resolution
//      back-compat path). That's a subtle behaviour change worth
//      avoiding.
//
// What this variant changes vs. .desktop.mll today: NOTHING. Both
// emit the same HostTable tree. The diff lives in the comments only.

layout Grid {
  HostTable [sheet] {
    HostTableHead {
      Row {
        For ( each: slot: column-headers , as: header ) {
          Text ( content: header )
        }
      }
    }
    HostTableBody {
      For ( each: slot: viewport-rows , as: row ) {
        Row {
          Text ( content: row )
        }
      }
    }
  }
}
