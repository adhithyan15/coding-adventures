### Added — HTML Parser Formatting Adoption
- `</b>` adoption across `<aside>` now preserves the html5lib `<em><foo><foo>`
  continuation during tree construction, retiring the old finish-time
  `<em>/<aside>` post-parse repair.

