// Grid.mll — Layout definition for the Grid component.
//
// Layout tree:
//
//   Column [ root ]          ← flex column container; fills available width
//     Grid [ cell-grid ] (   ← the single table primitive; no children
//       headers: slot: column-headers ,
//       rows:    slot: viewport-rows
//     )
//
// The Grid primitive is a leaf node — it takes no children in the layout
// tree.  Instead it receives slot references as props and the React backend
// expands them into <thead>/<tbody> via .map() at compile time.
//
// Part names ("root", "cell-grid") map to mosstyle selectors so the .msl
// file can provide scoped visual styles without any class name collisions.

layout Grid {
  Column [ root ] {
    Grid [ cell-grid ] (
      headers: slot: column-headers ,
      rows:    slot: viewport-rows
    )
  }
}
