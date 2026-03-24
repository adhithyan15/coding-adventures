# progress-bar

Terminal progress bar for tracking build execution. Lua 5.4 port of the
Go `progress-bar` package from the coding-adventures monorepo.

## How it fits in the stack

This package provides visual feedback during build execution. The build tool
uses it to show which packages are being built, how many are complete, and
how long the build has been running. It supports both flat (single-level)
and hierarchical (parent/child) progress tracking.

## API

### Event types

```lua
local progress = require("coding_adventures.progress_bar")

progress.STARTED   -- item began processing
progress.FINISHED  -- item completed (success or failure)
progress.SKIPPED   -- item was bypassed
```

### Flat mode

```lua
local t = progress.new(21, io.stderr, "")
t:start()
t:send({ type = progress.STARTED, name = "pkg-a" })
t:send({ type = progress.FINISHED, name = "pkg-a", status = "built" })
t:send({ type = progress.SKIPPED, name = "pkg-b" })
t:stop()
```

Output:

```
[████████░░░░░░░░░░░░]  7/21  Building: pkg-a, pkg-b  (12.3s)
```

### Labeled mode

```lua
local t = progress.new(3, io.stderr, "Level")
t:start()
t:send({ type = progress.SKIPPED, name = "level-1" })
t:stop()
```

Output:

```
Level 1/3  [██████░░░░░░░░░░░░░░]  waiting...  (0.5s)
```

### Hierarchical mode

```lua
local parent = progress.new(3, io.stderr, "Level")
parent:start()

local child = parent:child(7, "Package")
child:send({ type = progress.STARTED, name = "pkg-a" })
child:send({ type = progress.FINISHED, name = "pkg-a", status = "built" })
child:finish()   -- advances parent by 1

parent:stop()
```

Output:

```
Level 1/3  [████░░░░░░░░░░░░░░░░]  3/7  Building: pkg-a  (2.1s)
```

## Differences from Go version

- **Synchronous**: Go uses goroutines and channels for concurrent event
  processing. This Lua version processes events synchronously in `:send()`.
  Each call immediately updates state and redraws.
- **String event types**: Go uses `iota` integer constants. Lua uses string
  constants (`"STARTED"`, `"FINISHED"`, `"SKIPPED"`) for easier debugging.
- **Writer interface**: Go uses `io.Writer`. Lua uses any table with a
  `:write(str)` method (e.g., `io.stderr`).
- **No nil-receiver pattern**: Go allows calling methods on nil pointers.
  Lua trackers must be non-nil, but `:send()` is a no-op before `:start()`.

## Development

```bash
# Run tests
cd tests && busted . --verbose --pattern=test_
```
