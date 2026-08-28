# venture-browser-cairo

This crate owns Venture's backend-neutral native page session and Cairo RGBA
renderer. It combines `venture-browser-core::BrowserHostController` with the
shared HTML layout and paint pipeline, then exposes thin compatibility C ABIs
for the Mosaic-generated Qt, Flutter, and Compose hosts.

The three generated hosts load backend-named copies of this one dynamic
library. They therefore share navigation, chrome projection, scrolling, hover,
link activation, retained-page reflow, and rendering behavior without placing
cross-platform browser ownership in a toolkit-specific package.

`begin_navigation` and `complete_subresource` expose the core-owned
document-first lifecycle to Qt, Flutter, and Compose event loops. The bridge
retains no toolkit-specific loader or decoder; hosts dispatch the shared
scheduler effects and repaint only when completion outcomes request it.

View Source follows that same boundary: the shared core creates the synthetic
preformatted document and the C ABI serializes one `open-auxiliary-document`
effect. Qt emits the request to its window layer, while Flutter and Compose
retain it for their presenters and direct acceptance; none reparses source.

The bridge also loads and atomically persists the shared versioned bookmark
catalog. Linux follows `$XDG_DATA_HOME/venture/bookmarks.json` (falling back to
`~/.local/share/venture/bookmarks.json`); every platform can override the path
with `VENTURE_BOOKMARKS_PATH` for isolated profiles.

```sh
cargo test -p venture-browser-cairo
```
