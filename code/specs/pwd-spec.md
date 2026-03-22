# pwd — Print Working Directory

A reimplementation of the POSIX `pwd` utility, driven by CLI Builder.

---

## 1. Overview

`pwd` prints the absolute pathname of the current working directory. It is the
simplest possible Unix tool: no subcommands, no positional arguments, just two
mutually exclusive flags that control how symlinks are handled.

This makes `pwd` the ideal first program to build on CLI Builder — the JSON spec
is tiny, the logic fits in a single function, and every language can implement it.

### 1.1 Why pwd?

Building `pwd` across all six languages (Python, Go, Ruby, Rust, TypeScript,
Elixir) demonstrates CLI Builder's core value proposition: **define the interface
once in JSON, implement only the logic**. The same `pwd.json` spec file drives
argument parsing, help generation, version output, and error reporting in every
language — the developer writes zero parsing code.

---

## 2. Behavior

### 2.1 Synopsis

```
pwd [-L | -P] [--help] [--version]
```

### 2.2 Flags

| Flag | Long form | Description |
|------|-----------|-------------|
| `-L` | `--logical` | Display the logical current working directory (from `$PWD` environment variable). This is the default. |
| `-P` | `--physical` | Display the physical current working directory with all symbolic links resolved. |

`-L` and `-P` are mutually exclusive. If neither is specified, `-L` is assumed.

`--help` and `--version` are auto-injected by CLI Builder.

### 2.3 Logical vs Physical

The **logical** path is the value of the `$PWD` environment variable. When a user
`cd`s through a symlink, the shell updates `$PWD` to reflect the symlinked path —
not the resolved target. This is what users expect to see.

The **physical** path resolves all symlinks in the path. For example, if `/home`
is a symlink to `/usr/home`, logical `pwd` shows `/home/user` while physical `pwd`
shows `/usr/home/user`.

If `$PWD` is not set or does not point to the current directory, even `-L` mode
must fall back to the physical path. This matches POSIX behavior.

### 2.4 Exit Status

| Code | Meaning |
|------|---------|
| 0 | Success |
| >0 | An error occurred (e.g., current directory has been deleted) |

---

## 3. CLI Builder Integration

### 3.1 The JSON Spec

The entire CLI interface is described in `pwd.json`:

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "pwd",
  "display_name": "pwd",
  "description": "Print the absolute pathname of the current working directory",
  "version": "1.0.0",
  "parsing_mode": "posix",
  "builtin_flags": { "help": true, "version": true },
  "flags": [
    {
      "id": "logical",
      "short": "L",
      "long": "logical",
      "description": "Display the logical current working directory (default)",
      "type": "boolean"
    },
    {
      "id": "physical",
      "short": "P",
      "long": "physical",
      "description": "Display the physical current working directory (resolve all symlinks)",
      "type": "boolean"
    }
  ],
  "arguments": [],
  "commands": [],
  "mutually_exclusive_groups": [
    {
      "id": "pwd-mode",
      "flag_ids": ["logical", "physical"],
      "required": false
    }
  ]
}
```

### 3.2 What CLI Builder Handles

- Parsing `-L`, `-P`, `--logical`, `--physical`, `--help`, `--version`
- Rejecting unknown flags with suggestions (e.g., `--phsyical` → "did you mean --physical?")
- Enforcing mutual exclusivity of `-L` and `-P`
- Generating help text automatically from the spec
- Returning a structured result (flags dict + arguments dict) to the program

### 3.3 What the Program Handles

The program's only job is the business logic:
1. Read the `physical` flag from the parse result
2. If `physical` is true → resolve symlinks and print
3. Otherwise → read `$PWD` and print (fall back to resolved if `$PWD` is unset)

---

## 4. Implementation Notes

### 4.1 Language-Specific Details

| Language | Physical path | Logical path ($PWD) |
|----------|--------------|---------------------|
| Python | `pathlib.Path.cwd().resolve()` | `os.environ.get("PWD", ...)` |
| Go | `os.Getwd()` returns physical; `os.Getenv("PWD")` for logical |
| Ruby | `Pathname.new('.').realpath` | `ENV["PWD"]` |
| Rust | `std::env::current_dir()` | `std::env::var("PWD")` |
| TypeScript | `fs.realpathSync('.')` | `process.env.PWD` |
| Elixir | `File.cwd!()` then resolve | `System.get_env("PWD")` |

### 4.2 Testing Strategy

Tests exercise CLI Builder integration, not just the logic:
- Parse with no flags → logical result
- Parse with `-P` → physical result
- Parse with `-L` → logical result
- Parse with `--help` → HelpResult with expected text
- Parse with `--version` → VersionResult "1.0.0"
- Parse with unknown flag → error
