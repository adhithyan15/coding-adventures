# An isolated Ruby gem consumer must install its local runtime dependency first

When smoke-testing a locally built gem, install its local runtime dependency
into the isolated gem home first. RubyGems may not use that installed gem to
resolve the dependency of another local gem file, so install the package with
`--ignore-dependencies` only after its dependencies are already present. Run
the consumer with `GEM_HOME` and `GEM_PATH` restricted to that directory to
verify the packaged files load without ambient gems.
---
category: Ruby
---
