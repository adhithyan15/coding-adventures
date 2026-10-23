---
category: CI & GitHub Actions
---

# A package-manager lock keyed on a list of subcommands misses the write nobody listed: luarocks install ran unserialized

The build tool's executor serializes BUILDs that write the shared LuaRocks
tree under `~/.luarocks` with a `global:luarocks-tree` resource lock. It
matched only `luarocks make` and `luarocks remove`. `luarocks install` writes
the same tree and takes the same exclusive lock, so a full-build shard
(PR #17864, shard 11) failed `lua/tcp_client` with

    Error: command 'install' requires exclusive write access to
    /home/runner/.luarocks - try --force-lock to overwrite the lock

while another package's `luarocks make` held it. Seventeen Lua packages run
`luarocks install`, so it surfaced only when one landed beside a `make`.

Fix: `luarocksTreeWriteRe` in `internal/executor/executor.go` matches
`make`, `remove` and `install` as command words. It applies to any package,
like the opam and gem locks.

Next time:
- When guarding a tool's shared state, list the tool's subcommands that write
  it from the tool's own documentation. Don't take them from the BUILD lines
  you happened to look at.
- Census the repo's actual usage
  (`git grep -hoE "luarocks +[a-z-]+" -- 'code/**/BUILD*' | sort | uniq -c`)
  and check every writing subcommand it finds is covered.
- An "exclusive write access"/lock error in a parallel build is a missing
  serialization, not a flake to rerun.
