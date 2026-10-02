### Changed — Engram's effect handlers claim importAnki and exportAnki explicitly (UI87 §7.2)

Each `[host_effects]` handler (Qt, SwiftUI, Flutter, Compose) now lists
`kinds = ["importAnki", "exportAnki"]`. Every handler already answered exactly
those two kinds and failed anything else, so behaviour is unchanged; the
generated entry point now names them when it installs the platform router
(in each backend's own form, e.g. Kotlin's `setOf("importAnki", "exportAnki")`),
sending
those two to Engram's handler, `files.open`/`files.save` to Mosaic's platform
library, and failing any other kind in the host rather than in Engram's code.
This is the manifest shape UI87 §7.2 shows for Engram.
