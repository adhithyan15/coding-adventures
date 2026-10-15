---
category: Cross-platform & Windows BUILD_windows
---

# Case-variant rejection fixtures must remove the lowercase twin on case-insensitive filesystems

The Script Ductus pin-loader test tried to add `Kannada.json` beside the valid
`kannada.json`. On the default macOS filesystem those names address the same
directory entry, so the fixture rewrote the valid file and `readdirSync`
continued to report its lowercase spelling; the loader correctly saw nothing
unsafe and the negative test failed. A case-policy fixture must first remove
the valid lowercase owner and then add the uppercase spelling. That presents
the intended bad name on both case-sensitive and case-insensitive filesystems.
