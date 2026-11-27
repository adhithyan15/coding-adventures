---
category: Swift
---

# A Swift buffer closure that returns nil must declare its optional result type

In the Windows-only environment reader, `withUnsafeBufferPointer` returned a `String` on its success path while an empty-buffer guard returned `nil`. Swift inferred the closure result as `String` from the other returns and rejected `nil` only when Windows compiled the conditional branch. Annotate the closure result explicitly (`{ pointer -> String? in`) whenever its branches include both a value and `nil`; then run the Windows Swift package build as well as the host-platform tests because Linux excludes `#if os(Windows)` code from type checking.
