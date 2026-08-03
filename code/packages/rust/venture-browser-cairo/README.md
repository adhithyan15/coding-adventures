# venture-browser-cairo

This crate owns Venture's backend-neutral native page session and Cairo RGBA
renderer. It combines `venture-browser-core::BrowserHostController` with the
shared HTML layout and paint pipeline, then exposes thin compatibility C ABIs
for the Mosaic-generated Qt, Flutter, and Compose hosts.

The three generated hosts load backend-named copies of this one dynamic
library. They therefore share navigation, chrome projection, scrolling, hover,
link activation, retained-page reflow, and rendering behavior without placing
cross-platform browser ownership in a toolkit-specific package.

```sh
cargo test -p venture-browser-cairo
```
