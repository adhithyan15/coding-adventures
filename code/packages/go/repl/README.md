# repl (Go)

A language-agnostic **Read–Eval–Print Loop** framework.

## What is this?

A REPL (Read–Eval–Print Loop) is the interactive shell found in Python, Ruby,
Elixir's `iex`, Node.js, and virtually every dynamic language. This package
provides the *framework* — the scaffolding that handles I/O, concurrency, and
the main loop. You plug in your own language evaluator and get a fully
functional interactive shell.

## How it fits in the stack

```
Your language back-end
        │
        ▼
 ┌──────────────┐
 │  repl (this) │  ← loop, async eval, I/O injection
 └──────────────┘
        │
        ▼
   os.Stdin / os.Stdout  (or your injected I/O)
```

## Interfaces

### `Language`

The only interface you *must* implement to build a REPL back-end:

```go
type Language interface {
    Eval(input string) Result
}
```

`Eval` receives one line (or logical unit) of user input and returns a `Result`
describing what happened.

### `Result`

```go
type Result struct {
    Tag       string  // "ok", "error", or "quit"
    Output    string  // the text to print (when HasOutput is true)
    HasOutput bool    // false → don't print anything (e.g., silent assignment)
}
```

### `Prompt`

Controls the strings shown before each input line:

```go
type Prompt interface {
    GlobalPrompt() string   // "> "  — primary prompt
    LinePrompt()   string   // "... " — continuation prompt
}
```

### `Waiting`

Animates the terminal while eval is running (spinner, progress bar, etc.):

```go
type Waiting interface {
    Start() interface{}
    Tick(state interface{}) interface{}
    TickMs() int
    Stop(state interface{})
}
```

## Built-in implementations

| Name | Interface | Description |
|------|-----------|-------------|
| `EchoLanguage` | `Language` | Echoes input; `:quit` exits |
| `DefaultPrompt` | `Prompt` | `"> "` and `"... "` |
| `SilentWaiting` | `Waiting` | No-op; 100 ms tick |

## Usage

```go
package main

import "github.com/adhithyan15/coding-adventures/code/packages/go/repl"

func main() {
    // Use the built-ins for a minimal echo REPL.
    repl.Run(repl.EchoLanguage{}, repl.DefaultPrompt{}, repl.SilentWaiting{})
}
```

### Custom language

```go
type MyLang struct{ /* interpreter state */ }

func (m *MyLang) Eval(input string) repl.Result {
    if input == ":quit" {
        return repl.Result{Tag: "quit"}
    }
    output, err := m.interpret(input)
    if err != nil {
        return repl.Result{Tag: "error", Output: err.Error(), HasOutput: true}
    }
    return repl.Result{Tag: "ok", Output: output, HasOutput: output != ""}
}

func main() {
    repl.Run(&MyLang{}, repl.DefaultPrompt{}, repl.SilentWaiting{})
}
```

### I/O injection (tests, pipes)

```go
inputs := []string{"hello", "world", ":quit"}
i := 0
inputFn := func() (string, bool) {
    if i >= len(inputs) { return "", false }
    s := inputs[i]; i++
    return s, true
}

var buf strings.Builder
outputFn := func(s string) { buf.WriteString(s) }

repl.RunWithIO(repl.EchoLanguage{}, repl.DefaultPrompt{}, repl.SilentWaiting{},
    inputFn, outputFn)

fmt.Println(buf.String())
```

## Panic safety

If your `Language.Eval` implementation panics, the REPL catches the panic,
prints an error message, and continues. The loop will not crash.

## Async evaluation

Every call to `Eval` runs in a fresh goroutine. The main loop ticks the
`Waiting` animation while it waits. This design means:

- Long-running evaluations don't freeze the UI.
- Spinner / progress animations are easy to implement.
- The framework is safe to use in concurrent programs.
