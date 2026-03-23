# frozen_string_literal: true

# ================================================================
# Lattice Transpiler -- Source to CSS Pipeline
# ================================================================
#
# This module is intentionally thin. It wires together three
# packages into a single end-to-end pipeline:
#
#   1. LatticeParser.parse()        -- Source text -> Lattice AST
#   2. LatticeTransformer.transform() -- Lattice AST -> Clean CSS AST
#   3. CSSEmitter.emit()            -- Clean CSS AST -> CSS text
#
# Each step is a standalone package with its own tests. This module
# just connects them in sequence.
#
# Pipeline Diagram:
#
#   Lattice Source
#        |
#        v
#   [LatticeLexer]    <- lattice.tokens
#        |
#        v tokens
#   [LatticeParser]   <- lattice.grammar
#        |
#        v AST (CSS + Lattice nodes)
#   [LatticeTransformer]   <- scope, evaluator
#        |
#        v AST (CSS nodes only)
#   [CSSEmitter]
#        |
#        v
#     CSS Text
#
# Usage:
#
#   css = CodingAdventures::LatticeTranspiler.transpile(source)
#
#   css = CodingAdventures::LatticeTranspiler.transpile(
#     source,
#     minified: true,
#     indent: "    "
#   )
# ================================================================

module CodingAdventures
  module LatticeTranspiler
    # Transpile Lattice source text to CSS.
    #
    # This is the main entry point. Pass in Lattice source text and
    # receive back CSS text.
    #
    # @param source [String] the Lattice source text to transpile
    # @param minified [Boolean] if true, emit minified CSS (no whitespace)
    # @param indent [String] indentation per nesting level (default: 2 spaces)
    # @return [String] the transpiled CSS text
    # @raise [CodingAdventures::LatticeAstToCss::LatticeError] for
    #   Lattice-specific errors (undefined variables, circular mixins, etc.)
    # @raise [CodingAdventures::Parser::GrammarParseError] for syntax errors
    # @raise [CodingAdventures::Lexer::LexerError] for lexical errors
    #
    # Example:
    #
    #   css = CodingAdventures::LatticeTranspiler.transpile(<<~LATTICE)
    #     $primary: #4a90d9;
    #     h1 { color: $primary; }
    #   LATTICE
    #   # => "h1 {\n  color: #4a90d9;\n}\n"
    def self.transpile(source, minified: false, indent: "  ")
      # Step 1: Parse Lattice source into an AST.
      # This runs the lexer (lattice.tokens) and parser (lattice.grammar).
      # The result is a "stylesheet" ASTNode containing both CSS nodes
      # (qualified_rule, declaration, at_rule) and Lattice nodes
      # (variable_declaration, mixin_definition, if_directive, etc.).
      ast = CodingAdventures::LatticeParser.parse(source)

      # Step 2: Transform the Lattice AST into a clean CSS AST.
      # The transformer runs three passes:
      #   Pass 1: Collect variable/mixin/function definitions
      #   Pass 2: Expand all Lattice constructs (substitute variables,
      #           expand mixins, evaluate control flow and functions)
      #   Pass 3: Remove empty nodes
      # The result contains only pure CSS nodes.
      transformer = CodingAdventures::LatticeAstToCss::LatticeTransformer.new
      css_ast = transformer.transform(ast)

      # Step 3: Emit CSS text from the clean AST.
      # The emitter walks the CSS AST and produces formatted CSS text.
      emitter = CodingAdventures::LatticeAstToCss::CSSEmitter.new(
        indent: indent,
        minified: minified
      )
      emitter.emit(css_ast)
    end
  end
end
