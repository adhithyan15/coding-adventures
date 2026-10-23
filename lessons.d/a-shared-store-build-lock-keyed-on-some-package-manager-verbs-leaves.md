---
category: BUILD files & dependency management
---

# A shared-store build lock keyed on some package-manager verbs leaves the others racing it

The Go build tool serialises BUILDs that write the shared LuaRocks tree
(`~/.luarocks`) with a `global:luarocks-tree` lock. It recognised writers by
substring, `luarocks make` or `luarocks remove`, and only in Lua packages.
`lua/tcp_client` runs `luarocks install luasocket --local`. It took no lock,
ran beside two locked `luarocks make`s, and both of those failed in 0.2s
with `command 'make' requires exclusive write access to
/home/runner/.luarocks` (PR #17822, shard 11). A lock protects nothing if
any writer can skip it.

**Fix:** match the command word plus every verb that writes the store
(`make|install|remove|build|purge|test|config`, leading flags allowed),
in any package, with a regex like `rubyGemInstallRe`. Table tests cover
writers and read-only verbs.

**Do differently:** when adding a shared-resource lock, list the tool's
writing verbs from its documentation. Don't just list the ones today's BUILD
files happen to use. Then grep the BUILD files for every verb actually in
use (`grep -rhoE 'luarocks +(-[^ ]+ +)*[a-z_]+'`). A new BUILD that uses a
verb the lock doesn't know about gets a race that only shows up when the
scheduler happens to put it next to a locked one.
