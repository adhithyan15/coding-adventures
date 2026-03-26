# @coding-adventures/cli-builder

A declarative CLI argument parsing library driven by directed graphs and state machines.

Write a JSON spec file describing your CLI's structure. CLI Builder handles all
parsing, validation, help generation, and error messaging at runtime. You focus
entirely on what your tool does.

---

## Where This Fits in the Stack

```
Layer N    Your CLI Tool
           ↑ uses
Layer N-1  @coding-adventures/cli-builder   ← this package
           ↑ uses
Layer 7    @coding-adventures/state-machine (DFA, ModalStateMachine)
           ↑ uses
Layer 5    @coding-adventures/directed-graph (Graph, hasCycle, transitiveClosure)
```

CLI Builder is built on the formal automata and directed graph libraries already
in this repository:

- **`DirectedGraph`** drives command routing (G_cmd) and flag dependency
  validation (G_flag). Cycle detection on G_flag catches circular `requires`
  dependencies at load time.

- **`ModalStateMachine`** tracks parse mode token-by-token: `SCANNING`,
  `FLAG_VALUE`, and `END_OF_FLAGS`. The mode switches let the parser handle
  `--output file.txt` (where `file.txt` is a flag value, not a positional
  argument) correctly.

---

## The Core Insight

A CLI tool's valid syntax forms a **directed graph**. Consider `git remote add
origin <url>`: the user navigates from the root `git` node → `remote` node →
`add` node, then provides two positional arguments. Invalid input = a token with
no matching edge from the current node.

Flag constraints form a **second directed graph** layered on top: edges for
`requires` (A requires B) and conflict detection (A conflicts with B).

The state machine describes **how parsing proceeds** token-by-token: after `--output`,
the next token is a value; after `--`, everything is positional.

---

## Quick Start

### 1. Write a spec file (`my-tool.json`)

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "my-tool",
  "description": "Does something useful",
  "version": "1.0.0",
  "flags": [
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Enable verbose output",
      "type": "boolean"
    },
    {
      "id": "output",
      "short": "o",
      "long": "output",
      "description": "Output file path",
      "type": "path",
      "value_name": "FILE"
    }
  ],
  "arguments": [
    {
      "id": "input",
      "name": "INPUT",
      "description": "Input file",
      "type": "file",
      "required": true
    }
  ]
}
```

### 2. Parse in your program

```typescript
import { Parser } from "@coding-adventures/cli-builder";

const parser = new Parser("./my-tool.json", process.argv);
const result = parser.parse();

if ("text" in result) {
  // HelpResult: user passed --help or -h
  process.stdout.write(result.text + "\n");
  process.exit(0);
}

if ("version" in result && !("flags" in result)) {
  // VersionResult: user passed --version
  process.stdout.write(result.version + "\n");
  process.exit(0);
}

// ParseResult: normal invocation
const { flags, arguments: args } = result;
console.log("verbose:", flags["verbose"]);   // true or false
console.log("output:", flags["output"]);     // string or null
console.log("input:", args["input"]);        // string
```

### 3. Handle errors

```typescript
import { ParseErrors, SpecError } from "@coding-adventures/cli-builder";

try {
  const result = parser.parse();
  // ...
} catch (e) {
  if (e instanceof ParseErrors) {
    // User made mistakes — show all errors at once
    for (const err of e.errors) {
      console.error(`[${err.errorType}] ${err.message}`);
      if (err.suggestion) console.error(`  Hint: ${err.suggestion}`);
    }
    process.exit(1);
  }
  if (e instanceof SpecError) {
    // Bug in the spec file — fatal
    console.error("Invalid spec:", e.message);
    process.exit(2);
  }
}
```

---

## Spec Format Reference

The spec file is a UTF-8 JSON document. See `code/specs/cli-builder-spec.md`
for the full specification.

### Top-Level Fields

| Field | Required | Description |
|---|---|---|
| `cli_builder_spec_version` | yes | Must be `"1.0"` |
| `name` | yes | Program name (`"git"`, `"ls"`) |
| `description` | yes | One-line description shown in help |
| `version` | no | If present, `--version` auto-enabled |
| `parsing_mode` | no | `gnu` (default), `posix`, `subcommand_first`, `traditional` |
| `builtin_flags` | no | `{"help": true, "version": true}` |
| `global_flags` | no | Flags valid at every nesting level |
| `flags` | no | Flags valid only at root level |
| `arguments` | no | Positional arguments at root level |
| `commands` | no | Subcommands (recursive) |
| `mutually_exclusive_groups` | no | Groups of mutually exclusive flags |

### Flag Fields

| Field | Required | Description |
|---|---|---|
| `id` | yes | Unique identifier within scope |
| `short` | one of | Single character: `"v"` → `-v` |
| `long` | one of | Word: `"verbose"` → `--verbose` |
| `single_dash_long` | one of | Multi-char with single dash: `"classpath"` → `-classpath` |
| `description` | yes | Help text |
| `type` | yes | `boolean`, `string`, `integer`, `float`, `path`, `file`, `directory`, `enum` |
| `required` | no | Default: `false` |
| `default` | no | Value when absent |
| `value_name` | no | Display name in help: `--output <FILE>` |
| `enum_values` | when enum | List of valid string values |
| `conflicts_with` | no | IDs of conflicting flags |
| `requires` | no | IDs of required dependencies (transitive) |
| `required_unless` | no | Required unless one of these IDs is present |
| `repeatable` | no | If true, may appear multiple times → array result |

---

## Example Specs

The spec file (`code/specs/cli-builder-spec.md`) includes full JSON specs for:

- `echo` — variadic args, flag conflicts
- `ls` — flag stacking (`-lah`), flag requires (`-h` requires `-l`)
- `cp` — variadic sources with required trailing destination (last-wins algorithm)
- `grep` — conditional required arg, exclusive group, repeatable `-e`
- `tar` — traditional mode (`tar xvf` without dashes)
- `git` — deep subcommands, global flags, command aliases
- `docker run` — repeatable flags with values (`-p`, `-e`, `-v`)
- `java` — `single_dash_long` flags (`-classpath`, `-verbose`)

---

## Architecture

```
Parser
├── SpecLoader          loads + validates JSON, builds CliSpec
├── TokenClassifier     classifies argv tokens (DFA-based)
│   └── longest-match-first disambiguation for single_dash_long
├── ModalStateMachine   tracks parse mode (SCANNING/FLAG_VALUE/END_OF_FLAGS)
├── PositionalResolver  assigns positional tokens to arg slots (last-wins)
└── FlagValidator       checks constraints (conflicts, requires, groups)
    └── DirectedGraph.transitiveClosure for transitive requires
```

### Parsing Phases

**Phase 1 — Routing**: Walk argv, match subcommand tokens against G_cmd.
Builds the `command_path` and identifies the leaf `CommandDef`.

**Phase 2 — Scanning**: Re-walk argv, skip command tokens. Classify each token
with `TokenClassifier`. Drive `ModalStateMachine` for mode switching. Accumulate
`parsedFlags` and `positionalTokens`. Return `HelpResult`/`VersionResult` early
if builtin flags are encountered.

**Phase 3 — Validation**:
- `PositionalResolver` partitions positional tokens into argument slots
- `FlagValidator` checks all flag constraints against G_flag
- Collect all errors, throw `ParseErrors` if any

---

## Error Types

All errors include `errorType` (snake_case), `message` (human-readable),
optional `suggestion` (fuzzy match for typos), and `context` (command_path).

| Error Type | Cause |
|---|---|
| `unknown_command` | Token matches no known subcommand |
| `unknown_flag` | Flag token matches no known flag in scope |
| `missing_required_flag` | `required: true` flag absent |
| `missing_required_argument` | `required: true` argument absent |
| `conflicting_flags` | Two `conflicts_with` flags both present |
| `missing_dependency_flag` | A flag's `requires` dependency absent |
| `too_few_arguments` | Variadic arg got fewer than `variadic_min` |
| `too_many_arguments` | More positional tokens than argument slots |
| `invalid_value` | Value failed type coercion |
| `invalid_enum_value` | Value not in `enum_values` |
| `exclusive_group_violation` | Multiple flags in exclusive group present |
| `missing_exclusive_group` | Required exclusive group has no flag present |
| `duplicate_flag` | Non-repeatable flag appears more than once |
| `invalid_stack` | Unknown character in stacked flags |
| `spec_error` | The JSON spec itself is invalid |

---

## Development

```bash
npm install
npm test
npm run test:coverage
```

Tests use [Vitest](https://vitest.dev/) with V8 coverage. Target: 90%+ line coverage.
