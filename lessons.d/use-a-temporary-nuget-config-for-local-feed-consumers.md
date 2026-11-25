---
category: C#
---

# Use a temporary NuGet config for local-feed consumers

Repeated `dotnet restore -s <local-feed> -s <public-URL>` arguments can make
the URL resolve as a relative path on Windows, producing a misleading
`NU1301` path under the consumer directory. Create a temporary `NuGet.Config`
with `<clear />`, the local artifact feed, and the public source, then pass it
with `--configfile`. Keep the package cache task-local when validating that an
installed consumer resolves the packed artifacts rather than a machine cache.
