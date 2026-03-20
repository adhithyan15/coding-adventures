defmodule CodingAdventures.Parser.ASTNode do
  @moduledoc """
  A generic AST node produced by grammar-driven parsing.

  Every node records which grammar rule created it (`rule_name`) and the
  matched sub-structure (`children`). Children are a mix of `ASTNode`
  structs and `Token` structs.

  This generic representation makes the parser language-agnostic — the same
  `ASTNode` type works for JSON, Python, Ruby, or any language whose grammar
  is written in a `.grammar` file.
  """

  alias CodingAdventures.Lexer.Token

  defstruct [:rule_name, children: []]

  @type t :: %__MODULE__{
          rule_name: String.t(),
          children: [t() | Token.t()]
        }

  @doc "True if this node wraps a single token (no sub-structure)."
  def leaf?(%__MODULE__{children: [%Token{}]}), do: true
  def leaf?(_), do: false

  @doc "The token if this is a leaf node, nil otherwise."
  def token(%__MODULE__{children: [%Token{} = t]}), do: t
  def token(_), do: nil
end
