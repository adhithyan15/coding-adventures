# coding-adventures-event-loop (Lua)

A lightweight event emitter and tick-based scheduler — the foundation of
any interactive application.

## What Is an Event Loop?

An event loop sits at the top of a running program and asks "did anything
happen?" on every iteration, dispatching events to registered handlers.
This module provides two complementary patterns:

- **Push-based**: `on` / `emit` / `once` / `off` — the Observer pattern.
  Producers emit events; consumers register handlers. Decoupled by design.
- **Pull-based**: `on_tick` / `tick` / `run` — the game loop pattern.
  Each tick advances the simulation clock by `delta_time`.

## Usage

```lua
local EventLoop = require("coding_adventures.event_loop")

local loop = EventLoop.new()

-- Register a persistent handler
loop:on("damage", function(data)
    print("Took " .. data.amount .. " damage!")
end)

-- Register a one-shot handler
loop:once("startup", function(data)
    print("Version: " .. data.version)
end)

-- Register a tick handler (game loop style)
loop:on_tick(function(dt)
    -- advance physics by dt seconds
end)

-- Fire an event
loop:emit("startup", { version = "1.0" })
loop:emit("damage",  { amount = 10 })
loop:emit("startup", { version = "1.0" })  -- once handler does NOT fire again

-- Run 60 ticks at 1/60 second each
loop:run(60, 1/60)
print(loop.elapsed_time)   -- ≈ 1.0
print(loop.tick_count)     -- 60
```

## API

### `EventLoop.new()` → loop
Create a new event loop instance.

### `loop:on(event_name, callback)`
Register a persistent handler. Fires on every `emit(event_name, ...)`.

### `loop:once(event_name, callback)`
Register a one-shot handler. Automatically removed after the first emit.

### `loop:off(event_name [, callback])`
Remove handlers. If `callback` is nil, removes all handlers for the event.
Otherwise removes only the first matching handler.

### `loop:emit(event_name, data)`
Fire all handlers registered for `event_name`, passing `data` to each.

### `loop:on_tick(callback)`
Register a tick handler: `function(delta_time)`.

### `loop:tick([delta_time])`
Advance time by one step (default `delta_time = 1.0`). Fires all tick handlers.

### `loop:run([n_ticks [, delta_time]])`
Run `n_ticks` ticks (default 1) each with the given `delta_time` (default 1.0).

### `loop:step([delta_time])`
Convenience alias for `loop:run(1, delta_time)`.

## License

MIT
