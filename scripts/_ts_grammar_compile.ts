// _ts_grammar_compile.ts — thin wrapper invoked by generate-compiled-grammars.sh
// to compile a grammar file using the TypeScript grammar-tools library.
//
// Usage:
//   vite-node _ts_grammar_compile.ts tokens <input.tokens> <output.ts>
//   vite-node _ts_grammar_compile.ts grammar <input.grammar> <output.ts>
//
// This bypasses the main-module guard in index.ts that is incompatible with
// vite-node's argv handling.
import { compileTokensCommand, compileGrammarCommand } from "../code/programs/typescript/grammar-tools/index.ts";

const [,, mode, input, output] = process.argv;
if (mode === "tokens") {
    process.exit(compileTokensCommand(input, output));
} else if (mode === "grammar") {
    process.exit(compileGrammarCommand(input, output));
} else {
    process.stderr.write(`Usage: _ts_grammar_compile.ts tokens|grammar <input> <output>\n`);
    process.exit(2);
}
