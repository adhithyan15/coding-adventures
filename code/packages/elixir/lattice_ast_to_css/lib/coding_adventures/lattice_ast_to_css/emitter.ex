defmodule CodingAdventures.LatticeAstToCss.Emitter do
  @moduledoc """
  CSS emitter — reconstructs CSS text from a clean AST.

  After the transformer has expanded all Lattice nodes (variables, mixins,
  control flow, functions), the AST contains only pure CSS nodes:

  - `stylesheet` — the root
  - `qualified_rule` — selector + block (e.g., `h1 { color: red; }`)
  - `at_rule` — @-rules (e.g., `@media`, `@import`)
  - `selector_list` — comma-separated selectors
  - `complex_selector` — compound selectors with combinators
  - `compound_selector` — type/class/id/pseudo selectors
  - `block` — `{ declarations }`
  - `declaration` — `property: value;`
  - `value_list` — space-separated values
  - `function_call` — `rgb(255, 0, 0)`
  - `priority` — `!important`

  The emitter walks this tree and produces formatted CSS text.

  ## Two Formatting Modes

  - **Pretty-print** (default): 2-space indentation, newlines between
    declarations, blank lines between rules. Human-readable.

  - **Minified**: No unnecessary whitespace. Production-ready.

  ## Design

  The emitter dispatches on `rule_name`. Each rule has a handler function
  that knows how to format that particular CSS construct. Unknown rules
  fall through to a default handler that recurses into children.

  The emitter assumes the AST is clean — no Lattice nodes remain. If a
  Lattice node is encountered, it produces empty output (silently skipped).

  ## Usage

      css = Emitter.emit(css_ast)
      # => "h1 {\\n  color: red;\\n}\\n"

      css = Emitter.emit(css_ast, minified: true)
      # => "h1{color:red;}"
  """

  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  @doc """
  Emit CSS text from a clean AST.

  This is the main entry point. Pass the root `stylesheet` node (or any
  subtree) and get back a formatted CSS string.

  ## Parameters

  - `node` — an `%ASTNode{}` (typically the root `stylesheet`)
  - `opts` — keyword options:
    - `:minified` — if `true`, emit minified CSS (default: `false`)
    - `:indent` — indentation string (default: `"  "`)

  ## Returns

  A CSS string. If the AST produces no output, returns `""`.

  ## Examples

      css = Emitter.emit(css_ast)
      # "h1 {\\n  color: red;\\n}\\n"

      css = Emitter.emit(css_ast, minified: true)
      # "h1{color:red;}"
  """
  @spec emit(ASTNode.t() | Token.t(), keyword()) :: String.t()
  def emit(node, opts \\ []) do
    minified = Keyword.get(opts, :minified, false)
    indent = Keyword.get(opts, :indent, "  ")

    result = emit_node(node, 0, minified, indent)
    stripped = String.trim(result)

    if stripped == "" do
      ""
    else
      stripped <> "\n"
    end
  end

  # ---------------------------------------------------------------------------
  # Node dispatch
  # ---------------------------------------------------------------------------

  # Raw token — return its value
  defp emit_node(%Token{type: "STRING", value: v}, _depth, _min, _indent) do
    ~s("#{v}")
  end

  defp emit_node(%Token{value: value}, _depth, _min, _indent), do: value

  defp emit_node(%ASTNode{rule_name: rule_name, children: children}, depth, min, indent) do
    case rule_name do
      "stylesheet" -> emit_stylesheet(children, depth, min, indent)
      "rule" -> emit_rule(children, depth, min, indent)
      "qualified_rule" -> emit_qualified_rule(children, depth, min, indent)
      "at_rule" -> emit_at_rule(children, depth, min, indent)
      "at_prelude" -> emit_at_prelude(children, depth, min, indent)
      "at_prelude_token" -> emit_default(children, depth, min, indent)
      "at_prelude_tokens" -> emit_at_prelude_tokens(children, depth, min, indent)
      "function_in_prelude" -> emit_function_in_prelude(children, depth, min, indent)
      "paren_block" -> emit_paren_block(children, depth, min, indent)
      "selector_list" -> emit_selector_list(children, depth, min, indent)
      "complex_selector" -> emit_complex_selector(children, depth, min, indent)
      "combinator" -> emit_combinator(children)
      "compound_selector" -> emit_compound_selector(children, depth, min, indent)
      "simple_selector" -> emit_simple_selector(children)
      "subclass_selector" -> emit_subclass_selector(children, depth, min, indent)
      "class_selector" -> emit_class_selector(children)
      "id_selector" -> emit_id_selector(children)
      "attribute_selector" -> emit_attribute_selector(children, depth, min, indent)
      "attr_matcher" -> emit_attr_matcher(children)
      "attr_value" -> emit_attr_value(children)
      "pseudo_class" -> emit_pseudo_class(children, depth, min, indent)
      "pseudo_class_args" -> emit_pseudo_class_args(children, depth, min, indent)
      "pseudo_class_arg" -> emit_default(children, depth, min, indent)
      "pseudo_element" -> emit_pseudo_element(children)
      "block" -> emit_block(children, depth, min, indent)
      "block_contents" -> emit_block_contents(children, depth, min, indent)
      "block_item" -> emit_block_item(children, depth, min, indent)
      "declaration_or_nested" -> emit_declaration_or_nested(children, depth, min, indent)
      "declaration" -> emit_declaration(children, depth, min, indent)
      "property" -> emit_property(children)
      "priority" -> "!important"
      "value_list" -> emit_value_list(children, depth, min, indent)
      "value" -> emit_value(children, depth, min, indent)
      "function_call" -> emit_function_call(children, depth, min, indent)
      "function_args" -> emit_function_args(children, depth, min, indent)
      "function_arg" -> emit_function_arg(children, depth, min, indent)
      # Lattice-specific nodes should not appear in a clean AST — silently skip
      _ -> emit_default(children, depth, min, indent)
    end
  end

  defp emit_node(_, _depth, _min, _indent), do: ""

  # ---------------------------------------------------------------------------
  # Top-Level Structure
  # ---------------------------------------------------------------------------

  # stylesheet = { rule } ;
  # Join rules with blank lines (pretty) or nothing (minified).
  defp emit_stylesheet(children, depth, min, indent) do
    parts =
      children
      |> Enum.map(fn child -> emit_node(child, depth, min, indent) end)
      |> Enum.reject(fn s -> String.trim(s) == "" end)

    if min do
      Enum.join(parts)
    else
      Enum.join(parts, "\n\n")
    end
  end

  # rule = lattice_rule | at_rule | qualified_rule ;
  # A wrapper — just emit the single child.
  defp emit_rule([child | _], depth, min, indent) do
    emit_node(child, depth, min, indent)
  end

  defp emit_rule([], _depth, _min, _indent), do: ""

  # ---------------------------------------------------------------------------
  # Qualified Rules
  # ---------------------------------------------------------------------------

  # qualified_rule = selector_list block ;
  # Emits: selector_list { declarations... }
  defp emit_qualified_rule(children, depth, min, indent) do
    selector = Enum.reduce(children, "", fn child, acc ->
      case child do
        %ASTNode{rule_name: "selector_list"} ->
          acc <> emit_node(child, depth, min, indent)
        _ -> acc
      end
    end)

    block_text = Enum.reduce(children, "", fn child, acc ->
      case child do
        %ASTNode{rule_name: "block"} ->
          acc <> emit_block(child.children, depth, min, indent)
        _ -> acc
      end
    end)

    if min do
      selector <> block_text
    else
      if selector == "" do
        block_text
      else
        selector <> " " <> block_text
      end
    end
  end

  # ---------------------------------------------------------------------------
  # At-Rules
  # ---------------------------------------------------------------------------

  # at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;
  defp emit_at_rule(children, depth, min, indent) do
    keyword = Enum.reduce(children, "", fn child, acc ->
      case child do
        %Token{type: "AT_KEYWORD"} -> child.value
        _ -> acc
      end
    end)

    prelude = Enum.reduce(children, "", fn child, acc ->
      case child do
        %ASTNode{rule_name: "at_prelude"} ->
          emit_at_prelude(child.children, depth, min, indent)
        _ -> acc
      end
    end)

    has_semicolon = Enum.any?(children, fn
      %Token{type: "SEMICOLON"} -> true
      _ -> false
    end)

    block_text = Enum.reduce(children, "", fn child, acc ->
      case child do
        %ASTNode{rule_name: "block"} ->
          emit_block(child.children, depth, min, indent)
        _ -> acc
      end
    end)

    if min do
      if has_semicolon do
        keyword <> prelude <> ";"
      else
        keyword <> prelude <> block_text
      end
    else
      prelude_part = if String.trim(prelude) == "", do: "", else: " " <> prelude

      if has_semicolon do
        keyword <> prelude_part <> ";"
      else
        keyword <> prelude_part <> " " <> block_text
      end
    end
  end

  defp emit_at_prelude(children, depth, min, indent) do
    parts = Enum.map(children, fn child -> emit_node(child, depth, min, indent) end)
    Enum.join(parts, " ")
  end

  defp emit_at_prelude_tokens(children, depth, min, indent) do
    parts = Enum.map(children, fn child -> emit_node(child, depth, min, indent) end)
    Enum.join(parts, " ")
  end

  defp emit_function_in_prelude(children, depth, min, indent) do
    parts = Enum.map(children, fn child ->
      case child do
        %Token{type: "RPAREN"} -> ")"
        _ -> emit_node(child, depth, min, indent)
      end
    end)
    Enum.join(parts)
  end

  defp emit_paren_block(children, depth, min, indent) do
    parts = Enum.map(children, fn child ->
      case child do
        %Token{type: "LPAREN"} -> "("
        %Token{type: "RPAREN"} -> ")"
        _ -> emit_node(child, depth, min, indent)
      end
    end)
    Enum.join(parts)
  end

  # ---------------------------------------------------------------------------
  # Selectors
  # ---------------------------------------------------------------------------

  # selector_list = complex_selector { COMMA complex_selector } ;
  defp emit_selector_list(children, depth, min, indent) do
    parts =
      children
      |> Enum.reject(fn
        %Token{type: "COMMA"} -> true
        _ -> false
      end)
      |> Enum.map(fn child -> emit_node(child, depth, min, indent) end)

    sep = if min, do: ",", else: ", "
    Enum.join(parts, sep)
  end

  # complex_selector = compound_selector { [ combinator ] compound_selector } ;
  defp emit_complex_selector(children, depth, min, indent) do
    parts = Enum.map(children, fn child -> emit_node(child, depth, min, indent) end)
    Enum.join(parts, " ")
  end

  defp emit_combinator([child | _]) do
    case child do
      %Token{value: v} -> v
      _ -> ""
    end
  end

  defp emit_combinator([]), do: ""

  # compound_selector — concatenate without spaces: h1.classname#id
  defp emit_compound_selector(children, depth, min, indent) do
    parts = Enum.map(children, fn child -> emit_node(child, depth, min, indent) end)
    Enum.join(parts)
  end

  defp emit_simple_selector([child | _]) do
    case child do
      %Token{value: v} -> v
      _ -> ""
    end
  end

  defp emit_simple_selector([]), do: ""

  defp emit_subclass_selector([child | _], depth, min, indent) do
    emit_node(child, depth, min, indent)
  end

  defp emit_subclass_selector([], _depth, _min, _indent), do: ""

  # class_selector = DOT IDENT ;  → .classname
  defp emit_class_selector(children) do
    parts =
      children
      |> Enum.filter(fn %Token{} -> true; _ -> false end)
      |> Enum.map(fn %Token{value: v} -> v end)
    Enum.join(parts)
  end

  # id_selector = HASH ;
  defp emit_id_selector([%Token{value: v} | _]), do: v
  defp emit_id_selector([]), do: ""

  # attribute_selector = LBRACKET IDENT [ attr_matcher attr_value [ IDENT ] ] RBRACKET ;
  defp emit_attribute_selector(children, depth, min, indent) do
    parts = Enum.map(children, fn child ->
      case child do
        %Token{type: "LBRACKET"} -> "["
        %Token{type: "RBRACKET"} -> "]"
        %Token{value: v} -> v
        %ASTNode{} -> emit_node(child, depth, min, indent)
        _ -> ""
      end
    end)
    Enum.join(parts)
  end

  defp emit_attr_matcher([child | _]) do
    case child do
      %Token{value: v} -> v
      _ -> ""
    end
  end

  defp emit_attr_matcher([]), do: ""

  defp emit_attr_value([child | _]) do
    case child do
      %Token{type: "STRING", value: v} -> ~s("#{v}")
      %Token{value: v} -> v
      _ -> ""
    end
  end

  defp emit_attr_value([]), do: ""

  # pseudo_class = COLON FUNCTION pseudo_class_args RPAREN | COLON IDENT ;
  defp emit_pseudo_class(children, depth, min, indent) do
    parts = Enum.map(children, fn child ->
      case child do
        %Token{type: "COLON"} -> ":"
        %Token{type: "RPAREN"} -> ")"
        %Token{value: v} -> v
        %ASTNode{} -> emit_node(child, depth, min, indent)
        _ -> ""
      end
    end)
    Enum.join(parts)
  end

  defp emit_pseudo_class_args(children, depth, min, indent) do
    parts = Enum.map(children, fn child -> emit_node(child, depth, min, indent) end)
    Enum.join(parts)
  end

  # pseudo_element = COLON_COLON IDENT ;
  defp emit_pseudo_element(children) do
    parts = Enum.map(children, fn child ->
      case child do
        %Token{type: "COLON_COLON"} -> "::"
        %Token{value: v} -> v
        _ -> ""
      end
    end)
    Enum.join(parts)
  end

  # ---------------------------------------------------------------------------
  # Blocks and Declarations
  # ---------------------------------------------------------------------------

  # block = LBRACE block_contents RBRACE ;
  defp emit_block(children, depth, min, indent) do
    contents_node = Enum.find(children, fn
      %ASTNode{rule_name: "block_contents"} -> true
      _ -> false
    end)

    if min do
      case contents_node do
        nil -> "{}"
        node ->
          inner = emit_block_contents(node.children, depth + 1, min, indent)
          "{" <> inner <> "}"
      end
    else
      case contents_node do
        nil ->
          "{\n" <> String.duplicate(indent, depth) <> "}"
        node ->
          inner = emit_block_contents(node.children, depth + 1, min, indent)
          if String.trim(inner) == "" do
            "{\n" <> String.duplicate(indent, depth) <> "}"
          else
            "{\n" <> inner <> "\n" <> String.duplicate(indent, depth) <> "}"
          end
      end
    end
  end

  # block_contents = { block_item } ;
  defp emit_block_contents(children, depth, min, indent) do
    parts =
      children
      |> Enum.map(fn child -> emit_node(child, depth, min, indent) end)
      |> Enum.reject(fn s -> String.trim(s) == "" end)

    if min do
      Enum.join(parts)
    else
      prefix = String.duplicate(indent, depth)
      Enum.map_join(parts, "\n", fn part -> prefix <> part end)
    end
  end

  defp emit_block_item([child | _], depth, min, indent) do
    emit_node(child, depth, min, indent)
  end

  defp emit_block_item([], _depth, _min, _indent), do: ""

  defp emit_declaration_or_nested([child | _], depth, min, indent) do
    emit_node(child, depth, min, indent)
  end

  defp emit_declaration_or_nested([], _depth, _min, _indent), do: ""

  # declaration = property COLON value_list [ priority ] SEMICOLON ;
  defp emit_declaration(children, depth, min, indent) do
    prop = Enum.reduce(children, "", fn child, acc ->
      case child do
        %ASTNode{rule_name: "property"} -> emit_property(child.children)
        _ -> acc
      end
    end)

    value = Enum.reduce(children, "", fn child, acc ->
      case child do
        %ASTNode{rule_name: "value_list"} ->
          emit_value_list(child.children, depth, min, indent)
        _ -> acc
      end
    end)

    has_priority = Enum.any?(children, fn
      %ASTNode{rule_name: "priority"} -> true
      _ -> false
    end)

    priority_str = if has_priority, do: " !important", else: ""

    if min do
      prop <> ":" <> value <> priority_str <> ";"
    else
      prop <> ": " <> value <> priority_str <> ";"
    end
  end

  defp emit_property([child | _]) do
    case child do
      %Token{value: v} -> v
      _ -> ""
    end
  end

  defp emit_property([]), do: ""

  # ---------------------------------------------------------------------------
  # Values
  # ---------------------------------------------------------------------------

  # value_list = value { value } ;
  # Space-separate values, but collapse spaces around commas.
  defp emit_value_list(children, depth, min, indent) do
    parts = Enum.map(children, fn child -> emit_node(child, depth, min, indent) end)

    result = Enum.join(parts, " ")
    # Collapse spaces around commas: " , " → ", " and " ," → ","
    result
    |> String.replace(" , ", ", ")
    |> String.replace(" ,", ",")
  end

  # value = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT | HASH | ...
  defp emit_value([child | _], depth, min, indent) do
    case child do
      %ASTNode{} -> emit_node(child, depth, min, indent)
      %Token{type: "STRING", value: v} -> ~s("#{v}")
      %Token{value: v} -> v
      _ -> ""
    end
  end

  defp emit_value([], _depth, _min, _indent), do: ""

  # function_call = FUNCTION function_args RPAREN | URL_TOKEN ;
  defp emit_function_call(children, depth, min, indent) do
    case children do
      [%Token{type: "URL_TOKEN", value: v}] ->
        v

      _ ->
        parts = Enum.map(children, fn child ->
          case child do
            %Token{type: "FUNCTION", value: v} -> v  # Already includes "("
            %Token{type: "RPAREN"} -> ")"
            %Token{value: v} -> v
            %ASTNode{} -> emit_node(child, depth, min, indent)
            _ -> ""
          end
        end)
        Enum.join(parts)
    end
  end

  # function_args = { function_arg } ;
  defp emit_function_args(children, depth, min, indent) do
    parts = Enum.map(children, fn child -> emit_node(child, depth, min, indent) end)

    result = Enum.join(parts, " ")
    result
    |> String.replace(" , ", ", ")
    |> String.replace(" ,", ",")
  end

  # function_arg — single or multiple children
  #
  # When a function_arg contains a single token or AST node, emit it directly.
  # When it contains multiple children (e.g., a nested function call structured as
  # FUNCTION, function_args, RPAREN), join the parts with NO space — otherwise
  # "rgb( 255, 0, 0 )" would become "rgb ( 255, 0, 0 )" with an unwanted space.
  defp emit_function_arg([child], depth, min, indent) do
    case child do
      %ASTNode{} -> emit_node(child, depth, min, indent)
      %Token{value: v} -> v
      _ -> ""
    end
  end

  defp emit_function_arg([_ | _] = children, depth, min, indent) do
    parts = Enum.map(children, fn child ->
      case child do
        %Token{type: "RPAREN"} -> ")"
        %ASTNode{} -> emit_node(child, depth, min, indent)
        %Token{value: v} -> v
        _ -> ""
      end
    end)
    Enum.join(parts, "")
  end

  defp emit_function_arg([], _depth, _min, _indent), do: ""

  # ---------------------------------------------------------------------------
  # Default and Utilities
  # ---------------------------------------------------------------------------

  # Default handler: concatenate children with spaces
  defp emit_default(children, depth, min, indent) do
    parts = Enum.map(children, fn child -> emit_node(child, depth, min, indent) end)
    Enum.join(parts, " ")
  end
end
