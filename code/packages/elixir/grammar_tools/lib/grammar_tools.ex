defmodule CodingAdventures.GrammarTools do
  @moduledoc """
  Parser and validator for `.tokens` and `.grammar` files.

  Grammar-tools is the foundation of the grammar-driven parsing pipeline.
  It reads declarative grammar definitions and produces structured data
  that the lexer and parser engines consume.

  ## Modules

  - `TokenGrammar` — parses `.tokens` files (lexical grammar)
  - `ParserGrammar` — parses `.grammar` files (syntactic grammar, EBNF)
  - `CrossValidator` — validates cross-references between tokens and grammar
  """

  alias CodingAdventures.GrammarTools.{TokenGrammar, ParserGrammar, CrossValidator, Compiler}

  defdelegate parse_token_grammar(source), to: TokenGrammar, as: :parse
  defdelegate parse_parser_grammar(source), to: ParserGrammar, as: :parse
  defdelegate cross_validate(token_grammar, parser_grammar), to: CrossValidator, as: :validate
  defdelegate compile_token_grammar(grammar, source_file \\ ""), to: Compiler
  defdelegate compile_parser_grammar(grammar, source_file \\ ""), to: Compiler
end
