// Grid.mll — Layout definition for the Grid component.
//
// UI31-L10 migration: this layout now composes from the UI31 HostTable
// kernel family (HostTable + HostTableHead/Body + Row + Text) instead
// of the legacy built-in `Grid` primitive. The motivation is the UI31
// non-negotiable accessibility + RTL contracts that the HostTable
// family ships across all seven backends (React, HTML, WebComponent,
// Flutter, Qt, SwiftUI, XAML) per #4143, #4156, #4162, #4166, #4185,
// #4194, #4198 — semantics that the built-in `Grid` primitive only
// delivered on React.
//
// Layout tree:
//
//   Column [ root ]                          ← flex column container
//     HostTable [ cell-grid ]                ← native <table> on web,
//                                             SwiftUI VStack/HStack,
//                                             QtQuick ColumnLayout,
//                                             Flutter DataTable,
//                                             WinUI Grid on XAML.
//       HostTableHead                        ← native <thead>
//         Row                                ← native <tr>
//           For ( each: column-headers,
//                 as: header )               ← one <th> per header text
//             Text ( content: header )
//       HostTableBody                        ← native <tbody>
//         For ( each: viewport-rows,
//               as: row )                    ← one <tr> per data row
//           Row {
//             Text ( content: row )
//           }
//
// Why HostTable* over the built-in `Grid` primitive?
// --------------------------------------------------
// The built-in `Grid` primitive was a React-only special case: it
// emitted `<table><thead><tbody>` directly, but every other backend
// either errored or fell back to a div-soup placeholder. HostTable*
// makes each backend lower to its native, accessibility-aware table
// widget:
//
//   - HTML / React / WebComponent → real `<table>` (with `<thead>`,
//     `<tbody>`, `<tr>`, `<td>`) — screen readers walk row/col
//     associations automatically; ARIA `aria-rowindex` and
//     `aria-colindex` are intrinsic to the elements.
//   - Flutter → `DataTable` widget — TalkBack / VoiceOver see real
//     table semantics, not `Container` mush.
//   - SwiftUI → `VStack(.leading, spacing: 0)` of `HStack` rows that
//     SwiftUI's accessibility framework groups as a coherent table.
//   - Qt → `ColumnLayout` of `RowLayout`s — Qt Accessibility recognises
//     the structural shape.
//   - XAML → `<Grid>` with `<Grid.RowDefinitions>` per section — WinUI's
//     automation peer derives row/col semantics from `Grid.Row="..."`.
//
// All seven backends ALSO honour the UI31 §3.2 RTL contract — Grid's
// interface would need a `dir` slot for authors to actually flip the
// direction (out of scope here — would extend Grid.mil), but the
// contract is preserved end-to-end so a future Grid v2 can opt in
// without touching emitters.
//
// v1 caveats (intentional — matched to Grid.mil's slot shape)
// -----------------------------------------------------------
// Grid.mil declares `column-headers : list<text>` and `viewport-rows :
// list<text>` — both flat lists of text. So each `<tr>` in the body
// carries one `<td>` containing the row's text. A v2 Grid extending
// viewport-rows to `list<list<text>>` would let the inner For iterate
// columns; that is a separate change (matches the v0.1.0 caveat in
// `packages/mosaic/mosaic-pkg-grid/src/Grid.mll`).
//
// Part names ("root", "cell-grid") still map to mosstyle selectors so
// the .msl file can provide scoped visual styles. `.mos-Grid-cell-grid`
// now selects the HostTable's emitted `<table>` element directly,
// matching the previous behaviour bit-for-bit on web.

layout Grid {
  Column [ root ] {
    HostTable [ cell-grid ] {
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
}
