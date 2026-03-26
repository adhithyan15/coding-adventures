# CLI Builder Specification

A language-agnostic runtime library for declarative CLI argument parsing, driven by
directed graphs and state machines.

---

## 1. Overview and Motivation

Building a CLI tool involves two distinct concerns:

1. **What the tool accepts** — the valid syntax: which subcommands exist, which flags
   apply to which commands, which arguments are required, what values are valid.
2. **What the tool does** — the implementation: the business logic that runs once the
   user's input has been validated and parsed.

Most CLI libraries conflate these. Developers write parsing code interleaved with
business logic, repeat validation boilerplate across every tool, and end up with
help text that drifts out of sync with reality.

CLI Builder separates them cleanly. A developer writes a JSON specification file
describing the CLI's structure, and CLI Builder handles all parsing, validation,
help generation, and error messaging at runtime. The developer focuses entirely on
what the tool does.

### 1.1 The Directed Graph Insight

A CLI tool's valid syntax forms a directed graph. Consider `git remote add origin
<url>`: the user navigates from the root `git` node, to the `remote` subcommand
node, to the `add` subcommand node, then provides two positional arguments. The
valid invocations of a CLI tool are exactly the valid paths through this graph from
the root node to an accepting state.

```
program_root
  └── remote
        ├── add      <name> <url>
        ├── remove   <name>
        ├── rename   <old> <new>
        └── set-url  <name> <url>
```

Every node in this graph is a valid stopping point (if no required arguments remain).
Every edge is labeled by the token that triggers the transition. Invalid input = a
token with no matching edge from the current node.

Flag constraints (one flag requires another; two flags conflict) form a second
directed graph layered on top of the routing graph. Cycle detection on that graph
catches spec errors like "A requires B requires A" before any parsing happens.

### 1.2 The State Machine Execution Engine

The directed graph describes *what is valid*. A state machine describes *how
parsing proceeds* token by token.

Parsing argv is inherently stateful. After seeing `--output`, the next token is
a value, not a flag or subcommand — the parser is in a different mode. After seeing
`--`, all subsequent tokens are positional regardless of leading dashes. After
routing into `git commit`, flags valid for `git add` are no longer in scope.

CLI Builder drives parsing with a Modal State Machine composed of four modes:

```
ROUTING ──────────────────────────────────────► SCANNING
  │  (subcommand token consumed)                    │
  │  (non-subcommand, non-flag token seen)           │
  │                                                  │
  │                                            ┌─────┴──────────────┐
  │                                            │                    │
  │                                        FLAG_VALUE         END_OF_FLAGS
  │                                            │  (after --)        │
  │                                            │                    │
  └────────────────────────────────────────────┘                    │
                                                      (all remaining tokens
                                                       are positional)
```

The token classification DFA reads each argv token character-by-character and
emits a typed token event that drives the modal machine.

### 1.3 Design Goals

1. **Language agnostic** — one JSON spec file works with CLI Builder implementations
   in any language
2. **Declarative** — describe *what* is accepted, not *how* to parse it
3. **Separation of concerns** — the spec is the interface; the implementation is the
   behavior
4. **Composable** — built on the `directed_graph` and `state_machine` packages already
   in this repository
5. **~85% coverage** — targets the vast majority of CLI tools; tools with embedded
   sub-languages (find predicates, awk programs) are explicitly out of scope

---

## 2. Specification Format

The CLI specification is a UTF-8 JSON file. An implementation reads this file at
startup, validates it, builds the internal directed graph and state machine, and
is then ready to parse argv.

### 2.1 Top-Level Structure

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "program-name",
  "display_name": "Program Name",
  "description": "One-line description of what this program does",
  "version": "1.0.0",
  "parsing_mode": "gnu",
  "builtin_flags": { "help": true, "version": true },
  "global_flags": [ ... ],
  "flags": [ ... ],
  "arguments": [ ... ],
  "commands": [ ... ],
  "mutually_exclusive_groups": [ ... ]
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `cli_builder_spec_version` | string | yes | Spec format version. Currently `"1.0"`. |
| `name` | string | yes | Program name as invoked (e.g., `"ls"`, `"git"`). |
| `display_name` | string | no | Human-readable name for help output. |
| `description` | string | yes | One-line description. Shown in help. |
| `version` | string | no | Version string. If present, `--version` is auto-enabled. |
| `parsing_mode` | string | no | One of `"posix"`, `"gnu"`, `"subcommand_first"`, `"traditional"`. Default: `"gnu"`. |
| `builtin_flags` | object | no | Control auto-injection of `--help` and `--version`. Default: both `true`. |
| `global_flags` | array | no | Flags valid at every nesting level (see §2.2). |
| `flags` | array | no | Flags valid only at root level (see §2.2). |
| `arguments` | array | no | Positional arguments at root level (see §2.3). |
| `commands` | array | no | Subcommands (see §2.4). Recursive — commands may contain commands. |
| `mutually_exclusive_groups` | array | no | Groups of mutually exclusive flags (see §2.5). |

**Parsing modes:**

| Mode | Behavior |
|---|---|
| `posix` | Flags must appear before positional arguments. The first non-flag token ends flag scanning. Equivalent to POSIX `getopt` behavior. |
| `gnu` | Flags may appear anywhere in argv. `--` always ends flag scanning. This is the default and matches most modern CLI tools. |
| `subcommand_first` | The first non-flag token is always interpreted as a subcommand name, never as a positional argument. |
| `traditional` | The first token may be stacked short flags without a leading `-` (tar-style). If `argv[1]` does not start with `-` and is not a recognized subcommand, it is treated as concatenated short flag characters: `tar xvf` is equivalent to `tar -x -v -f`. Falls back to `gnu` for all subsequent tokens. |

### 2.2 Flag Definition

A flag is a named, optional token prefixed with `-` (short form) or `--` (long
form). At least one of `short`, `long`, or `single_dash_long` must be specified.

```json
{
  "id": "long-listing",
  "short": "l",
  "long": "long-listing",
  "description": "Use long listing format",
  "type": "boolean",
  "required": false,
  "default": null,
  "value_name": null,
  "enum_values": [],
  "conflicts_with": ["single-column"],
  "requires": [],
  "required_unless": [],
  "repeatable": false
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | yes | Unique identifier within this scope. Used in `conflicts_with`, `requires`, output dict keys. |
| `short` | string | no | Single character without the `-` prefix (e.g., `"l"`). |
| `long` | string | no | Word or hyphenated word without the `--` prefix (e.g., `"long-listing"`). |
| `single_dash_long` | string | no | Multi-character name used with a single leading `-` (e.g., `"classpath"` → `-classpath`). Used by Java, X11, and some Unix utilities. Matched with longest-match-first (see §5.2). |
| `description` | string | yes | Human-readable description. Shown in help output. |
| `type` | string | yes | Value type (see §3). |
| `required` | boolean | no | Whether this flag must be present. Default: `false`. |
| `default` | any | no | Value used when the flag is absent and `required` is `false`. |
| `value_name` | string | no | Shown in help for non-boolean flags: `--output=VALUE`. |
| `enum_values` | array | no | Required when `type` is `"enum"`. Lists all valid string values. |
| `conflicts_with` | array | no | IDs of flags that cannot be used alongside this one. Edges in the flag conflict graph. |
| `requires` | array | no | IDs of flags that must also be present when this flag is used. Edges in the flag dependency graph G_flag. |
| `required_unless` | array | no | This flag is required unless at least one of the listed flag IDs is present. |
| `repeatable` | boolean | no | If `true`, the flag may appear multiple times. The result is an array of values. Default: `false`. |

**Constraints:**
- `id` must be unique within its scope (root, or a specific `commands` entry).
- If `type` is `"enum"`, `enum_values` must be a non-empty array.
- If `type` is `"boolean"`, `value_name` and `enum_values` are ignored.
- `conflicts_with` and `requires` must reference valid flag IDs within the same scope
  or within `global_flags`.

### 2.3 Argument Definition

An argument is a positional token: one that does not begin with `-` and is not a
subcommand name.

```json
{
  "id": "source",
  "display_name": "SOURCE",
  "description": "Source file or directory to copy",
  "type": "path",
  "required": true,
  "variadic": true,
  "variadic_min": 1,
  "variadic_max": null,
  "default": null,
  "enum_values": [],
  "required_unless_flag": []
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | yes | Unique identifier within this scope. Used in output dict keys. |
| `display_name` | string | yes | Display name shown in help text (e.g., `"FILE"`, `"DEST"`). Required args shown as `<DISPLAY_NAME>`, optional as `[DISPLAY_NAME]`. |
| `description` | string | yes | Human-readable description. |
| `type` | string | yes | Value type (see §3). |
| `required` | boolean | no | Whether at least one value must be provided. Default: `true`. |
| `variadic` | boolean | no | Whether multiple values may be provided. Default: `false`. |
| `variadic_min` | integer | no | Minimum count when `variadic` is `true`. Default: `1` if `required`, else `0`. |
| `variadic_max` | integer | no | Maximum count when `variadic` is `true`. `null` means unlimited. Default: `null`. |
| `default` | any | no | Used when `required` is `false` and the argument is absent. |
| `enum_values` | array | no | Required when `type` is `"enum"`. |
| `required_unless_flag` | array | no | This argument is optional if any of the listed flag IDs is present. Useful for patterns like `grep` where `-e PATTERN` makes the positional `PATTERN` optional. |

**Argument ordering rules:**

1. Arguments are consumed in the order they appear in the spec array.
2. At most one argument in a scope may be `variadic`. It need not be last — but if
   non-variadic required arguments follow it, the positional resolution algorithm
   (§6.4) applies.
3. Optional arguments (`required: false`) may appear in any position in the spec.
   The parser attempts to satisfy them in order but does not error if absent.

**Backward compatibility note:** The `display_name` field was previously called
`name` in earlier drafts. Implementations should accept both `display_name` and
`name` for arguments, preferring `display_name` when both are present. New specs
should always use `display_name`.

### 2.4 Command Definition

A command (subcommand) is a named routing token. When the parser encounters it
during the routing phase, it transitions into that command's context: its own flags,
arguments, and nested subcommands become active.

```json
{
  "id": "cmd-remote-add",
  "name": "add",
  "aliases": ["a"],
  "description": "Add a named remote repository",
  "inherit_global_flags": true,
  "flags": [ ... ],
  "arguments": [ ... ],
  "commands": [ ... ]
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | yes | Unique identifier within the parent's `commands` array. |
| `name` | string | yes | The token the user types (e.g., `"add"`, `"commit"`). |
| `aliases` | array | no | Alternative tokens for this command. The canonical `name` is always used in the output's `command_path`. |
| `description` | string | yes | Human-readable description. Shown in `COMMANDS` table of parent's help. |
| `inherit_global_flags` | boolean | no | Whether `global_flags` defined at the root apply in this context. Default: `true`. |
| `flags` | array | no | Flags specific to this subcommand context. Same schema as §2.2. |
| `arguments` | array | no | Positional arguments for this subcommand. Same schema as §2.3. |
| `commands` | array | no | Nested subcommands. Same schema as §2.4. Recursive. |

**Constraints:**
- `id` must be unique among siblings (commands at the same nesting level).
- `name` and all `aliases` must be unique among siblings.
- A command with no `commands` of its own is a leaf command. Its associated
  `flags` and `arguments` define what is accepted in its context.

### 2.5 Mutually Exclusive Groups

A mutually exclusive group ensures that at most one (or, if `required`, exactly
one) of a set of flags is used in a single invocation.

```json
{
  "id": "grep-engine",
  "flag_ids": ["extended-regexp", "fixed-strings", "perl-regexp"],
  "required": false
}
```

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | string | yes | Unique identifier. |
| `flag_ids` | array | yes | IDs of flags in this group. Must reference valid flags in the same scope. |
| `required` | boolean | no | If `true`, exactly one of the flags must be present. If `false`, at most one may be present. Default: `false`. |

Note: `conflicts_with` on individual flags is bilateral and pairwise. Mutually
exclusive groups are the right tool when three or more flags are collectively
exclusive — expressing this with `conflicts_with` alone would require N² edges.

---

## 3. Type System

The following types are valid for flag values and positional arguments.

| Type | Description | Validation at Parse Time |
|---|---|---|
| `boolean` | Flag presence = `true`, absence = `false`. No value token is consumed. | N/A |
| `string` | Any non-empty string. | Must be non-empty. |
| `integer` | A whole number in base 10. | Must parse as a signed integer. |
| `float` | A floating-point number. | Must parse as an IEEE 754 double. |
| `path` | A filesystem path (file or directory). | Must be syntactically valid. Existence is **not** checked at parse time. Use for output paths or paths that may not exist yet. |
| `file` | Path to an existing file. | Must refer to an existing, readable file at parse time. |
| `directory` | Path to an existing directory. | Must refer to an existing directory at parse time. |
| `enum` | One of a fixed set of strings. | Must exactly match one of `enum_values`. Comparison is case-sensitive. |

**Notes:**
- `file` and `directory` require filesystem access at parse time. Implementations
  must treat permission errors as an `invalid_value` error, not a crash.
- For variadic arguments, each element in the list is validated individually.
- Implementations should coerce types eagerly (at parse time) rather than returning
  raw strings and expecting the caller to coerce. The parsed result should contain
  native integer/float values, not strings.

---

## 4. The Directed Graph Model

### 4.1 Command Routing Graph (G_cmd)

The spec defines a command routing graph G_cmd = (V_cmd, E_cmd):

- **V_cmd** contains one `ProgramNode` (the root) plus one `CommandNode` for every
  entry in `commands` at any nesting depth.
- **E_cmd** contains one directed edge `parent → child` for each subcommand
  relationship. Each edge is labeled by the command's `name` and all of its `aliases`.

The routing graph is used at runtime during Phase 1 (§6.2). At each step, the
parser calls `successors(current_node)` on G_cmd to get the set of valid next
subcommands. If the current token matches a label on any outgoing edge, the parser
follows that edge; otherwise routing ends.

G_cmd must be a DAG (no cycles). Since commands cannot contain themselves, this is
guaranteed by the recursive JSON structure — but implementations may verify it with
`has_cycle?` as a sanity check.

### 4.2 Flag Dependency Graph (G_flag)

Each command scope has its own flag dependency graph G_flag = (V_flag, E_flag):

- **V_flag** contains one node per flag in scope (including inherited `global_flags`).
- **E_flag** contains a directed edge `A → B` for every flag `A` that lists `B` in
  its `requires` array.

G_flag is used in two ways:

1. **Spec validation (load time)**: Call `has_cycle?` on G_flag. A cycle means the
   spec is self-contradictory (e.g., `-v requires -q` and `-q requires -v`). This
   is a spec error — the library must report it and refuse to parse.
2. **Constraint validation (parse time)**: For every flag that is present in the
   parsed result, call `transitive_closure(flag_id)` on G_flag and verify that all
   transitively required flags are also present.

### 4.3 Parsing as Graph Traversal

A valid argv invocation is one where:

1. There exists a path in G_cmd from `ProgramNode` to some node N, where each edge
   on the path is matched by a token in argv.
2. The remaining tokens (flags and positional arguments) satisfy N's flag and
   argument schema.
3. All constraint graphs (conflicts, requires, exclusive groups) are satisfied.

The state machine (§5) is the execution engine that performs this traversal.

---

## 5. Token Classification and Disambiguation

### 5.1 The Token Classification DFA

Before parsing, the implementation constructs a token classification DFA from the
resolved command's flag set (global flags + flags in the command path). This DFA
reads one argv token character-by-character and emits a typed token event.

**Token types:**

| Token Type | Pattern | Notes |
|---|---|---|
| `END_OF_FLAGS` | Exactly `--` | Signals that all remaining tokens are positional. |
| `LONG_FLAG(name)` | `--name` (no `=`) | Boolean flag or value-taking flag where value follows as next token. |
| `LONG_FLAG_WITH_VALUE(name, value)` | `--name=value` | Flag and value in a single token. |
| `SINGLE_DASH_LONG(name)` | `-name` matching a declared `single_dash_long` flag | Longest-match-first (see §5.2). |
| `SHORT_FLAG(char)` | `-x` where `x` is a single declared short flag | May consume next token as value if flag is non-boolean. |
| `SHORT_FLAG_WITH_VALUE(char, value)` | `-xVALUE` where `x` is a non-boolean flag | Value is the remainder of the token after the flag character. |
| `STACKED_FLAGS(chars)` | `-xyz` after longest-match fails | Each character must be a valid boolean short flag, except optionally the last. |
| `POSITIONAL(value)` | Any token that does not begin with `-`, or begins with `-` but matches nothing | Consumed as a positional argument value. |

The special token `"-"` (a single dash with no following character) is always
`POSITIONAL("-")`. It conventionally represents stdin/stdout in Unix tools.

### 5.2 Longest-Match-First Disambiguation

When a token begins with a single `-` followed by two or more characters that are
not another `-`, the classifier applies these rules in order. The first rule that
matches wins.

**Rule 1 — Single-dash-long match:**
Check whether the substring after `-` exactly matches the `single_dash_long` field
of any flag in scope. If yes, emit `SINGLE_DASH_LONG(name)`.

```
token: "-classpath"   known single_dash_long: ["classpath", "cp", "jar"]
→ SINGLE_DASH_LONG("classpath")  ✓
```

**Rule 2 — Single-character short flag:**
Check whether `token[1]` (the first character after `-`) exactly matches the `short`
field of any flag in scope. If yes:
- If that flag is `boolean`: emit `SHORT_FLAG(char)` and treat remaining characters
  as the start of a new potential stack (recurse into Rule 1 then Rule 2 on the
  remainder — see stacking below).
- If that flag is non-boolean: the remaining characters (if any) are the inline
  value. Emit `SHORT_FLAG_WITH_VALUE(char, remainder)` if remainder is non-empty,
  or `SHORT_FLAG(char)` if remainder is empty (value will be the next token).

**Rule 3 — Stacked short flags:**
The token is treated as concatenated short flag characters. Walk each character:
- If it matches a `boolean` short flag, record it and continue.
- If it matches a non-boolean short flag, the remaining characters are its inline
  value. Stop.
- If it does not match any short flag, emit `UNKNOWN_FLAG` error.

All characters in the stack except possibly the last must be boolean flags.

```
token: "-lah"   flags: l=boolean, a=boolean, h=boolean
→ STACKED_FLAGS(['l', 'a', 'h'])  ✓

token: "-lf"    flags: l=boolean, f=file (non-boolean)
→ not valid as stacked: -f requires a value. Emit SHORT_FLAG('l'), then
  SHORT_FLAG_WITH_VALUE would need the rest... but 'f' is last with no value chars.
  Emit SHORT_FLAG('l') + SHORT_FLAG('f') where the value is the next token.  ✓

token: "-lX"    flags: l=boolean, X=unknown
→ SHORT_FLAG('l'), then UNKNOWN_FLAG('-X')  ✗ (error)
```

**Rule 4 — No match:**
Emit `UNKNOWN_FLAG` error with a fuzzy suggestion.

### 5.3 Traditional Mode (tar-style)

When `parsing_mode` is `"traditional"`, the classifier applies one additional rule
before all others, but only for `argv[1]` (the first token after the program name):

If `argv[1]` does not start with `-` and does not match any known subcommand name,
treat it as a stack of short flag characters without a leading dash. Walk each
character using the stacking rules from §5.2. If this fails (unknown character),
fall back to treating it as a positional argument.

```
argv: ["tar", "xvf", "archive.tar"]
argv[1] = "xvf" (no leading dash, not a subcommand)
→ classify as STACKED_FLAGS(['x', 'v', 'f'])
argv[2] = "archive.tar" → POSITIONAL("archive.tar")
```

All tokens after `argv[1]` follow normal `gnu` mode rules.

---

## 6. Parsing Algorithm

Parsing proceeds in three sequential phases: routing, scanning, and validation.
The directed graph drives routing; the modal state machine drives scanning;
constraint validation uses both.

### 6.1 Preprocessing

Strip `argv[0]` (the program name itself). Record it as `program`.

If `parsing_mode` is `"traditional"`, check `argv[1]`. Apply the traditional-mode
classifier (§5.3) if applicable, then proceed.

### 6.2 Phase 1 — Routing (Directed Graph)

```
current_node = ProgramNode
command_path = [program_name]
i = 0  # index into remaining argv tokens

while i < len(argv):
  token = argv[i]

  if token == "--":
    break  # end-of-flags marker; stop routing

  if token starts with "-":
    # Skip flags during routing — they belong to Phase 2.
    # Peek ahead to skip the value if this flag takes one.
    skip_flag_token(token, i, active_flags_at(current_node))
    i += (1 or 2)
    continue

  valid_next = successors(current_node, G_cmd)
  canonical = canonical_name_for(token, valid_next)  # resolves aliases

  if canonical is not nil:
    command_path.append(canonical)
    current_node = node_for(canonical, G_cmd)
    i++
  else:
    break  # first positional-looking token that is not a subcommand
```

After routing, `current_node` is the leaf command node whose flag/argument schema
will be used for Phase 2. `command_path` records the full invocation path.

### 6.3 Phase 2 — Scanning (Modal State Machine)

Build the active flag set:
```
active_flags = global_flags (if inherit_global_flags)
             + flags of every node in command_path
             + builtin flags (help, version)
```

Construct the token classification DFA from `active_flags`.

Initialize the Modal State Machine with modes `{ROUTING, SCANNING, FLAG_VALUE,
END_OF_FLAGS}`. Start in `SCANNING` mode (routing was done in Phase 1).

```
parsed_flags = {}
positional_tokens = []
pending_flag = nil  # set when a non-boolean flag was seen

re-walk argv (skip tokens that are in command_path):

  for each token:
    classified = classify_token(token, classification_dfa)

    match mode:

    SCANNING:
      match classified:
        END_OF_FLAGS      → switch_mode(END_OF_FLAGS)
        LONG_FLAG(n)      → flag = lookup_flag_by_long(n)
                            if flag is boolean: parsed_flags[flag.id] = true
                            else: pending_flag = flag; switch_mode(FLAG_VALUE)
        LONG_FLAG_WITH_VALUE(n, v)
                          → flag = lookup_flag_by_long(n)
                            parsed_flags[flag.id] = coerce(v, flag.type)
        SINGLE_DASH_LONG(n)
                          → flag = lookup_flag_by_sdl(n)
                            if boolean: parsed_flags[flag.id] = true
                            else: pending_flag = flag; switch_mode(FLAG_VALUE)
        SHORT_FLAG(c)     → flag = lookup_flag_by_short(c)
                            if boolean: parsed_flags[flag.id] = true
                            else: pending_flag = flag; switch_mode(FLAG_VALUE)
        SHORT_FLAG_WITH_VALUE(c, v)
                          → flag = lookup_flag_by_short(c)
                            parsed_flags[flag.id] = coerce(v, flag.type)
        STACKED_FLAGS(cs) → for each char in cs:
                              flag = lookup_flag_by_short(char)
                              parsed_flags[flag.id] = true  (last may have inline value)
        POSITIONAL(v)     → if posix mode: switch_mode(END_OF_FLAGS) then push v
                            else: positional_tokens.append(v)
        UNKNOWN_FLAG      → record error, continue

    FLAG_VALUE:
      # The entire token is the value for pending_flag
      parsed_flags[pending_flag.id] = coerce(token, pending_flag.type)
      pending_flag = nil
      switch_mode(SCANNING)

    END_OF_FLAGS:
      positional_tokens.append(token)  # no classification needed
```

**Repeatable flags**: If `flag.repeatable` is `true`, `parsed_flags[flag.id]` is
an array. Each occurrence appends to it. If a non-repeatable flag appears more than
once, record a `duplicate_flag` error.

**Help and version**: If `--help` or `-h` is encountered at any point, return a
`HelpResult` immediately (do not continue scanning). If `--version` is encountered,
return a `VersionResult`.

### 6.4 Phase 3 — Validation

#### 6.4.1 Positional Argument Resolution

Given `positional_tokens` (a flat list) and the `arguments` array for the resolved
command node, assign tokens to argument slots.

```
arg_defs = current_node.arguments
variadic_idx = index of first arg where variadic=true, or -1

if variadic_idx == -1:
  # No variadic: one-to-one assignment in order
  for i, def in enumerate(arg_defs):
    if i < len(positional_tokens):
      assign coerce(positional_tokens[i], def.type) to def.id
    elif def.required and def.id not in required_unless_flag_satisfied(parsed_flags):
      record MissingArgumentError(def)
  if len(positional_tokens) > len(arg_defs):
    record TooManyArgumentsError

else:
  # Partition tokens around the variadic
  leading_defs  = arg_defs[0 .. variadic_idx-1]
  variadic_def  = arg_defs[variadic_idx]
  trailing_defs = arg_defs[variadic_idx+1 ..]

  # Assign leading (before variadic)
  for i, def in enumerate(leading_defs):
    assign positional_tokens[i] to def.id (or error if absent and required)

  # Assign trailing (after variadic) — consume from the end
  trailing_start = len(positional_tokens) - len(trailing_defs)
  for i, def in enumerate(trailing_defs):
    token_idx = trailing_start + i
    if token_idx >= len(positional_tokens) and def.required:
      record MissingArgumentError(def)
    else:
      assign positional_tokens[token_idx] to def.id

  # Variadic gets everything in between
  variadic_tokens = positional_tokens[len(leading_defs) .. trailing_start - 1]
  count = len(variadic_tokens)
  if count < variadic_def.variadic_min:
    record TooFewArgumentsError(variadic_def, count)
  if variadic_def.variadic_max is not null and count > variadic_def.variadic_max:
    record TooManyArgumentsError(variadic_def, count)
  assign [coerce(t, variadic_def.type) for t in variadic_tokens] to variadic_def.id
```

This algorithm handles the `cp`/`mv` pattern naturally:
```
cp a.txt b.txt c.txt /dest/
  leading_defs  = []           (variadic is first)
  variadic_def  = "source"
  trailing_defs = ["dest"]     (required, non-variadic, after variadic)

  trailing_start = 4 - 1 = 3
  dest     = positional_tokens[3] = "/dest/"
  variadic = positional_tokens[0..2] = ["a.txt", "b.txt", "c.txt"]
```

#### 6.4.2 Flag Constraint Validation

```
for each flag_id in parsed_flags:
  flag = flag_def(flag_id)

  # conflicts_with
  for other_id in flag.conflicts_with:
    if other_id in parsed_flags:
      record ConflictingFlagsError(flag_id, other_id)

  # requires (transitive via G_flag)
  for required_id in transitive_closure(flag_id, G_flag):
    if required_id not in parsed_flags:
      record MissingDependencyFlagError(flag_id, required_id)

# required flags
for flag_def in active_flags where required=true:
  if flag_def.id not in parsed_flags:
    exempt = any flag in flag_def.required_unless is in parsed_flags
    if not exempt:
      record MissingRequiredFlagError(flag_def)

# mutually_exclusive_groups
for group in current_node.mutually_exclusive_groups:
  present = [id for id in group.flag_ids if id in parsed_flags]
  if len(present) > 1:
    record ExclusiveGroupViolationError(group, present)
  if group.required and len(present) == 0:
    record MissingRequiredGroupError(group)
```

#### 6.4.3 Spec Validation (Load Time)

When loading a spec, before any argv is parsed:

1. Verify `cli_builder_spec_version` is `"1.0"` (or a version the implementation supports).
2. For each scope (root + every command): verify no duplicate flag `id`, command `id`,
   or argument `id`.
3. Verify every flag has at least one of `short`, `long`, or `single_dash_long`.
4. Verify all `conflicts_with` and `requires` IDs exist in the same scope or in
   `global_flags`.
5. Verify all `mutually_exclusive_groups` reference valid flag IDs in the same scope.
6. Verify `enum_values` is present and non-empty when `type` is `"enum"`.
7. Verify at most one argument per scope has `variadic: true`.
8. Build G_flag for each scope and call `has_cycle?`. If a cycle exists, report
   a spec error: circular `requires` dependency.

Spec validation errors are fatal. The library must surface them before attempting
any parse.

---

## 7. Output Format

On a successful parse, the library returns a structured result:

```json
{
  "program": "git",
  "command_path": ["git", "remote", "add"],
  "flags": {
    "verbose": false,
    "dry-run": false
  },
  "arguments": {
    "name": "origin",
    "url": "https://github.com/user/repo"
  }
}
```

**Field definitions:**

| Field | Type | Description |
|---|---|---|
| `program` | string | Always `argv[0]`. The program name as invoked. |
| `command_path` | array | Full path of commands from root to resolved leaf: `["git", "remote", "add"]`. For root-level invocation (no subcommands): `["program-name"]`. |
| `flags` | object | Map from flag `id` to parsed value. All flags in scope are present — absent optional flags use `false` for booleans, `null` for others (or `default` if set). |
| `arguments` | object | Map from argument `id` to parsed value. Variadic arguments produce arrays. |

**Special results** (returned instead of the normal object):
- **HelpResult**: Triggered by `--help` or `-h`. Contains the rendered help text for
  the deepest resolved command. The library should print it and exit 0.
- **VersionResult**: Triggered by `--version`. Contains the `version` string from the
  spec. The library should print it and exit 0.

**Error result**: If parsing fails, the library returns (or raises) an error containing
a list of `ParseError` objects (see §8). Implementations may choose whether to collect
all errors (report everything wrong at once) or fail fast (stop at the first error).
Collecting all errors is strongly preferred for usability.

---

## 8. Error Handling

### 8.1 Error Structure

Each `ParseError` contains:
- `error_type` — a snake_case string identifying the error category (machine-readable)
- `message` — a human-readable sentence explaining the error
- `suggestion` — an optional string with a corrective hint (e.g., a fuzzy match)
- `context` — the `command_path` at the point where the error was detected

### 8.2 Error Types

| Error Type | Trigger | Example Message |
|---|---|---|
| `unknown_command` | Token in subcommand position matches no known command or alias | `Unknown command 'comit'. Did you mean 'commit'?` |
| `unknown_flag` | Flag token matches no known flag in scope | `Unknown flag '--mesage'. Did you mean '--message'?` |
| `missing_required_flag` | A `required: true` flag is absent and `required_unless` is not satisfied | `--message is required for 'git commit'` |
| `missing_required_argument` | A `required: true` argument is absent and `required_unless_flag` is not satisfied | `Missing required argument: <DEST>` |
| `conflicting_flags` | Two flags that list each other in `conflicts_with` are both present | `-f/--force and -i/--interactive cannot be used together` |
| `missing_dependency_flag` | A flag is present but a flag it `requires` (directly or transitively) is absent | `-h/--human-readable requires -l/--long-listing` |
| `too_few_arguments` | A variadic argument receives fewer values than `variadic_min` | `Expected at least 1 <SOURCE>, got 0` |
| `too_many_arguments` | A variadic argument receives more values than `variadic_max`, or more positional tokens exist than argument slots | `Expected at most 1 <FILE>, got 3` |
| `invalid_value` | A value fails type coercion | `Invalid integer for --count: 'abc'` |
| `invalid_enum_value` | A value is not in `enum_values` | `Invalid value 'bork' for --format. Must be one of: json, csv, table` |
| `exclusive_group_violation` | Multiple flags in a mutually exclusive group are present | `Only one of -E/--extended-regexp, -F/--fixed-strings, -P/--perl-regexp may be used` |
| `missing_exclusive_group` | A `required: true` group has no flags present | `One of -E/--extended-regexp, -F/--fixed-strings, -P/--perl-regexp is required` |
| `duplicate_flag` | A non-repeatable flag appears more than once | `--verbose specified more than once` |
| `invalid_stack` | A stacked flag sequence contains an unknown character or a non-boolean flag in the wrong position | `Unknown flag '-X' in stack '-lXh'` |
| `spec_error` | The JSON spec itself is invalid (cycle in requires, duplicate IDs, etc.) | `Circular requires dependency: verbose → quiet → verbose` |

### 8.3 Fuzzy Matching

For `unknown_command` and `unknown_flag` errors, implementations should compute
Levenshtein edit distance between the unknown token and all valid tokens at that
scope. If the closest match has edit distance ≤ 2, include it as the `suggestion`.

---

## 9. Help Message Generation

CLI Builder auto-generates help text from the spec. The format is:

```
USAGE
  <name> [OPTIONS] [COMMAND] [ARGS...]

DESCRIPTION
  <description>

COMMANDS
  subcommand    Description of the subcommand.
  other         Description of another subcommand.

OPTIONS
  -s, --long-name <VALUE>    Description of the flag. [default: val]
  -b, --boolean              Boolean flag description.

GLOBAL OPTIONS
  -h, --help     Show this help message and exit.
  --version      Show version and exit.
```

For a subcommand (`program subcommand --help`):

```
USAGE
  <program> <subcommand> [OPTIONS] <ARG> [ARG...]

DESCRIPTION
  <subcommand description>

OPTIONS
  ...

ARGUMENTS
  <ARG>      Description. Required.
  [ARG...]   Description. Optional, repeatable.
```

**Formatting rules:**
- Required positional arguments: `<DISPLAY_NAME>`
- Optional positional arguments: `[DISPLAY_NAME]`
- Variadic required: `<DISPLAY_NAME>...`
- Variadic optional: `[DISPLAY_NAME...]`
- Non-boolean flags: `-s, --long <VALUE>` (value_name or the type name uppercased)
- Boolean flags: `-s, --long`
- `single_dash_long` flags: `-classpath <VALUE>`
- Default values appended as `[default: X]` when set and `required` is `false`

---

## 10. Examples — Unix Utilities Stress Test

The following full JSON specifications demonstrate the range of CLI patterns that
CLI Builder handles. Each example includes representative valid invocations and the
expected output or behavior.

---

### 10.1 echo — Minimal: variadic args, flag conflicts

`echo` prints its arguments. The `-e` and `-E` flags conflict (one enables
backslash interpretation, the other disables it).

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "echo",
  "description": "Display a line of text",
  "version": "8.32",
  "flags": [
    {
      "id": "no-newline",
      "short": "n",
      "description": "Do not output the trailing newline",
      "type": "boolean"
    },
    {
      "id": "enable-escapes",
      "short": "e",
      "description": "Enable interpretation of backslash escapes",
      "type": "boolean",
      "conflicts_with": ["disable-escapes"]
    },
    {
      "id": "disable-escapes",
      "short": "E",
      "description": "Disable interpretation of backslash escapes (default)",
      "type": "boolean",
      "conflicts_with": ["enable-escapes"]
    }
  ],
  "arguments": [
    {
      "id": "string",
      "display_name": "STRING",
      "description": "Text to print",
      "type": "string",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ]
}
```

| Invocation | Result |
|---|---|
| `echo hello world` | `flags: {no-newline:false, enable-escapes:false, disable-escapes:false}` `args: {string:["hello","world"]}` |
| `echo -n hello` | `flags: {no-newline:true, ...}` `args: {string:["hello"]}` |
| `echo -e -E hello` | Error: `conflicting_flags` (-e and -E) |
| `echo` | `flags: {...all false}` `args: {string:[]}` |

---

### 10.2 pwd — Flags only, mutual conflict

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "pwd",
  "description": "Print name of current/working directory",
  "version": "8.32",
  "flags": [
    {
      "id": "logical",
      "short": "L",
      "description": "Use PWD from environment, even if it contains symlinks",
      "type": "boolean",
      "conflicts_with": ["physical"]
    },
    {
      "id": "physical",
      "short": "P",
      "description": "Avoid all symlinks",
      "type": "boolean",
      "conflicts_with": ["logical"]
    }
  ]
}
```

---

### 10.3 ls — Stacking, flag requires, optional variadic

`ls -lah /tmp` is the canonical stacking example. `-h` (human-readable sizes) only
makes sense with `-l` (long listing), so it `requires` `-l`.

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "ls",
  "description": "List directory contents",
  "version": "8.32",
  "parsing_mode": "gnu",
  "flags": [
    {
      "id": "long-listing",
      "short": "l",
      "description": "Use long listing format",
      "type": "boolean",
      "conflicts_with": ["single-column"]
    },
    {
      "id": "all",
      "short": "a",
      "long": "all",
      "description": "Do not ignore entries starting with .",
      "type": "boolean"
    },
    {
      "id": "human-readable",
      "short": "h",
      "long": "human-readable",
      "description": "Print sizes like 1K 234M 2G",
      "type": "boolean",
      "requires": ["long-listing"]
    },
    {
      "id": "reverse",
      "short": "r",
      "long": "reverse",
      "description": "Reverse order while sorting",
      "type": "boolean"
    },
    {
      "id": "sort-time",
      "short": "t",
      "description": "Sort by modification time, newest first",
      "type": "boolean"
    },
    {
      "id": "recursive",
      "short": "R",
      "long": "recursive",
      "description": "List subdirectories recursively",
      "type": "boolean"
    },
    {
      "id": "single-column",
      "short": "1",
      "description": "List one file per line",
      "type": "boolean",
      "conflicts_with": ["long-listing"]
    }
  ],
  "arguments": [
    {
      "id": "path",
      "display_name": "PATH",
      "description": "Directory or file to list",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0,
      "default": "."
    }
  ]
}
```

| Invocation | Result |
|---|---|
| `ls` | All flags false, `path: ["."]` (default) |
| `ls -lah /tmp` | Stacked: `long-listing:true, all:true, human-readable:true`, `path:["/tmp"]` |
| `ls -la` | `long-listing:true, all:true`, no path → default `"."` |
| `ls -h` | Error: `missing_dependency_flag` (-h requires -l) |
| `ls -1 -l` | Error: `conflicting_flags` (-1 and -l) |

---

### 10.4 cat — Optional variadic files (stdin fallback)

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "cat",
  "description": "Concatenate files and print on the standard output",
  "version": "8.32",
  "flags": [
    {
      "id": "number-nonblank",
      "short": "b",
      "long": "number-nonblank",
      "description": "Number nonempty output lines",
      "type": "boolean"
    },
    {
      "id": "show-ends",
      "short": "E",
      "long": "show-ends",
      "description": "Display $ at end of each line",
      "type": "boolean"
    },
    {
      "id": "number",
      "short": "n",
      "long": "number",
      "description": "Number all output lines",
      "type": "boolean"
    },
    {
      "id": "squeeze-blank",
      "short": "s",
      "long": "squeeze-blank",
      "description": "Suppress repeated empty output lines",
      "type": "boolean"
    },
    {
      "id": "show-tabs",
      "short": "T",
      "long": "show-tabs",
      "description": "Display TAB characters as ^I",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "file",
      "display_name": "FILE",
      "description": "Files to concatenate. If absent or '-', reads from standard input.",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ]
}
```

Note: The convention that `-` means stdin is a runtime behavior of the tool, not
enforced by CLI Builder. The `file` argument type is `path` (not `file`) to permit
`-` and non-existent stdin placeholders.

---

### 10.5 wc — Counting flags, optional files

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "wc",
  "description": "Print newline, word, and byte counts for each file",
  "version": "8.32",
  "flags": [
    { "id": "lines",   "short": "l", "long": "lines",   "description": "Print the newline counts", "type": "boolean" },
    { "id": "words",   "short": "w", "long": "words",   "description": "Print the word counts",    "type": "boolean" },
    { "id": "bytes",   "short": "c", "long": "bytes",   "description": "Print the byte counts",    "type": "boolean" },
    { "id": "chars",   "short": "m", "long": "chars",   "description": "Print the character counts","type": "boolean" },
    { "id": "max-line-length", "short": "L", "long": "max-line-length",
      "description": "Print the maximum display width", "type": "boolean" }
  ],
  "arguments": [
    {
      "id": "file",
      "display_name": "FILE",
      "description": "Files to count. If absent, reads from standard input.",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ]
}
```

---

### 10.6 cp — Variadic sources with required trailing dest

The classic last-wins pattern. `cp src1 src2 ... dest` — the final positional
argument is always the destination.

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "cp",
  "description": "Copy files and directories",
  "version": "8.32",
  "flags": [
    { "id": "recursive",    "short": "r", "long": "recursive",    "description": "Copy directories recursively", "type": "boolean" },
    { "id": "force",        "short": "f", "long": "force",        "description": "Overwrite without prompting",  "type": "boolean",
      "conflicts_with": ["interactive", "no-clobber"] },
    { "id": "interactive",  "short": "i", "long": "interactive",  "description": "Prompt before overwrite",      "type": "boolean",
      "conflicts_with": ["force", "no-clobber"] },
    { "id": "no-clobber",   "short": "n", "long": "no-clobber",   "description": "Do not overwrite existing file","type": "boolean",
      "conflicts_with": ["force", "interactive"] },
    { "id": "verbose",      "short": "v", "long": "verbose",      "description": "Explain what is being done",   "type": "boolean" }
  ],
  "arguments": [
    {
      "id": "source",
      "display_name": "SOURCE",
      "description": "Source file(s) or directory",
      "type": "path",
      "required": true,
      "variadic": true,
      "variadic_min": 1
    },
    {
      "id": "dest",
      "display_name": "DEST",
      "description": "Destination file or directory",
      "type": "path",
      "required": true,
      "variadic": false
    }
  ]
}
```

| Invocation | Result |
|---|---|
| `cp a.txt /tmp/` | `source:["a.txt"]`, `dest:"/tmp/"` |
| `cp a.txt b.txt c.txt /dest/` | `source:["a.txt","b.txt","c.txt"]`, `dest:"/dest/"` |
| `cp a.txt` | Error: `missing_required_argument` (DEST) |
| `cp` | Error: `too_few_arguments` (SOURCE needs ≥ 1) |

`mv` uses the identical argument structure; only the flags differ.

---

### 10.7 grep — Conditional required arg, exclusive group, repeatable flag

`grep`'s `PATTERN` argument is required unless `-e` or `-f` is provided (they
supply the pattern themselves). `-E`, `-F`, and `-P` select the regex engine and
are mutually exclusive.

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "grep",
  "description": "Print lines that match patterns",
  "version": "3.7",
  "flags": [
    { "id": "ignore-case",         "short": "i", "long": "ignore-case",         "description": "Ignore case distinctions in patterns",            "type": "boolean" },
    { "id": "invert-match",        "short": "v", "long": "invert-match",        "description": "Invert the sense of matching",                    "type": "boolean" },
    { "id": "line-number",         "short": "n", "long": "line-number",         "description": "Print line number with output lines",             "type": "boolean" },
    { "id": "count",               "short": "c", "long": "count",               "description": "Print only a count of matching lines",            "type": "boolean" },
    { "id": "recursive",           "short": "r", "long": "recursive",           "description": "Read all files under each directory recursively", "type": "boolean" },
    { "id": "files-with-matches",  "short": "l", "long": "files-with-matches",  "description": "Print only names of files with matches",          "type": "boolean" },
    {
      "id": "regexp",
      "short": "e",
      "long": "regexp",
      "description": "Use PATTERN as the pattern; can be used multiple times",
      "type": "string",
      "value_name": "PATTERN",
      "repeatable": true
    },
    {
      "id": "file",
      "short": "f",
      "long": "file",
      "description": "Obtain patterns from FILE, one per line",
      "type": "file",
      "value_name": "FILE",
      "repeatable": true
    },
    { "id": "extended-regexp", "short": "E", "long": "extended-regexp", "description": "PATTERN is an extended regular expression", "type": "boolean" },
    { "id": "fixed-strings",   "short": "F", "long": "fixed-strings",   "description": "PATTERN is a set of newline-separated strings", "type": "boolean" },
    { "id": "perl-regexp",     "short": "P", "long": "perl-regexp",     "description": "PATTERN is a Perl regular expression", "type": "boolean" }
  ],
  "arguments": [
    {
      "id": "pattern",
      "display_name": "PATTERN",
      "description": "The search pattern",
      "type": "string",
      "required": true,
      "required_unless_flag": ["regexp", "file"]
    },
    {
      "id": "files",
      "display_name": "FILE",
      "description": "Files to search. If absent, reads from standard input.",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ],
  "mutually_exclusive_groups": [
    {
      "id": "regex-engine",
      "flag_ids": ["extended-regexp", "fixed-strings", "perl-regexp"],
      "required": false
    }
  ]
}
```

| Invocation | Result |
|---|---|
| `grep -i foo file.txt` | `pattern:"foo"`, `files:["file.txt"]` |
| `grep -E '^[0-9]+' *.log` | Extended mode, `pattern:"^[0-9]+"`, `files:["*.log"]` |
| `grep -e foo -e bar file.txt` | `regexp:["foo","bar"]`, pattern arg absent (ok), `files:["file.txt"]` |
| `grep -E -F pattern` | Error: `exclusive_group_violation` |
| `grep file.txt` | Error: `missing_required_argument` (PATTERN) — unless -e or -f present |

---

### 10.8 sort — Flags with string values

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "sort",
  "description": "Sort lines of text files",
  "version": "8.32",
  "flags": [
    {
      "id": "key",
      "short": "k",
      "long": "key",
      "description": "Sort via a key; KEYDEF gives location and type",
      "type": "string",
      "value_name": "KEYDEF",
      "repeatable": true
    },
    {
      "id": "field-separator",
      "short": "t",
      "long": "field-separator",
      "description": "Use SEP instead of non-blank to blank transition",
      "type": "string",
      "value_name": "SEP"
    },
    { "id": "reverse",        "short": "r", "long": "reverse",        "description": "Reverse the result of comparisons", "type": "boolean" },
    { "id": "numeric-sort",   "short": "n", "long": "numeric-sort",   "description": "Compare according to string numerical value", "type": "boolean" },
    { "id": "unique",         "short": "u", "long": "unique",         "description": "With -c, check for strict ordering; without, output only the first of equal lines", "type": "boolean" },
    { "id": "ignore-case",    "short": "f", "long": "ignore-case",    "description": "Fold lower case to upper case characters", "type": "boolean" },
    { "id": "ignore-blanks",  "short": "b", "long": "ignore-blanks",  "description": "Ignore leading blanks", "type": "boolean" }
  ],
  "arguments": [
    {
      "id": "file",
      "display_name": "FILE",
      "description": "Files to sort. If absent, reads from standard input.",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ]
}
```

---

### 10.9 head and tail — Integer-valued flags with defaults

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "head",
  "description": "Output the first part of files",
  "version": "8.32",
  "flags": [
    {
      "id": "lines",
      "short": "n",
      "long": "lines",
      "description": "Print the first NUM lines instead of the first 10",
      "type": "integer",
      "value_name": "NUM",
      "default": 10,
      "conflicts_with": ["bytes"]
    },
    {
      "id": "bytes",
      "short": "c",
      "long": "bytes",
      "description": "Print the first NUM bytes of each file",
      "type": "integer",
      "value_name": "NUM",
      "conflicts_with": ["lines"]
    },
    {
      "id": "quiet",
      "short": "q",
      "long": "quiet",
      "description": "Never print headers giving file names",
      "type": "boolean"
    },
    {
      "id": "verbose",
      "short": "v",
      "long": "verbose",
      "description": "Always print headers giving file names",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "file",
      "display_name": "FILE",
      "description": "Files to read. If absent, reads from standard input.",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ]
}
```

---

### 10.10 tar — Traditional mode, required exclusive group

`tar` is the canonical traditional-mode tool. `tar xvf archive.tar` works without
any leading dashes. The operation flag (`-c`/`-x`/`-t`) is required and they are
mutually exclusive.

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "tar",
  "description": "An archiving utility",
  "version": "1.34",
  "parsing_mode": "traditional",
  "flags": [
    { "id": "create",  "short": "c", "description": "Create a new archive",              "type": "boolean" },
    { "id": "extract", "short": "x", "description": "Extract files from an archive",     "type": "boolean" },
    { "id": "list",    "short": "t", "description": "List the contents of an archive",   "type": "boolean" },
    { "id": "verbose", "short": "v", "long": "verbose",  "description": "Verbosely list files processed", "type": "boolean" },
    {
      "id": "file",
      "short": "f",
      "long": "file",
      "description": "Use archive file or device ARCHIVE",
      "type": "path",
      "value_name": "ARCHIVE"
    },
    { "id": "gzip",    "short": "z", "long": "gzip",    "description": "Filter the archive through gzip",  "type": "boolean" },
    { "id": "bzip2",   "short": "j", "long": "bzip2",   "description": "Filter the archive through bzip2", "type": "boolean" },
    { "id": "xz",      "short": "J", "long": "xz",      "description": "Filter the archive through xz",    "type": "boolean" },
    {
      "id": "directory",
      "short": "C",
      "long": "directory",
      "description": "Change to directory DIR before performing any operations",
      "type": "directory",
      "value_name": "DIR"
    }
  ],
  "arguments": [
    {
      "id": "member",
      "display_name": "MEMBER",
      "description": "Archive members to extract or list",
      "type": "path",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ],
  "mutually_exclusive_groups": [
    {
      "id": "operation",
      "flag_ids": ["create", "extract", "list"],
      "required": true
    },
    {
      "id": "compression",
      "flag_ids": ["gzip", "bzip2", "xz"],
      "required": false
    }
  ]
}
```

| Invocation | Result |
|---|---|
| `tar xvf archive.tar` | Traditional: `extract:true, verbose:true, file:"archive.tar"` |
| `tar -czvf out.tar.gz ./src` | GNU-style: `create:true, gzip:true, verbose:true, file:"out.tar.gz"`, `member:["./src"]` |
| `tar tf archive.tar` | Traditional: `list:true, file:"archive.tar"` |
| `tar vf archive.tar` | Error: `missing_exclusive_group` (one of create/extract/list required) |
| `tar cxf archive.tar` | Error: `exclusive_group_violation` (create and extract) |

---

### 10.11 java (partial) — Single-dash-long flags

Java uses single-dash multi-character flags. `-classpath` and `-cp` are equivalent;
`-jar` changes the interpretation of the first positional argument. This demonstrates
longest-match-first: `-classpath` must not be decomposed as stacked single-char flags.

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "java",
  "description": "Launches a Java application",
  "flags": [
    {
      "id": "classpath",
      "single_dash_long": "classpath",
      "description": "Specifies a list of directories and archives to search for class files",
      "type": "string",
      "value_name": "classpath"
    },
    {
      "id": "classpath-short",
      "single_dash_long": "cp",
      "description": "Alias for -classpath",
      "type": "string",
      "value_name": "classpath",
      "conflicts_with": ["classpath"]
    },
    {
      "id": "verbose",
      "single_dash_long": "verbose",
      "description": "Enable verbose output",
      "type": "boolean"
    },
    {
      "id": "jar",
      "single_dash_long": "jar",
      "description": "Execute a program encapsulated in a JAR file",
      "type": "boolean"
    },
    {
      "id": "version",
      "single_dash_long": "version",
      "description": "Report the product version",
      "type": "boolean"
    }
  ],
  "arguments": [
    {
      "id": "class-or-jar",
      "display_name": "CLASS|JARFILE",
      "description": "The class whose main method is called, or the JAR file to execute",
      "type": "string",
      "required": false
    },
    {
      "id": "args",
      "display_name": "ARGS",
      "description": "Arguments passed to the main method",
      "type": "string",
      "required": false,
      "variadic": true,
      "variadic_min": 0
    }
  ]
}
```

| Invocation | Disambiguation |
|---|---|
| `java -classpath . Main` | Rule 1: `-classpath` matches `single_dash_long "classpath"` ✓ |
| `java -cp . Main` | Rule 1: `-cp` matches `single_dash_long "cp"` ✓ |
| `java -verbose Main` | Rule 1: `-verbose` matches `single_dash_long "verbose"` (boolean) ✓ |

---

### 10.12 git (partial) — Deep subcommands, global flags, nested routing

`git` is the canonical subcommand tool. This spec covers the most common subset.

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "git",
  "description": "The stupid content tracker",
  "version": "2.43.0",
  "parsing_mode": "subcommand_first",
  "global_flags": [
    {
      "id": "work-tree",
      "short": "C",
      "description": "Run as if git was started in PATH",
      "type": "directory",
      "value_name": "PATH",
      "repeatable": true
    },
    {
      "id": "config-env",
      "short": "c",
      "description": "Pass a configuration parameter to the command",
      "type": "string",
      "value_name": "name=value",
      "repeatable": true
    },
    {
      "id": "no-pager",
      "long": "no-pager",
      "description": "Do not pipe output into a pager",
      "type": "boolean"
    }
  ],
  "commands": [
    {
      "id": "cmd-add",
      "name": "add",
      "description": "Add file contents to the index",
      "flags": [
        { "id": "dry-run",  "short": "n", "long": "dry-run",  "description": "Dry run",                           "type": "boolean" },
        { "id": "verbose",  "short": "v", "long": "verbose",  "description": "Be verbose",                        "type": "boolean" },
        { "id": "all",      "short": "A", "long": "all",      "description": "Add all changes",                   "type": "boolean" },
        { "id": "patch",    "short": "p", "long": "patch",    "description": "Interactively choose hunks of patch","type": "boolean" }
      ],
      "arguments": [
        {
          "id": "pathspec",
          "display_name": "PATHSPEC",
          "description": "Files to add content from",
          "type": "path",
          "required": true,
          "variadic": true,
          "variadic_min": 1
        }
      ]
    },
    {
      "id": "cmd-commit",
      "name": "commit",
      "description": "Record changes to the repository",
      "flags": [
        {
          "id": "message",
          "short": "m",
          "long": "message",
          "description": "Use the given message as the commit message",
          "type": "string",
          "value_name": "MSG",
          "required": true,
          "required_unless": ["amend", "reuse-message", "fixup", "squash"]
        },
        { "id": "all",           "short": "a", "long": "all",           "description": "Stage modified and deleted files",           "type": "boolean" },
        { "id": "amend",         "long": "amend",                        "description": "Amend the previous commit",                  "type": "boolean" },
        { "id": "reuse-message", "short": "C", "long": "reuse-message",  "description": "Reuse log message and authorship",           "type": "string", "value_name": "COMMIT" },
        { "id": "fixup",         "long": "fixup",                        "description": "Create fixup commit for specified commit",    "type": "string", "value_name": "COMMIT" },
        { "id": "squash",        "long": "squash",                       "description": "Create squash commit for specified commit",   "type": "string", "value_name": "COMMIT" },
        { "id": "verbose",       "short": "v", "long": "verbose",        "description": "Show diff in the commit message editor",     "type": "boolean" }
      ]
    },
    {
      "id": "cmd-push",
      "name": "push",
      "description": "Update remote refs along with associated objects",
      "flags": [
        { "id": "force",             "short": "f", "long": "force",             "description": "Force updates",                           "type": "boolean" },
        { "id": "force-with-lease",  "long": "force-with-lease",                "description": "Force update only if tip matches expected", "type": "boolean" },
        { "id": "verbose",           "short": "v", "long": "verbose",           "description": "Run verbosely",                           "type": "boolean" },
        { "id": "tags",              "long": "tags",                             "description": "Push all refs under refs/tags",            "type": "boolean" },
        { "id": "dry-run",           "short": "n", "long": "dry-run",           "description": "Do everything except actually send",      "type": "boolean" }
      ],
      "arguments": [
        {
          "id": "remote",
          "display_name": "REMOTE",
          "description": "The remote to push to",
          "type": "string",
          "required": false
        },
        {
          "id": "refspec",
          "display_name": "REFSPEC",
          "description": "Refs to push",
          "type": "string",
          "required": false,
          "variadic": true,
          "variadic_min": 0
        }
      ]
    },
    {
      "id": "cmd-remote",
      "name": "remote",
      "description": "Manage set of tracked repositories",
      "flags": [
        { "id": "verbose", "short": "v", "long": "verbose", "description": "Be verbose", "type": "boolean" }
      ],
      "commands": [
        {
          "id": "cmd-remote-add",
          "name": "add",
          "description": "Add a remote named NAME for the repository at URL",
          "flags": [
            { "id": "fetch",  "short": "f", "description": "After setup, fetch from the remote",  "type": "boolean" },
            { "id": "tags",   "long": "tags",   "description": "Import every tag",                "type": "boolean" },
            { "id": "no-tags","long": "no-tags","description": "Do not import tags automatically", "type": "boolean",
              "conflicts_with": ["tags"] }
          ],
          "arguments": [
            { "id": "name", "display_name": "NAME", "description": "Name for the remote", "type": "string", "required": true },
            { "id": "url",  "display_name": "URL",  "description": "URL of the remote",   "type": "string", "required": true }
          ]
        },
        {
          "id": "cmd-remote-remove",
          "name": "remove",
          "aliases": ["rm"],
          "description": "Remove the remote named NAME",
          "arguments": [
            { "id": "name", "display_name": "NAME", "description": "Name of the remote to remove", "type": "string", "required": true }
          ]
        },
        {
          "id": "cmd-remote-rename",
          "name": "rename",
          "description": "Rename the remote named OLD to NEW",
          "arguments": [
            { "id": "old", "display_name": "OLD", "description": "Current remote name", "type": "string", "required": true },
            { "id": "new", "display_name": "NEW", "description": "New remote name",     "type": "string", "required": true }
          ]
        },
        {
          "id": "cmd-remote-set-url",
          "name": "set-url",
          "description": "Change URLs for the remote",
          "arguments": [
            { "id": "name", "display_name": "NAME", "description": "Remote name",  "type": "string", "required": true },
            { "id": "url",  "display_name": "URL",  "description": "New URL",      "type": "string", "required": true }
          ]
        }
      ]
    }
  ]
}
```

| Invocation | command_path |
|---|---|
| `git add src/foo.rb` | `["git", "add"]` |
| `git commit -m "fix bug"` | `["git", "commit"]` |
| `git remote add origin https://...` | `["git", "remote", "add"]` |
| `git remote rm origin` | `["git", "remote", "remove"]` (alias resolved) |
| `git -C /repo push --force` | Global flag `-C`, then `["git", "push"]` |
| `git comit` | Error: `unknown_command 'comit'. Did you mean 'commit'?` |

---

### 10.13 docker run (partial) — Repeatable flags with values

`docker run` has many repeatable flags (`-p`, `-e`, `-v`) that accumulate into
arrays. This is one of the most complex real-world CLI patterns.

```json
{
  "cli_builder_spec_version": "1.0",
  "name": "docker",
  "description": "A self-sufficient runtime for containers",
  "parsing_mode": "subcommand_first",
  "commands": [
    {
      "id": "cmd-run",
      "name": "run",
      "description": "Run a command in a new container",
      "flags": [
        { "id": "detach",      "short": "d", "long": "detach",  "description": "Run container in background", "type": "boolean" },
        { "id": "interactive", "short": "i", "long": "interactive", "description": "Keep STDIN open",         "type": "boolean" },
        { "id": "tty",         "short": "t", "long": "tty",         "description": "Allocate a pseudo-TTY",   "type": "boolean" },
        { "id": "rm",          "long": "rm",                         "description": "Remove container on exit","type": "boolean" },
        {
          "id": "publish",
          "short": "p",
          "long": "publish",
          "description": "Publish a container's port to the host",
          "type": "string",
          "value_name": "HOST:CONTAINER",
          "repeatable": true
        },
        {
          "id": "env",
          "short": "e",
          "long": "env",
          "description": "Set environment variables",
          "type": "string",
          "value_name": "KEY=VALUE",
          "repeatable": true
        },
        {
          "id": "volume",
          "short": "v",
          "long": "volume",
          "description": "Bind mount a volume",
          "type": "string",
          "value_name": "HOST:CONTAINER[:OPTIONS]",
          "repeatable": true
        },
        {
          "id": "name",
          "long": "name",
          "description": "Assign a name to the container",
          "type": "string",
          "value_name": "NAME"
        },
        {
          "id": "network",
          "long": "network",
          "description": "Connect a container to a network",
          "type": "string",
          "value_name": "NETWORK"
        }
      ],
      "arguments": [
        {
          "id": "image",
          "display_name": "IMAGE",
          "description": "The container image to run",
          "type": "string",
          "required": true
        },
        {
          "id": "command",
          "display_name": "COMMAND",
          "description": "Command to run in the container",
          "type": "string",
          "required": false,
          "variadic": true,
          "variadic_min": 0
        }
      ]
    }
  ]
}
```

| Invocation | Result |
|---|---|
| `docker run nginx` | `image:"nginx"`, all flags at defaults |
| `docker run -d -p 8080:80 --name web nginx` | `detach:true, publish:["8080:80"], name:"web", image:"nginx"` |
| `docker run -it --rm ubuntu bash` | `interactive:true, tty:true, rm:true, image:"ubuntu", command:["bash"]` |
| `docker run -e FOO=1 -e BAR=2 alpine` | `env:["FOO=1","BAR=2"], image:"alpine"` |

---

## 11. Stress Test Analysis

### 11.1 What CLI Builder Handles Well (≈85% of tools)

**Simple tools:** echo, pwd, ls, cat, wc, sort, head, tail, cut, touch, mkdir,
rm, chmod, chown — all flag variations, optional args, default values.

**Source/destination tools:** cp, mv, ln — variadic sources plus required trailing
dest handled by the positional resolution algorithm.

**Pattern-matching tools:** grep, awk (partial), sed (partial) — required pattern
args, conditional requirements via `required_unless_flag`.

**Subcommand tools:** git, docker, npm, cargo, kubectl, gh, brew — any depth of
subcommand nesting; per-subcommand flag scopes; global flags.

**Tools with exclusive flag groups:** tar (operation), grep (regex engine),
curl (method), openssl (command).

**Traditional-mode tools:** tar — stacked flags without leading dash.

**Single-dash-long tools:** java, xterm, ffmpeg — `single_dash_long` with
longest-match-first disambiguation.

**Repeatable flags:** docker (-p, -e, -v), git (-c), curl (-H) — accumulate
into arrays.

### 11.2 What Fits with Caveats (🟡)

**Conditional required args** (`grep -e` making PATTERN optional): Supported via
`required_unless_flag`. Limitation: the condition is "any of these flags is present"
— more complex conditions (e.g., "required unless -e or -f and not running on stdin")
are not expressible.

**Two-argument required-after-variadic** (`cp`, `mv`): Supported by the positional
resolution algorithm's last-wins logic. Only works when the trailing non-variadic
args are all required — optional trailing args after a variadic create ambiguity.

**Context-sensitive defaults** (`head -n 10` default vs. `head -c` implying bytes):
Supported at the individual flag level via `default`. Cross-flag conditional defaults
(e.g., "default for -n is 10 unless -c is given, then -n has no default") are not
expressible in the spec format; they must be handled in the tool's business logic.

### 11.3 What CLI Builder Cannot Model (❌)

**`find` expression predicates.** `find . -name "*.txt" -type f -mtime +1 -exec
rm {} \;` is not a regular CLI structure — it is a context-free grammar embedded
in argv. `-and`, `-or`, `-not`, and parentheses form an expression tree. No flat
flag/argument schema can capture this.

**`dd` key=value syntax.** `dd if=input.img of=/dev/sda bs=512 count=1000` uses
free-form `key=value` tokens rather than `-flag value` or positional args. The
set of valid keys and their types cannot be expressed in the CLI Builder spec
format.

**`ssh` host path arguments.** `scp user@host:/path/to/file local/` requires
context-sensitive parsing of a structured value within a positional argument. This
is a value format problem, not a CLI structure problem.

**`awk` and `sed` programs.** The first argument to awk is an AWK program; the
first to sed is a sed script. These are full embedded languages passed as strings.
CLI Builder can model the fact that the first argument is a `string`, but not
validate or interpret its contents.

**Order-sensitive flags that change subsequent arg meaning.** Some tools change
their behavior based on flag order — flags accumulate into a pipeline. This requires
stateful execution during parsing, not just validation after parsing.

### 11.4 Coverage Estimate

Based on analyzing the GNU coreutils (71 utilities), the Git command set (170+
subcommands), and Docker CLI (50+ subcommands):

| Category | Count | CLI Builder support |
|---|---|---|
| Flag-only tools | ~15 | ✅ Full |
| Tools with optional variadic args | ~25 | ✅ Full |
| Tools with subcommands | ~20 | ✅ Full |
| Tools with exclusive groups | ~10 | ✅ Full |
| Tools with traditional mode | ~5 | ✅ Full |
| Tools with single-dash-long | ~5 | ✅ Full |
| Tools with conditional required | ~8 | 🟡 Partial |
| Tools with embedded DSLs | ~4 | ❌ Out of scope |
| Tools with key=value args | ~3 | ❌ Out of scope |

**Estimated coverage: 85–90% of CLI tools people typically build.**

---

## 12. Spec Validation Reference

When loading a spec, an implementation must check all of the following. If any
check fails, raise a `spec_error` and refuse to parse argv.

| Rule | Check |
|---|---|
| Version | `cli_builder_spec_version` is `"1.0"` (or a supported version) |
| Required fields | `name` and `description` are present and non-empty strings |
| Unique IDs | No two flags share an `id` within the same scope; same for commands and arguments |
| Unique names | No two commands share a `name` or `alias` among siblings |
| Flag presence | Every flag has at least one of `short`, `long`, or `single_dash_long` |
| Enum values | Every flag or argument with `type: "enum"` has a non-empty `enum_values` array |
| Variadic count | At most one argument per scope has `variadic: true` |
| Reference validity | All IDs in `conflicts_with`, `requires`, `required_unless`, `required_unless_flag`, and `flag_ids` refer to flags that exist in the same scope or in `global_flags` |
| Exclusive group validity | All `flag_ids` in `mutually_exclusive_groups` refer to flags in the same scope |
| No circular requires | `has_cycle?(G_flag)` returns `false` for every scope |

---

## 13. Future Work

The following features are explicitly deferred to a future version of the spec:

**v1.1:**
- `env_var` field on flags and arguments — read default from an environment variable
  (e.g., `GREP_OPTIONS`, `GIT_EDITOR`)
- `min` / `max` constraints on `integer` and `float` types
- `pattern` constraint on `string` type (regex validation of the value)

**v2.0:**
- Shell completion metadata — export the spec in a form consumable by bash/zsh
  completion generators
- Config file fallback — read defaults from `~/.toolrc` or `XDG_CONFIG_HOME`
  before applying built-in defaults
- Argument value formats — structured argument types like `HOST:PORT`,
  `KEY=VALUE`, `user@host` without requiring business logic to parse them
- Conditional subcommand visibility — hide subcommands based on environment
  (e.g., admin-only commands)
