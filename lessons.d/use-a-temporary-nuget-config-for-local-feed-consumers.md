---
category: C#
---

# Use a temporary NuGet config for local-feed consumers

Repeated `dotnet restore -s <local-feed> -s <public-URL>` arguments can make
the URL resolve as a relative path on Windows, producing a misleading
`NU1301` path under the consumer directory. Create a temporary `NuGet.Config`
with `<clear />`, the local artifact feed, and the public source, then pass it
with `--configfile`. Use package-source mapping so owned package IDs resolve
only from the local feed, and keep the package cache task-local. Before
recursive cleanup, verify the canonical temp parent, the exact generated child
path, and that the target itself is not a reparse point.
