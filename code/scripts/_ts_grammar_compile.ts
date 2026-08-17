// _ts_grammar_compile.ts — thin wrapper invoked by generate-compiled-grammars.sh
// to compile a grammar file using the TypeScript grammar-tools library.
//
// Usage:
//   vite-node _ts_grammar_compile.ts tokens <input.tokens> <output.ts> [--force]
//   vite-node _ts_grammar_compile.ts grammar <input.grammar> <output.ts> [--force]
//
// This bypasses the main-module guard in index.ts that is incompatible with
// vite-node's argv handling.
import { compileTokensCommand, compileGrammarCommand } from "../programs/typescript/grammar-tools/index.ts";

const [,, mode, input, output, ...rest] = process.argv;
const force = rest.includes("--force") || rest.includes("-f");
if (mode === "tokens") {
    process.exit(compileTokensCommand(input, output, force));
} else if (mode === "grammar") {
    process.exit(compileGrammarCommand(input, output, force));
} else {
    process.stderr.write(`Usage: _ts_grammar_compile.ts tokens|grammar <input> <output> [--force]\n`);
    process.exit(2);
}
