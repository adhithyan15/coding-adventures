# repl

A pluggable Read-Eval-Print Loop (REPL) framework for Lua with injectable language, prompt, and waiting plug-ins.

## What Is a REPL?

A REPL is the interactive shell found in every interpreter — `lua -i`, `python3`, `irb`, `node`. It loops forever:

1. **Read** — show a prompt, read a line from the user
2. **Eval** — pass that line to the language evaluator
3. **Print** — display the result (or error)
4. **Loop** — back to step 1

The loop stops when the user signals quit (`:quit` in EchoLanguage, or Ctrl-D / EOF).

## Design: Plug-in Architecture

Every behavioural concern is a *plug-in* — a plain Lua table with a documented method signature. Nothing is hard-coded. This makes the framework:

- **Testable** — inject fake `input_fn` / `output_fn`; no terminal required
- **Reusable** — drop in any language evaluator without touching loop code
- **Observable** — swap in a spinner or progress bar without changing eval logic

## The Three Plug-in Interfaces

Lua has no formal interface system, so interfaces are documented conventions. The loop validates them at startup.

### Language

```lua
-- language.eval(input) → result
--
-- result is one of:
--   { tag = "ok",    output = string_or_nil }  -- success
--   { tag = "error", message = string        }  -- evaluation failed
--   { tag = "quit"                           }  -- exit the REPL
```

The tagged-union pattern lets callers switch on `result.tag` without `pcall`.

### Prompt

```lua
-- prompt.global_prompt() → string   -- shown before each new expression
-- prompt.line_prompt()   → string   -- shown on continuation lines
```

### Waiting

```lua
-- waiting.start()       → state     -- called before eval
-- waiting.tick(state)   → state     -- called each animation frame
-- waiting.tick_ms()     → integer   -- ms between ticks
-- waiting.stop(state)   → nil       -- called after eval
```

#### Note on Lua's Synchronous Eval

Lua coroutines are cooperative: once `language.eval()` is called, no other code runs until it returns. Therefore `waiting.tick()` is **never called between `start()` and `stop()`** in a standard Lua host. The interface is still correct — a multi-threaded host (e.g., LuaJIT threads) could call `tick()` from a background thread. We document rather than paper over this limitation.

## Built-in Plug-ins

| Name | Role | Behaviour |
|------|------|-----------|
| `EchoLanguage` | Language | Echoes input; `:quit` signals exit |
| `DefaultPrompt` | Prompt | `"> "` and `"... "` |
| `SilentWaiting` | Waiting | All no-ops; `tick_ms()` = 100 |

## I/O Injection

```lua
-- run_with_io(language, prompt, waiting, input_fn, output_fn, opts)
--   input_fn()    → string or nil   (nil = EOF)
--   output_fn(s)  → nil
--   opts          → optional table (see Mode Support below)
```

`run(language, prompt, waiting, opts)` is a convenience wrapper using `io.read` / `io.write`.

## Mode Support

Both `run` and `run_with_io` accept an optional `opts` table as their last
argument. The only recognised field is `mode`:

| `opts.mode` | Behaviour |
|-------------|-----------|
| `"sync"` (default) | Normal synchronous eval — the only mode that works in standard Lua. |
| `"async"` | Raises an error immediately at startup. |

```lua
-- Default (sync) — opts can be omitted entirely
repl.run(repl.EchoLanguage)

-- Explicit sync — identical to the above
repl.run(repl.EchoLanguage, nil, nil, { mode = "sync" })

-- Async — raises: "async mode is not supported in the Lua REPL implementation."
repl.run(repl.EchoLanguage, nil, nil, { mode = "async" })
```

### Why is async not supported?

Lua's standard coroutines are *cooperative*: once `language.eval()` is called,
no other Lua code runs until it returns. There is no preemptive scheduler in
the standard library. True async evaluation would require an external event
loop (e.g., [Luvit](https://luvit.io/), [lua-ev](https://github.com/brimworks/lua-ev),
or LuaJIT with OS threads) that is outside the scope of this package.

Rather than silently ignoring an `async` request or falling back to broken
behaviour, the framework raises a clear error so that cross-runtime code
(e.g., glue code driving both a Lua REPL and a Python REPL that does support
async) gets an actionable message instead of a confusing hang or wrong output.

If you need a waiting animation between `start()` and `stop()`, that is
achievable in sync mode by implementing a `waiting` plug-in whose `stop()`
erases the spinner retroactively after eval returns.

## Package Layout

```
repl/
  src/coding_adventures/repl/
    init.lua            -- public API, re-exports everything
    loop.lua            -- run / run_with_io engine
    echo_language.lua   -- EchoLanguage built-in
    default_prompt.lua  -- DefaultPrompt built-in
    silent_waiting.lua  -- SilentWaiting built-in
  tests/
    test_repl.lua       -- busted test suite
  coding-adventures-repl-0.1.0-1.rockspec
  BUILD
  README.md
  CHANGELOG.md
```

## Usage

### Interactive REPL with EchoLanguage

```lua
local repl = require("coding_adventures.repl")

-- Starts an interactive loop on stdio.
-- Type anything to see it echoed back. Type :quit to exit.
repl.run(repl.EchoLanguage)
```

### Custom Language Plug-in

```lua
local repl = require("coding_adventures.repl")

local UpperLanguage = {
    eval = function(input)
        if input == ":quit" then return { tag = "quit" } end
        return { tag = "ok", output = input:upper() }
    end
}

repl.run(UpperLanguage)
-- > hello
-- HELLO
-- > :quit
```

### Injecting I/O for Testing

```lua
local repl = require("coding_adventures.repl")

local inputs = { "hello", "world", ":quit" }
local i = 0
local outputs = {}

repl.run_with_io(
    repl.EchoLanguage,
    repl.DefaultPrompt,
    repl.SilentWaiting,
    function() i = i + 1; return inputs[i] end,
    function(s) outputs[#outputs + 1] = s end
)

-- outputs now contains: {"> ", "hello\n", "> ", "world\n", "> "}
```

### Custom Waiting Plug-in

```lua
local SpinnerWaiting = {
    frames = {"|", "/", "-", "\\"},
    eval = function(self)   -- (not used here)
    end,
}

function SpinnerWaiting.start()
    return { frame = 1 }
end

function SpinnerWaiting.tick(state)
    io.write("\r" .. SpinnerWaiting.frames[state.frame])
    io.flush()
    return { frame = (state.frame % 4) + 1 }
end

function SpinnerWaiting.tick_ms()
    return 80
end

function SpinnerWaiting.stop(_state)
    io.write("\r  \r")  -- erase spinner
end
```

## Exception Safety

`language.eval()` is wrapped in `pcall()`. If the language plug-in raises a Lua error (via `error()` or a failed `assert`), the REPL catches it, displays `"error: <message>"`, and continues — it does not crash.

## Dependencies

- Lua >= 5.4
- No external libraries

## Development

```bash
# Run tests (from package root)
bash BUILD

# Run tests manually
cd tests && busted . --verbose --pattern=test_
```
