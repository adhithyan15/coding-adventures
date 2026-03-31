# Browser-Compatible Grammar Loading

## Problem

All TypeScript lexer and parser packages load `.tokens` and `.grammar` files from the
filesystem at runtime using Node.js `readFileSync()`. This means:

1. **No browser support** — `fs`, `path`, and `url` modules don't exist in browsers.
2. **Deployment coupling** — packages must ship alongside the `code/grammars/` directory.
3. **Redundant parsing** — grammars are re-parsed on every invocation (no caching).
4. **Deep dependency chains** — each lexer depends on `grammar-tools` at runtime just
   to call `parseTokenGrammar()`/`parseParserGrammar()`, which pulls in `directed-graph`,
   `state-machine`, etc.

The compiled-grammars spec (PR 3) already generated `_grammar.ts` files containing
pre-parsed `TokenGrammar` and `ParserGrammar` objects as TypeScript literals. But no
package actually imports them — they're orphaned artifacts.

The lattice-transpiler worked around this by creating a `browser.ts` entry point that
embeds grammar strings as constants. This is a band-aid: it duplicates the grammar text,
requires manual sync when grammars change, and doesn't fix the underlying packages.

## Solution

Switch each lexer/parser to import its compiled `_grammar.ts` directly instead of
reading from the filesystem. This is the "PR 4" work described in the compiled-grammars
spec.

### What changes per package

**Lexer packages** (sql-lexer, lattice-lexer):

Before:
```typescript
import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { parseTokenGrammar } from "@coding-adventures/grammar-tools";
import { grammarTokenize } from "@coding-adventures/lexer";

const __dirname = dirname(fileURLToPath(import.meta.url));
const TOKENS_PATH = join(__dirname, "../../../../grammars/sql.tokens");

export function tokenizeSQL(source: string): Token[] {
  const text = readFileSync(TOKENS_PATH, "utf-8");
  const grammar = parseTokenGrammar(text);
  return grammarTokenize(source, grammar);
}
```

After:
```typescript
import { grammarTokenize } from "@coding-adventures/lexer";
import { TOKEN_GRAMMAR } from "./_grammar.js";

export function tokenizeSQL(source: string): Token[] {
  return grammarTokenize(source, TOKEN_GRAMMAR);
}
```

**Parser packages** (sql-parser, lattice-parser):

Before:
```typescript
import { readFileSync } from "fs";
import { fileURLToPath } from "url";
import { dirname, join } from "path";
import { parseParserGrammar } from "@coding-adventures/grammar-tools";
import { GrammarParser } from "@coding-adventures/parser";

const __dirname = dirname(fileURLToPath(import.meta.url));
const GRAMMAR_PATH = join(__dirname, "../../../../grammars/sql.grammar");

export function parseSQL(source: string): ASTNode {
  const tokens = tokenizeSQL(source);
  const text = readFileSync(GRAMMAR_PATH, "utf-8");
  const grammar = parseParserGrammar(text);
  const parser = new GrammarParser(tokens, grammar);
  return parser.parse();
}
```

After:
```typescript
import { GrammarParser } from "@coding-adventures/parser";
import { PARSER_GRAMMAR } from "./_grammar.js";

export function parseSQL(source: string): ASTNode {
  const tokens = tokenizeSQL(source);
  const parser = new GrammarParser(tokens, PARSER_GRAMMAR);
  return parser.parse();
}
```

### Lattice transpiler cleanup

Once lattice-lexer and lattice-parser use `_grammar.ts` directly, the
lattice-transpiler's `browser.ts` workaround becomes unnecessary. The main
`index.ts` entry point works in all environments. Remove `browser.ts` and
update documentation to remove the "browser entry point" instructions.

### Dependency simplification

After this change, the runtime dependency on `grammar-tools` is eliminated for
these packages. The `_grammar.ts` files use `import type { TokenGrammar }` from
grammar-tools (erased at runtime), so grammar-tools is only needed at compile
time for type checking. However, we keep it in `dependencies` (not
`devDependencies`) because TypeScript consumers need the types to resolve.

The `fs`, `path`, and `url` Node.js built-in imports are completely removed.

## Scope

### This PR (sql + lattice)

| Package | Change |
|---------|--------|
| sql-lexer | Import `TOKEN_GRAMMAR` from `_grammar.ts`, remove `readFileSync` |
| sql-parser | Import `PARSER_GRAMMAR` from `_grammar.ts`, remove `readFileSync` |
| lattice-lexer | Import `TOKEN_GRAMMAR` from `_grammar.ts`, remove `readFileSync` |
| lattice-parser | Import `PARSER_GRAMMAR` from `_grammar.ts`, remove `readFileSync` |
| lattice-transpiler | Remove `browser.ts`, update docs — main entry now works everywhere |
| sql-execution-engine | No code changes — already browser-compatible (just calls `parseSQL`) |

### Future work (same pattern, separate PRs)

25 TypeScript lexer/parser packages have `_grammar.ts` files. The same
transformation applies to all of them: excel, javascript, json, python, ruby,
starlark, toml, typescript, verilog, vhdl, xml.

## Implementation order

1. **sql-lexer** — Switch tokenizer.ts to import `_grammar.ts`, remove fs/path/url
2. **sql-parser** — Switch parser.ts to import `_grammar.ts`, remove fs/path/url
3. **lattice-lexer** — Same transformation
4. **lattice-parser** — Same transformation
5. **lattice-transpiler** — Remove browser.ts, update index.ts docs
6. **Tests** — Run all test suites to verify nothing breaks
7. **CHANGELOGs** — Update all 5 packages
8. **READMEs** — Update any browser-specific instructions
9. **Spec sync** — Update compiled-grammars.md to note PR 4 is done for these packages

## Testing

Existing tests should pass unchanged — the public API (`tokenizeSQL`,
`parseSQL`, `parseLattice`, `transpileLattice`) remains identical. The only
difference is where the grammar object comes from (compiled import vs runtime
file read).

## Non-goals

- Changing the grammar file format or compilation step
- Removing grammar-tools as a dependency (still needed for types)
- Converting all 25 lexer/parser packages (future PRs)
- Adding new browser-specific test infrastructure
