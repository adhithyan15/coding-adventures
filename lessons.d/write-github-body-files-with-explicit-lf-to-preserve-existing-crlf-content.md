---
category: Cross-platform & Windows BUILD_windows
---

# Write GitHub body files with explicit LF to preserve existing CRLF content

A scoped issue checkpoint preserved the existing API body around its owned
marker, then saved the payload with Python's platform-default newline handling.
On Windows, pre-existing CRLF plus new LF became mixed/doubled carriage returns.
The edit succeeded, but exact API readback verification correctly failed even
though the ordered logical lines matched.

Keep the failed payload and actual readback. Normalize line endings explicitly
and write with `newline="\n"`; compare the complete API readback, not only a
successful CLI exit. Verify all logical lines outside the owned marker remain
identical before applying this formatting repair. A failed equality assertion
is not an accepted checkpoint and must not be hidden by trimming arbitrary text.
