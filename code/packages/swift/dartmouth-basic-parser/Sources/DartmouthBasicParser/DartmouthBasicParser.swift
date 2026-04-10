// ============================================================================
// DartmouthBasicParser.swift -- Dartmouth BASIC (1964) Grammar-Driven Parser
// ============================================================================
//
// This module parses Dartmouth BASIC source text into a generic ASTNode tree
// using the grammar-driven GrammarParser engine and the rules in
// dartmouth_basic.grammar.
//
// ============================================================================
// WHAT IS A GRAMMAR-DRIVEN PARSER?
// ============================================================================
//
// A traditional parser is hand-coded: the programmer writes one Swift function
// per grammar rule, and those functions call each other. This is called
// "recursive descent" parsing. It works, but every language needs its own
// carefully crafted code.
//
// A grammar-driven parser does something more powerful: it reads a text file
// that *describes* the grammar (dartmouth_basic.grammar), and interprets those
// rules at runtime. The same Swift engine that parses BASIC can also parse
// JSON, Python, or any language — just swap the .grammar file.
//
// The grammar file uses EBNF (Extended Backus-Naur Form) notation:
//
//   program = { line } ;
//   line = LINE_NUM [ statement ] NEWLINE ;
//   statement = let_stmt | print_stmt | ... ;
//   let_stmt = "LET" variable EQ expr ;
//   expr = term { ( PLUS | MINUS ) term } ;
//
// The GrammarParser reads these rules and builds a packrat memo table that
// efficiently matches the token stream. It uses Warth's algorithm to handle
// left-recursive rules without infinite loops.
//
// ============================================================================
// HOW DOES THE AST LOOK?
// ============================================================================
//
// For the program:
//   10 LET X = 5 + 3
//   20 END
//
// The AST looks like this (simplified):
//
//   ASTNode("program")
//   ├── ASTNode("line")
//   │   ├── Token(LINE_NUM, "10")
//   │   ├── ASTNode("statement")
//   │   │   └── ASTNode("let_stmt")
//   │   │       ├── Token(KEYWORD, "LET")
//   │   │       ├── ASTNode("variable")
//   │   │       │   └── Token(NAME, "x")
//   │   │       ├── Token(EQ, "=")
//   │   │       └── ASTNode("expr")
//   │   │           ├── ASTNode("term")
//   │   │           │   └── ASTNode("power")
//   │   │           │       └── ASTNode("unary")
//   │   │           │           └── ASTNode("primary")
//   │   │           │               └── Token(NUMBER, "5")
//   │   │           ├── Token(PLUS, "+")
//   │   │           └── ASTNode("term")
//   │   │               └── ... (wrapping NUMBER "3")
//   │   └── Token(NEWLINE, "\n")
//   └── ASTNode("line")
//       ├── Token(LINE_NUM, "20")
//       ├── ASTNode("statement")
//       │   └── ASTNode("end_stmt")
//       │       └── Token(KEYWORD, "END")
//       └── Token(NEWLINE, "\n")
//
// Each ASTNode carries a `ruleName` (the grammar rule that matched it),
// `children` (a mix of sub-nodes and leaf tokens), and source position info.
//
// ============================================================================
// EXPRESSION PRECEDENCE IN DARTMOUTH BASIC
// ============================================================================
//
// BASIC follows standard mathematical operator precedence, encoded as a
// cascade of grammar rules (lowest-to-highest binding):
//
//   1. expr  — addition (+) and subtraction (−), LEFT-associative
//              "1 + 2 + 3" → ((1 + 2) + 3)
//
//   2. term  — multiplication (*) and division (/), LEFT-associative
//              "2 * 3 * 4" → ((2 * 3) * 4)
//
//   3. power — exponentiation (^), RIGHT-associative
//              "2 ^ 3 ^ 2" → 2 ^ (3 ^ 2) = 512  (not (2^3)^2 = 64)
//              Right-associativity is modelled by the grammar rule:
//                power = unary [ CARET power ] ;
//              The recursive `power` on the right side chains right-to-left.
//
//   4. unary — unary minus (−)
//              "-X" → negate X
//
//   5. primary — atoms: numbers, function calls, variables, parenthesised exprs
//
// Truth table for operator priority (higher = binds tighter):
//
//   ^ (exponentiation)   : 5  (highest, right-associative)
//   - (unary minus)      : 4
//   * /                  : 3
//   + -                  : 2  (lowest arithmetic)
//   = < > <= >= <>       : 1  (comparison, only in IF)
//
// ============================================================================
// THE 17 STATEMENT TYPES
// ============================================================================
//
// Dartmouth BASIC 1964 has exactly 17 statement types:
//
//   1. LET     — variable assignment: LET X = expr
//   2. PRINT   — output: PRINT [list]
//   3. INPUT   — read from user: INPUT var, var, ...
//   4. IF      — conditional: IF expr relop expr THEN line_num
//   5. GOTO    — unconditional jump: GOTO line_num
//   6. GOSUB   — subroutine call: GOSUB line_num
//   7. RETURN  — return from subroutine: RETURN
//   8. FOR     — counted loop header: FOR var = from TO to [STEP step]
//   9. NEXT    — loop footer: NEXT var
//  10. END     — normal program end
//  11. STOP    — halt with message
//  12. REM     — remark/comment (content already suppressed by lexer)
//  13. READ    — read from DATA pool: READ var, var, ...
//  14. DATA    — literal data pool: DATA num, num, ...
//  15. RESTORE — reset DATA pointer: RESTORE
//  16. DIM     — array dimensioning: DIM name(size), ...
//  17. DEF     — define user function: DEF FNx(param) = expr
//
// ============================================================================
// THE LINE_NUM vs NUMBER DISAMBIGUATION
// ============================================================================
//
// The grammar rules `goto_stmt`, `gosub_stmt`, and `if_stmt` all reference
// `LINE_NUM` as the jump target token:
//
//   goto_stmt  = "GOTO"  LINE_NUM ;
//   gosub_stmt = "GOSUB" LINE_NUM ;
//   if_stmt    = "IF" expr relop expr "THEN" LINE_NUM ;
//
// However, the lexer's `relabelLineNumbers` pass only promotes the FIRST
// NUMBER on each source line to LINE_NUM. Jump targets like:
//
//   10 GOTO 50         — the "50" is mid-line, lexer emits NUMBER("50")
//   20 IF X > 0 THEN 100  — "100" is mid-line, lexer emits NUMBER("100")
//
// are emitted as NUMBER, not LINE_NUM.
//
// To bridge this gap, the parser registers a `relabelJumpTargets` pre-parse
// hook via `GrammarParser.addPreParse`. This hook walks the token list and
// promotes any NUMBER that immediately follows KEYWORD("GOTO"),
// KEYWORD("GOSUB"), or KEYWORD("THEN") to LINE_NUM.
//
// Why a pre-parse hook rather than a third lexer pass?
//   The lexer operates purely on the character stream and pattern matching.
//   It does not know the keyword context (whether a NUMBER follows GOTO or
//   appears in an arithmetic expression). The parser, after lexing, can see
//   the full token sequence and can apply context-sensitive relabeling.
//
//   A pre-parse hook is the clean integration point: it runs after lexing,
//   before the grammar rules are applied, and transforms the token list in
//   place. The GrammarParser.addPreParse API is designed for exactly this.
//
// Example:
//   Token stream after lexing: LINE_NUM("10") KEYWORD("GOTO") NUMBER("50") NEWLINE
//   After relabelJumpTargets:  LINE_NUM("10") KEYWORD("GOTO") LINE_NUM("50") NEWLINE
//
// ============================================================================
// USAGE
// ============================================================================
//
//     let ast = try DartmouthBasicParser.parse("10 LET X = 5\n20 END\n")
//     // ast.ruleName == "program"
//     // ast.children contains two "line" nodes
//
//     // Or from pre-lexed tokens:
//     let tokens = try DartmouthBasicLexer.tokenize(source)
//     let ast = try DartmouthBasicParser.parseTokens(tokens)
//
// ============================================================================

import Foundation
import GrammarTools
import Lexer
import Parser
import DartmouthBasicLexer

/// Dartmouth BASIC (1964) grammar-driven parser.
///
/// This struct is the public API for converting BASIC source text into an
/// ASTNode tree. Internally it uses:
///
///   1. `DartmouthBasicLexer.tokenize(_:)` — tokenize the source
///   2. `loadGrammar()` — load `dartmouth_basic.grammar` from the monorepo
///   3. `GrammarParser` — the grammar-driven parser engine, with a
///      `relabelJumpTargets` pre-parse hook to fix LINE_NUM vs NUMBER for
///      GOTO/GOSUB/IF-THEN targets
///
/// The resulting `ASTNode` has `ruleName = "program"` and a tree of children
/// that faithfully reflects every rule in dartmouth_basic.grammar.
///
/// All methods are static; the struct has no instance state, so it is
/// trivially `Sendable`.
///
public struct DartmouthBasicParser: Sendable {

    /// The version of this parser package.
    public static let version = "0.1.0"

    // =========================================================================
    // MARK: - Public API
    // =========================================================================

    /// Parse a Dartmouth BASIC source string into an AST.
    ///
    /// This is the main entry point. It lexes the source text, loads the
    /// grammar, and parses the resulting tokens.
    ///
    /// - Parameter source: The BASIC source code to parse (may be multi-line).
    /// - Returns: An `ASTNode` with `ruleName = "program"`.
    /// - Throws: Lexer errors (`LexerError`) or parser errors (`GrammarParseError`)
    ///           if the source is malformed.
    ///
    public static func parse(_ source: String) throws -> ASTNode {
        let tokens = try DartmouthBasicLexer.tokenize(source)
        return try parseTokens(tokens)
    }

    /// Parse a pre-lexed token stream into an AST.
    ///
    /// Use this when you already have a `[Token]` array — for example, if you
    /// want to inspect the token stream before parsing, or in tests that
    /// construct tokens manually.
    ///
    /// - Parameter tokens: A `[Token]` array from `DartmouthBasicLexer.tokenize(_:)`.
    /// - Returns: An `ASTNode` with `ruleName = "program"`.
    /// - Throws: `GrammarParseError` if the tokens do not form a valid program.
    ///
    public static func parseTokens(_ tokens: [Token]) throws -> ASTNode {
        let grammar = try loadGrammar()
        let parser = GrammarParser(tokens: tokens, grammar: grammar)

        // Register the pre-parse hook that promotes NUMBER tokens in jump-target
        // positions to LINE_NUM, so the grammar rules `goto_stmt = "GOTO" LINE_NUM`,
        // `gosub_stmt = "GOSUB" LINE_NUM`, and `if_stmt = "IF" ... "THEN" LINE_NUM`
        // match correctly.
        //
        // The lexer's relabelLineNumbers pass handles line-start positions.
        // This hook handles mid-line jump targets (GOTO 50, GOSUB 200, THEN 100).
        parser.addPreParse { tokens in
            tokens = relabelJumpTargets(tokens)
        }

        return try parser.parse()
    }

    /// Load and parse the Dartmouth BASIC parser grammar.
    ///
    /// The grammar file path is computed relative to this source file:
    ///
    ///   DartmouthBasicParser.swift         ← this file
    ///     Sources/DartmouthBasicParser/    (1) strip filename
    ///     dartmouth-basic-parser/          (2)
    ///     swift/                           (3)
    ///     packages/                        (4)
    ///     code/                            (5)  → grammars/dartmouth_basic.grammar
    ///
    /// The `#filePath` directive resolves to the compile-time path of this
    /// source file inside the monorepo. Walking up 6 levels reaches `code/`,
    /// then we descend into `grammars/`.
    ///
    /// - Returns: A `ParserGrammar` parsed from `dartmouth_basic.grammar`.
    /// - Throws: If the file cannot be read or the grammar cannot be parsed.
    ///
    public static func loadGrammar() throws -> ParserGrammar {
        let thisFile = #filePath
        var url = URL(fileURLWithPath: thisFile)
        // Walk up 6 path components:
        //   DartmouthBasicParser.swift  → Sources/DartmouthBasicParser
        //   Sources/DartmouthBasicParser → Sources
        //   Sources → dartmouth-basic-parser
        //   dartmouth-basic-parser → swift
        //   swift → packages
        //   packages → code
        for _ in 0..<6 {
            url = url.deletingLastPathComponent()
        }
        let grammarURL = url
            .appendingPathComponent("grammars")
            .appendingPathComponent("dartmouth_basic.grammar")

        let content = try String(contentsOf: grammarURL, encoding: .utf8)
        return try parseParserGrammar(source: content)
    }

    // =========================================================================
    // MARK: - Pre-parse Hook: relabelJumpTargets
    // =========================================================================

    /// Promote NUMBER tokens in jump-target position to LINE_NUM.
    ///
    /// The grammar rules for GOTO, GOSUB, and IF-THEN all reference LINE_NUM
    /// as the jump target:
    ///
    ///   goto_stmt  = "GOTO"  LINE_NUM ;
    ///   gosub_stmt = "GOSUB" LINE_NUM ;
    ///   if_stmt    = "IF" expr relop expr "THEN" LINE_NUM ;
    ///
    /// The lexer's `relabelLineNumbers` pass only promotes numbers at
    /// line-start. This function handles the mid-line jump targets:
    ///
    ///   Input:  ... KEYWORD("GOTO")  NUMBER("50") ...
    ///   Output: ... KEYWORD("GOTO")  LINE_NUM("50") ...
    ///
    ///   Input:  ... KEYWORD("GOSUB") NUMBER("200") ...
    ///   Output: ... KEYWORD("GOSUB") LINE_NUM("200") ...
    ///
    ///   Input:  ... KEYWORD("THEN")  NUMBER("100") ...
    ///   Output: ... KEYWORD("THEN")  LINE_NUM("100") ...
    ///
    /// Algorithm:
    ///   Walk tokens left-to-right, tracking whether the previous keyword
    ///   was GOTO, GOSUB, or THEN. When the current token is NUMBER and the
    ///   previous meaningful token was one of those keywords, relabel to LINE_NUM.
    ///
    /// - Parameter tokens: The token stream (modified in place).
    /// - Returns: A new token array with jump targets promoted.
    ///
    static func relabelJumpTargets(_ tokens: [Token]) -> [Token] {
        // Keywords whose immediately-following NUMBER is a line number, not
        // an arithmetic value. All three appear in the token stream with
        // uppercase values (thanks to the GrammarLexer's case-insensitive
        // keyword normalisation).
        let jumpPrecedingKeywords: Set<String> = ["GOTO", "GOSUB", "THEN"]

        var result: [Token] = []
        result.reserveCapacity(tokens.count)

        // `followsJumpKeyword` tracks whether the last KEYWORD token we saw
        // was GOTO, GOSUB, or THEN. If so, the next NUMBER token is a target.
        var followsJumpKeyword = false

        for token in tokens {
            switch token.type {
            case "KEYWORD":
                // Check whether this keyword introduces a jump target.
                followsJumpKeyword = jumpPrecedingKeywords.contains(token.value)
                result.append(token)

            case "NUMBER" where followsJumpKeyword:
                // This NUMBER immediately follows GOTO, GOSUB, or THEN.
                // Relabel it to LINE_NUM so the grammar rule matches.
                result.append(Token(
                    type: "LINE_NUM",
                    value: token.value,
                    line: token.line,
                    column: token.column,
                    flags: token.flags
                ))
                followsJumpKeyword = false

            default:
                // Any non-KEYWORD, non-NUMBER token resets the flag.
                // This handles edge cases like "IF X > 0 THEN 100":
                // Between THEN and 100 there are no tokens, so the flag
                // persists correctly. For more complex cases, we reset on
                // any intervening non-whitespace token.
                if token.type != "NEWLINE" && token.type != "EOF" {
                    // Only reset on real tokens (not structural separators).
                    // This is conservative — GOTO/GOSUB are always immediately
                    // followed by the target in valid BASIC programs.
                    followsJumpKeyword = false
                }
                result.append(token)
            }
        }

        return result
    }
}
