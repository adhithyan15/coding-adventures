---
category: Elixir
---

# Check-then-act on a registered process name races its exit; unregister and rescue ArgumentError instead

`elixir/programs/ircd`'s test helper read:

```elixir
case Process.whereis(Name) do
  nil -> :ok
  _pid -> Process.unregister(Name)
end
```

It ran in `on_exit`. The handler is started by (and linked to) the test
process. ExUnit shuts that process down at the end of the test and runs
`on_exit` callbacks in a separate process, so the handler was dying from
the propagated exit signal and died between `whereis` and `unregister`.
A dead process's registered name is released automatically, so
`Process.unregister/1` raised `ArgumentError` ("1st argument: not a pid"),
and `clients can join a channel and chat` failed on a CI shard (PR #18005,
shard 15). The bug is in the test, not the server.

**Fix:** act, then interpret the error. Call `Process.unregister(Name)`
unconditionally and `rescue ArgumentError -> :ok`, since either way the
name is free afterwards.

**Do differently:** never pair `Process.whereis/1` with a follow-up
operation on the same name or pid (`unregister`, `exit`, `send` that must
arrive) when the process can be dying concurrently. Make the operation
itself idempotent and treat its "not there" error as the expected
outcome. The same applies to `:ets.delete` after `:ets.info`, and to
`File.rm` after `File.exists?`.
