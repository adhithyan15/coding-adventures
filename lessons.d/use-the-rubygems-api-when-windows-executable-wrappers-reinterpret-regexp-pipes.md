---
category: Cross-platform & Windows BUILD_windows
---

# Use the RubyGems API when Windows executable wrappers reinterpret regexp pipes

The Windows `gem` executable wrapper reinterpreted a quoted gem-name regexp
containing `|` as a command-shell pipe. The tooling probe therefore tried to
execute one of the names instead of listing installed dependencies. This was a
local diagnostic failure, not evidence of a package or CI failure.

For read-only environment inventory, invoke the known Ruby executable and query
`Gem::Specification.find_all_by_name` for each fixed name. This avoids another
shell parsing layer and reports actual installed versions. Keep production
package commands and their exit-status checks separate from environment probes;
do not change package sources merely to repair a local wrapper invocation.
