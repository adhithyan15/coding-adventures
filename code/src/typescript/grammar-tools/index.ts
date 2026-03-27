/**
 * grammar-tools — Define and validate .tokens and .grammar file formats.
 *
 * This package provides parsers and validators for two declarative file
 * formats used to describe programming language syntax:
 *
 * - **.tokens files** define the lexical grammar (what tokens exist)
 * - **.grammar files** define the syntactic grammar in EBNF (how tokens
 *   combine into valid programs)
 *
 * Together, these files provide a complete, language-agnostic description
 * of a programming language's surface syntax that can be used to generate
 * lexers and parsers for any target language.
 */

// Token grammar
export type { TokenDefinition, PatternGroup, TokenGrammar } from "./token-grammar.js";
export {
  TokenGrammarError,
  parseTokenGrammar,
  validateTokenGrammar,
  tokenNames,
  effectiveTokenNames,
} from "./token-grammar.js";

// Parser grammar
export type {
  RuleReference,
  TokenReference,
  Literal,
  Group,
  Optional,
  Repetition,
  Alternation,
  Sequence,
  GrammarElement,
  GrammarRule,
  ParserGrammar,
} from "./parser-grammar.js";
export {
  ParserGrammarError,
  parseParserGrammar,
  validateParserGrammar,
  ruleNames,
  grammarTokenReferences,
  grammarRuleReferences,
} from "./parser-grammar.js";

// Cross-validation
export { crossValidate } from "./cross-validator.js";

// Compiler
export { compileTokenGrammar, compileParserGrammar } from "./compiler.js";
