### Added — xmlns deduplication

Two references to the same package produce ONE `xmlns:prefix="..."`
declaration on the `<UserControl>` root. The internal map is keyed
by xmlns prefix; `BTreeMap` storage gives deterministic alphabetical
output ordering.

