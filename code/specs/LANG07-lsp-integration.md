# LANG07 — LSP Integration: Editor Intelligence for Every Language

## Overview

Any language built on the LANG pipeline gets a high-performance Language Server
Protocol (LSP) implementation with minimal effort.  The existing specs `LS00`
(generic LSP server framework) and `LS01` (language bridge) define the
infrastructure.  This spec defines the **integration points** that make a
LANG-pipeline language plug into that infrastructure in under 50 lines.

A fully featured LSP server provides:

- **Diagnostics** — red/yellow squiggles as you type, reported from the parser
- **Semantic tokens** — accurate syntax highlighting beyond regex patterns
- **Hover** — type and documentation on cursor hover
- **Go to Definition** — jump to where a function or variable is defined
- **Find References** — find all uses of a symbol
- **Rename** — rename across the whole file or project
- **Auto-complete** — context-aware completions
- **Code actions** — quick-fix suggestions for errors
- **Inlay hints** — inline type annotations for dynamically typed languages
- **Signature help** — function parameter hints as you type a call

---

## What the language must provide

A language built on the LANG pipeline already has a lexer, parser, and
(optionally) a type checker.  The LSP bridge (LS01) wraps these three
components:

```python
from ls01 import LSLanguageBridge
from my_lang_lexer import tokenise
from my_lang_parser import parse
from my_lang_type_checker import infer  # optional

bridge = LSLanguageBridge(
    tokenise=tokenise,               # str → list[Token]
    parse=parse,                     # list[Token] → AST | ParseError
    infer=infer,                     # AST → TypedAST (optional; None = untyped)
    language_id="mylang",            # must match VS Code language ID
    file_extensions=[".ml", ".my"],
)
```

That is the entire language-specific contribution.  The LS00 framework handles
all protocol boilerplate, incremental re-parsing, document state, and feature
dispatch.

---

## Performance requirements

An LSP server that is slow is worse than no LSP server — a 500ms hover delay
destroys the editing experience.  The LANG LSP integration is designed for
sub-100ms response times on files up to 10,000 lines.

### Incremental parsing

The LS00 framework maintains a **document cache** per open file:

```
Document cache entry:
  source_text: str          (current document content)
  tokens: list[Token]       (from last tokenise() call)
  ast: AST                  (from last parse() call)
  typed_ast: TypedAST       (from last infer() call, if available)
  version: int              (LSP document version counter)
```

On each edit, the framework calls `tokenise` only over the changed region
(using the `contentChanges` delta from the LSP `didChange` notification)
and merges the new tokens into the cached token list.  A full re-parse runs
only when the changed region crosses a statement boundary.

This keeps re-parse cost proportional to the size of the edit, not the size
of the file.

### Type-checker latency budget

Type checking is the most expensive phase.  The framework runs it on a
**background thread** with a 200ms debounce:

```
User types a character
  → tokenise (< 5ms for 1000 lines) — on keystroke thread
  → parse    (< 20ms for 1000 lines) — on keystroke thread
  → schedule type check in 200ms (debounced)
  → ...user types more characters (debounce resets)...
  → 200ms of silence → infer() runs on background thread
  → diagnostics pushed to editor
```

This pattern ensures the editor never stalls waiting for type inference.
Completions and hovers that need type information use the most recent typed
AST, which may be slightly stale — a deliberate trade-off.

---

## Feature implementation map

| LSP feature | Source in LANG pipeline |
|-------------|------------------------|
| Diagnostics | Parser error list + type checker error list |
| Semantic tokens | Lexer `Token.type` mapped to LSP token types |
| Hover | Type checker symbol table: symbol at cursor → type string |
| Go to Definition | Type checker: name resolution → `IIRFunction.name` → source location |
| Find References | Type checker symbol table: reverse lookup |
| Rename | Find References + text edit on each location |
| Auto-complete | Parser: list of names in scope at cursor position |
| Signature help | Type checker: function type at call site → parameter names + types |
| Inlay hints | Type checker: inferred type of each expression → annotation |

Languages without a type checker get diagnostics, semantic tokens, and basic
completion (names in scope).  Languages with a type checker get the full set.

---

## Semantic token mapping

The lexer produces typed tokens (e.g., `Token(type="KEYWORD", value="fn")`).
The LS01 bridge must declare a mapping from language token types to LSP semantic
token types:

```python
bridge = LSLanguageBridge(
    ...
    semantic_token_map={
        "KEYWORD":    "keyword",
        "IDENT":      "variable",
        "FN_NAME":    "function",
        "NUMBER":     "number",
        "STRING":     "string",
        "COMMENT":    "comment",
        "TYPE_NAME":  "type",
        "PARAM_NAME": "parameter",
    }
)
```

This mapping is declared once per language; the framework handles the LSP
`textDocument/semanticTokens/full` encoding automatically.

---

## Go to Definition via IIRModule

When the language has a bytecode compiler, Definition lookup can also work
through the compiled module:

```
User: Go to Definition of "add" at call site
  → type checker looks up "add" in symbol table → not found (untyped language)
  → fall back: IIRModule.functions → find IIRFunction(name="add")
  → sidecar: IIRFunction "add" → compiled from source lines 1–5
  → editor jumps to line 1
```

This fallback means even dynamically typed languages without full type checkers
get Go to Definition — as long as they have a bytecode compiler that emits
a named `IIRFunction`.

---

## Workspace-wide features

Rename and Find References work across the entire project by running the
bridge's `tokenise` + `parse` + `infer` pipeline on every file in the workspace
in parallel.  The LS00 framework manages the workspace index and invalidates
per-file on save.

For large projects (>500 files), the workspace index is built lazily: only
files that have been opened or that are transitively imported by an open file
are indexed eagerly.

---

## VSCode extension

The same generic `coding-adventures-lsp` extension serves all languages.  It
launches the correct language server based on the file extension:

```json
// package.json contributes
"languages": [
  { "id": "tetrad", "extensions": [".tet"] },
  { "id": "basic",  "extensions": [".bas", ".basic"] },
  { "id": "mylang", "extensions": [".ml"] }
],
"languageServerCommand": "coding-adventures-lsp --language ${languageId}"
```

The server executable is a thin dispatcher that instantiates the correct
`LSLanguageBridge` and hands it to the LS00 framework.

---

## Integration with vm-core (runtime diagnostics)

When `vm-core` encounters a runtime error (division by zero, stack overflow,
type mismatch), it can push a **runtime diagnostic** to the LSP client:

```python
vm.on_runtime_error(lambda err, frame:
    lsp_server.push_diagnostic(
        file=sidecar.offset_to_source(frame.ip).file,
        line=sidecar.offset_to_source(frame.ip).line,
        message=str(err),
        severity="error",
        source="runtime",
    )
)
```

This creates a live feedback loop: errors encountered during execution appear
as editor squiggles in real time, even for dynamically typed languages that
cannot catch them statically.

---

## Package additions

| Package | Addition |
|---------|----------|
| `ls00` (LS00) | Incremental re-parse, background type check thread, document cache |
| `ls01` (LS01) | `LSLanguageBridge` semantic token map; IIRModule fallback for Definition |
| Language packages | 10–50 lines: instantiate `LSLanguageBridge` with lexer/parser/infer |
