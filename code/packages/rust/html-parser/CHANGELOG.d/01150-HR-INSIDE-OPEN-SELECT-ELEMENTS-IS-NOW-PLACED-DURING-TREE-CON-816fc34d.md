- `<hr>` inside open `select` elements is now placed during tree construction,
  removing a post-parse repair while preserving the html5lib select-list DOM
  audit behavior.
