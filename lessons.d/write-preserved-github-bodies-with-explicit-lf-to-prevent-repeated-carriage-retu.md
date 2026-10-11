---
category: Cross-platform & Windows BUILD_windows
---

# Write preserved GitHub bodies with explicit LF to prevent repeated carriage returns on Windows

An issue update preserved an existing GitHub body containing CRLF sequences,
then wrote the combined body with Windows default newline conversion. Existing
CRLFs became repeated carriage returns, so a verified remote write differed
from the intended body and risked accumulating blank lines on later updates.

Normalize preserved API text by removing carriage returns, then write the
body-file with explicit LF (`Path.write_text(..., newline='\n')`). Compare the
remote reread using the same normalization. Preserve the substantive legacy
body and stop on a real content mismatch instead of accepting any successful
write command as evidence that the exact intended body was published.
