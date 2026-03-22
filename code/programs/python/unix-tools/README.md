# Unix Tools — Reimplemented with CLI Builder

A collection of classic Unix command-line tools, each reimplemented using [CLI Builder](../../../packages/python/cli-builder/). Every tool's command-line interface — flags, help text, version output, error messages — is defined declaratively in a JSON spec. The program files contain only business logic.

## Tools

| Tool | Description | Spec | Implementation |
|------|-------------|------|----------------|
| `pwd` | Print the absolute pathname of the current working directory | [`pwd.json`](pwd.json) | [`pwd_tool.py`](pwd_tool.py) |

## How It Works

Each tool follows the same pattern:

```
tool.json (declarative spec)        tool_impl.py (business logic only)
┌─────────────────────────┐         ┌─────────────────────────────┐
│ flags, arguments        │         │                             │
│ mutual exclusivity      │────────►│  only the core logic        │
│ help text, version      │         │  no argument parsing        │
│ error messages          │         │                             │
└─────────────────────────┘         └─────────────────────────────┘
        CLI Builder                        Your code
     handles all of this              handles only this
```

## Where It Fits in the Stack

```
Layer 8: CLI Builder (argument parsing, help, validation)
    └── This package: unix-tools (business logic only)

Layer 4: State Machine (drives CLI Builder's parsing modes)
Layer 3: Directed Graph (drives CLI Builder's command routing)
```

## Running

```bash
# Via the build system
./build-tool

# Manually (example with pwd)
python pwd_tool.py
python pwd_tool.py -P
python pwd_tool.py --help
```

## Testing

```bash
pytest tests/ -v
```
