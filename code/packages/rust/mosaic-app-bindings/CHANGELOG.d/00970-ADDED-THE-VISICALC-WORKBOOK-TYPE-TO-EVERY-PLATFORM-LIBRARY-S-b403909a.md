### Added the VisiCalc workbook type to every platform library's file-type table (#16927)

The XAML, Qt, Flutter, Compose and SwiftUI file-effect libraries map
`application/vnd.coding-adventures.visicalc+json` to the `.visicalc`
extension. They hold the same row, at the same position, as the browser
executor. VisiCalc's `files.open` / `files.save` can therefore name its own
type, and the save-name rule ("with a type, the name must already end in one
of that type's extensions") accepts `Workbook.visicalc`. The tests that pin
each table to Compose's now expect 14 rows.
