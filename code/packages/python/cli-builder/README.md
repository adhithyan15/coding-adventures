# coding-adventures-cli-builder

Declarative CLI argument parsing via directed graphs and state machines.

---

## What It Does

CLI Builder separates *what a tool accepts* from *what a tool does*. You write a
JSON specification file describing the CLI's structure — subcommands, flags,
positional arguments, constraints — and CLI Builder handles all parsing,
validation, help generation, and error reporting at runtime.

This is **Layer 8** of the coding-adventures computing stack, building directly on
the `directed-graph` (Layer 3) and `state-machine` (Layer 4) packages.

---

## How It Fits in the Stack

```
Layer 8 — CLI Builder       ← you are here
Layer 4 — State Machine     (DFA, ModalStateMachine)
Layer 3 — Directed Graph    (DirectedGraph, LabeledDirectedGraph)
Layer 1 — Logic Gates       (NAND, combinational, sequential circuits)
```

The command routing graph is a `DirectedGraph`. Flag dependency checking uses
`transitive_closure` and `has_cycle`. The parse engine is a `ModalStateMachine`
with four modes: ROUTING → SCANNING → FLAG_VALUE → END_OF_FLAGS.

---

## Usage

### 1. Write a JSON spec file

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "myapp",
  "description": "My great application",
  "version": "1.0.0",
  "flags": [
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Enable verbose output",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "file",
      "name": "FILE",
      "description": "Input file",
      "type": "path",
      "required": true
    }
  ]
}
```

### 2. Parse argv

```python
from cli_builder import Parser, ParseResult, HelpResult, VersionResult

result = Parser("myapp.json", ["myapp", "--verbose", "input.txt"]).parse()

if isinstance(result, HelpResult):
    print(result.text)
elif isinstance(result, VersionResult):
    print(result.version)
elif isinstance(result, ParseResult):
    print(result.flags["verbose"])    # True
    print(result.arguments["file"])   # "input.txt"
```

### 3. Handle errors

```python
from cli_builder import Parser, ParseErrors

try:
    result = Parser("myapp.json", ["myapp"]).parse()
except ParseErrors as e:
    print(e)   # nicely formatted list of all errors
    raise SystemExit(1)
```

---

## Supported Parsing Modes

| Mode | Behavior |
|---|---|
| `gnu` | Flags anywhere in argv (default). |
| `posix` | Flags must appear before positional arguments. |
| `subcommand_first` | First non-flag token is always a subcommand. |
| `traditional` | First token may be stacked flags without leading `-` (tar-style). |

---

## Token Types Recognized

- `--` → end of flags marker
- `--verbose` → long flag
- `--output=file.txt` → long flag with inline value
- `-classpath` (declared as `single_dash_long`) → single-dash long flag
- `-l` → short boolean flag
- `-ffile.txt` → short flag with inline value
- `-lah` → stacked boolean short flags
- `-` → positional (stdin/stdout convention)
- `hello` → positional argument

---

## Running Tests

```bash
cd code/packages/python/cli-builder
uv venv
uv pip install -e ../directed-graph -e ../state-machine -e ".[dev]"
python -m pytest
```
