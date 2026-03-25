# event-loop (Elixir)

A pluggable, generic event loop — the heartbeat of any interactive application.

## What is this?

An event loop is the outermost structure of any interactive program. It
repeatedly polls registered **sources** for events, dispatches each event to
registered **handlers**, and stops when a handler signals exit.

This is step one on the road to building a text editor from scratch (the
GPUI framework that powers Zed uses `winit`'s event loop, which has the same
shape as this library).

## Usage

```elixir
alias CodingAdventures.EventLoop

# A source is a {poll_fn, state} tuple.
# poll_fn :: state -> {[events], new_state}
tick_source = {
  fn count ->
    if count < 3 do
      {[:tick], count + 1}
    else
      {[:quit], count + 1}
    end
  end,
  0  # initial state
}

EventLoop.run(
  [tick_source],
  [fn
    :quit -> :exit
    :tick -> IO.puts("tick!"); :continue
  end]
)
```

## API

| Item | Description |
|---|---|
| `EventLoop.run(sources, handlers)` | Start the loop; blocks until a handler returns `:exit` |
| Source | `{poll_fn, state}` — `poll_fn :: state -> {[events], new_state}` |
| Handler | `fn event -> :continue \| :exit` |

## Design notes

- **Functional design**: no mutable state; sources carry their own state forward via `{poll_fn, state}` tuples.
- **Tail-recursive loop**: `loop/2` is tail-recursive and will not blow the stack.
- **State evolution**: `Enum.map_reduce/3` polls all sources and threads their state forward in one pass.
- **No GenServer overhead**: the loop is a plain recursive function — no process mailbox, no OTP supervision needed for this basic case.
- **CPU-friendly idle**: `Process.sleep(0)` yields the scheduler when the event queue is empty.

## Development

```bash
cd code/packages/elixir/event_loop
mix test
```
