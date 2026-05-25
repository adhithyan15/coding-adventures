// Grid.desktop.mll — desktop layout for the spreadsheet grid.
//
// UI31-L10 migration: rewritten from the legacy built-in `Grid`
// primitive (which was wired only in the React emitter) to the UI31
// HostTable kernel family. Every backend (React, HTML, WebComponent,
// Flutter, Qt, SwiftUI, XAML) lowers HostTable* to its native,
// accessibility-aware table widget per #4143, #4156, #4162, #4166,
// #4185, #4194, #4198.
//
// What this migration gains
// -------------------------
//
//   - Cross-backend a11y semantics. Before: only React had a real
//     `<table>` (the other 6 backends either errored on the `Grid`
//     primitive or emitted div-soup). After: every backend emits a
//     real native table widget — `<table>`/`<thead>`/`<tbody>`/`<tr>`/
//     `<td>` on web, `DataTable` on Flutter, `VStack(.leading)`+
//     `HStack` on SwiftUI, `ColumnLayout`+`RowLayout` on Qt, `<Grid>`+
//     `<Grid.RowDefinitions>` on XAML.
//   - UI31 §3.2 RTL contract on every backend (the `dir` slot is
//     unbound here today but the contract is wired through HostTable
//     end-to-end; a future Grid.mil revision can expose it as a slot).
//
// What this migration loses (deliberately, per the user's "degraded
// migration accepts loss" decision)
// --------------------------------
//
//   - `sticky-header: true/false` — the legacy `Grid` primitive's
//     sticky-header behaviour (which kept the column-header row
//     visible while the body scrolls) does not survive. The header
//     row simply scrolls away with the body. On desktop this is a
//     minor regression visible only on large grids.
//   - `selected-row` / `selected-col` highlight — the cell that the
//     user has selected is no longer styled differently. Selection is
//     still tracked in the host's reducer (the slots still exist on
//     the interface) but the visual indicator is dropped.
//   - `edit-row` / `edit-col` inline editing — there is no `<input>`
//     rendered in place of the cell text when editing. Editing must
//     happen via the FormulaBar.
//   - `column-widths` — per-column pixel widths are dropped. Columns
//     auto-size based on content.
//   - `total-height` — there is no scroll viewport with a fixed
//     `maxHeight`. The grid grows to fit its rows.
//   - `onNavigate` cell-click emit — the legacy Grid wired clicks on
//     each `<td>` to dispatch `onNavigate(row,col)`. HostTable does
//     not have a per-cell click hook (the kernel hasn't promoted one
//     yet); reintroducing it needs a kernel primitive for cell-click
//     or a userland Cell component.
//
// These losses can be re-acquired by either:
//   (a) extending the IR/grammar/emitters with per-cell decorations
//       (sticky-header attribute, selected-cell highlight, click-
//       binding) — a multi-PR effort across 7 emitters; or
//   (b) building a richer `Cell` component in mosaic-pkg-grid that
//       takes the row/col/selection/edit slots and emits the right
//       per-cell markup. Same pattern as mosaic-pkg-grid v0.1.0
//       already prototypes.
//
// Both are out of scope for L10 (the user's directive was "degraded
// migration accepting loss").
//
// Layout tree
// -----------
//
//   HostTable [sheet]                    ← native <table> / DataTable /
//                                          ColumnLayout / VStack / <Grid>
//     HostTableHead                      ← native <thead>
//       Row                              ← native <tr>
//         For ( each: column-headers,    ← one <th> per header text
//               as: header )
//           Text ( content: header )
//     HostTableBody                      ← native <tbody>
//       For ( each: viewport-rows,       ← one <tr> per data row
//             as: row )
//         Row {
//           Text ( content: row )        ← v1: one cell per row showing
//                                          the row's stringified form.
//                                          Full per-column iteration
//                                          (nested For) needs the
//                                          row-binding-as-iterable
//                                          grammar extension or a
//                                          Cell component (matches
//                                          mosaic-pkg-grid v0.1.0).
//         }
//
// The interface (Grid.mil) is unchanged. The slots that this
// degraded version doesn't consume (selected-row, edit-row, etc.)
// remain in the interface so the host can keep pushing them; they
// just don't influence the rendered output.

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
