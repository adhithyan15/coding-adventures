defmodule CodingAdventures.MosaicAnalyzer do
  @moduledoc """
  Mosaic Analyzer — Walks a Mosaic AST and produces a typed MosaicIR.

  The analyzer is the **third stage** of the Mosaic compiler pipeline:

      Source text → Lexer → Tokens → Parser → ASTNode → Analyzer → MosaicIR

  ## What is an Intermediate Representation?

  The term IR (Intermediate Representation) comes from compiler design. A
  compiler typically works in stages:

      Source code  →  AST  →  IR  →  Target code

  The AST is a direct reflection of the source syntax — every token and
  grammar rule has a corresponding tree node. The IR is a cleaned-up, typed
  version of the same information where:

    - Syntax noise (keywords, semicolons, braces) is stripped away
    - Every name is resolved — no bare strings, just typed values
    - Defaults are normalized — `"0"` becomes `{:number, 0.0, nil}`
    - Errors are caught — undefined slots, invalid types, unknown properties

  ## Primitive Nodes

  Primitive nodes are the built-in layout and display elements:

      Row, Column, Box, Stack, Text, Image, Icon, Spacer, Divider, Scroll

  Non-primitive nodes are imported component types (e.g., `Button`, `Badge`).
  The `is_primitive` field on `MosaicNode` distinguishes them.

  ## MosaicIR Type System (Elixir Tagged Tuples)

  Since Elixir does not have discriminated union types, we use tagged tuples
  as a idiomatic alternative:

  ### MosaicType

      {:primitive, "text"}        — text slot type
      {:primitive, "number"}      — number slot type
      {:primitive, "bool"}        — boolean slot type
      {:primitive, "image"}       — image slot type
      {:primitive, "color"}       — color slot type
      {:primitive, "node"}        — flexible node slot type
      {:component, "Button"}      — named component type from an import
      {:list, inner_type}         — parameterized list type

  ### MosaicValue

      {:slot_ref, "name"}               — @name slot reference
      {:string, "hello"}                — string literal
      {:number, 42.0}                   — bare number
      {:dimension, 16.0, "dp"}          — dimension with unit (e.g., 16dp)
      {:color_hex, "#2563eb"}           — hex color literal
      {:bool, true}                     — boolean literal
      {:ident, "center"}                — bare identifier
      {:enum_val, "align", "center"}    — dotted namespace.member reference

  ### MosaicChild

      {:node, %MosaicNode{}}                        — child element
      {:slot_ref_child, "name"}                     — @name; used as child
      {:when_block, "slot", [...children]}          — conditional subtree
      {:each_block, "slot", "item", [...children]}  — iterating subtree

  ## Usage

      {:ok, component} = CodingAdventures.MosaicAnalyzer.analyze(~S(
        component Label {
          slot title: text;
          Text { content: @title; }
        }
      ))

      component.name          # => "Label"
      component.slots         # => [%MosaicSlot{name: "title", type: {:primitive, "text"}, ...}]
      component.imports       # => []
      component.root.node_type  # => "Text"
  """

  alias CodingAdventures.MosaicParser
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  # ============================================================================
  # IR Struct Definitions
  # ============================================================================

  defmodule MosaicSlot do
    @moduledoc """
    A typed data slot — the "props API" of a Mosaic component.

    Slots are the only way data enters a Mosaic component. There are no global
    variables, no context, no implicit state. The host language fills slots
    via generated typed setters before the component renders.

    ## Fields

      - `:name` — slot name, e.g., `"title"`, `"avatar-url"`, `"display-name"`
      - `:type` — MosaicType tagged tuple, e.g., `{:primitive, "text"}`
      - `:default_value` — optional MosaicValue, present when `= value` exists
      - `:required` — `true` when no default value is given

    ## Examples

        # slot title: text;
        %MosaicSlot{name: "title", type: {:primitive, "text"}, required: true}

        # slot count: number = 0;
        %MosaicSlot{name: "count", type: {:primitive, "number"},
                    default_value: {:number, 0.0}, required: false}
    """
    defstruct [:name, :type, :default_value, required: true]

    @type mosaic_type ::
            {:primitive, String.t()}
            | {:component, String.t()}
            | {:list, mosaic_type()}

    @type mosaic_value ::
            {:slot_ref, String.t()}
            | {:string, String.t()}
            | {:number, float()}
            | {:dimension, float(), String.t()}
            | {:color_hex, String.t()}
            | {:bool, boolean()}
            | {:ident, String.t()}
            | {:enum_val, String.t(), String.t()}

    @type t :: %__MODULE__{
            name: String.t(),
            type: mosaic_type(),
            default_value: mosaic_value() | nil,
            required: boolean()
          }
  end

  defmodule MosaicProperty do
    @moduledoc """
    A single property assignment on a node.

    Properties set abstract layout/visual traits. Backends map them to
    platform-native equivalents (CSS properties, SwiftUI modifiers, etc.).

    ## Examples

        # padding: 16dp;
        %MosaicProperty{name: "padding", value: {:dimension, 16.0, "dp"}}

        # background: #2563eb;
        %MosaicProperty{name: "background", value: {:color_hex, "#2563eb"}}

        # align: center;
        %MosaicProperty{name: "align", value: {:ident, "center"}}
    """
    defstruct [:name, :value]

    @type t :: %__MODULE__{
            name: String.t(),
            value: MosaicSlot.mosaic_value()
          }
  end

  defmodule MosaicNode do
    @moduledoc """
    A visual node in the component tree.

    Nodes correspond to platform-native elements. Primitive nodes are the
    built-in layout containers and display elements (Row, Column, Box, Stack,
    Text, Image, Icon, Spacer, Divider, Scroll). Non-primitive nodes are
    imported component types.

    ## Fields

      - `:node_type` — element type name, e.g., `"Box"`, `"Text"`, `"Button"`
      - `:is_primitive` — `true` for built-in elements, `false` for imported components
      - `:properties` — list of `MosaicProperty` structs
      - `:children` — list of MosaicChild tagged tuples

    Note: The field is named `:node_type` (not `:node`) because `node` is
    a reserved word in Elixir (it is a macro for the current BEAM node name).
    """
    defstruct [:node_type, is_primitive: false, properties: [], children: []]

    @type mosaic_child ::
            {:node, t()}
            | {:slot_ref_child, String.t()}
            | {:when_block, String.t(), [mosaic_child()]}
            | {:each_block, String.t(), String.t(), [mosaic_child()]}

    @type t :: %__MODULE__{
            node_type: String.t(),
            is_primitive: boolean(),
            properties: [MosaicProperty.t()],
            children: [mosaic_child()]
          }
  end

  defmodule MosaicImport do
    @moduledoc """
    An `import X from "..."` declaration.

    Imports bring other `.mosaic` components into scope so they can be used
    as slot types or as composite nodes.

    ## Fields

      - `:name` — the component name used locally (either original or alias)

    ## Examples

        # import Button from "./button.mosaic"
        %MosaicImport{name: "Button"}

        # import Card as InfoCard from "./cards/info.mosaic"
        %MosaicImport{name: "InfoCard"}
    """
    defstruct [:name]

    @type t :: %__MODULE__{
            name: String.t()
          }
  end

  defmodule MosaicComponent do
    @moduledoc """
    A Mosaic component — the unit of UI composition.

    A component has:

      - A **name** (PascalCase by convention, e.g., `ProfileCard`)
      - **Slots** — typed data inputs (like props in React)
      - **Imports** — other components brought into scope
      - A **root** — the root `MosaicNode` of the visual hierarchy

    ## Example

        %MosaicComponent{
          name: "ProfileCard",
          slots: [
            %MosaicSlot{name: "avatar-url", type: {:primitive, "image"}, required: true},
            %MosaicSlot{name: "display-name", type: {:primitive, "text"}, required: true},
          ],
          imports: [%MosaicImport{name: "Button"}],
          root: %MosaicNode{node_type: "Column", is_primitive: true, ...}
        }
    """
    defstruct [:name, slots: [], imports: [], root: nil]

    @type t :: %__MODULE__{
            name: String.t(),
            slots: [MosaicAnalyzer.MosaicSlot.t()],
            imports: [MosaicAnalyzer.MosaicImport.t()],
            root: MosaicAnalyzer.MosaicNode.t() | nil
          }
  end

  # ============================================================================
  # Public API
  # ============================================================================

  @doc """
  Analyze Mosaic source text and return a typed `MosaicComponent`.

  This is the main entry point. It parses the source, then walks the
  resulting AST to produce a validated intermediate representation.

  Returns `{:ok, %MosaicComponent{}}` on success.
  Returns `{:error, message}` on parse error or semantic error.

  ## Examples

      {:ok, component} = CodingAdventures.MosaicAnalyzer.analyze(~S(
        component Label {
          slot title: text;
          Text { content: @title; }
        }
      ))
      component.name  # => "Label"

      {:error, msg} = CodingAdventures.MosaicAnalyzer.analyze("not valid mosaic")
  """
  @spec analyze(String.t()) ::
          {:ok, MosaicComponent.t()} | {:error, String.t()}
  def analyze(source) do
    case MosaicParser.parse(source) do
      {:ok, ast} -> analyze_ast(ast)
      {:error, msg} -> {:error, msg}
    end
  end

  @doc """
  Analyze a pre-parsed `ASTNode` and return a typed `MosaicComponent`.

  Use this variant when you already have an AST and want to avoid re-parsing.
  The AST root must have `rule_name: "file"`.

  Returns `{:ok, %MosaicComponent{}}` on success.
  Returns `{:error, message}` on semantic error.
  """
  @spec analyze_ast(ASTNode.t()) ::
          {:ok, MosaicComponent.t()} | {:error, String.t()}
  def analyze_ast(%ASTNode{rule_name: "file"} = ast) do
    try do
      component = do_analyze_file(ast)
      {:ok, component}
    rescue
      e in RuntimeError -> {:error, e.message}
    end
  end

  def analyze_ast(%ASTNode{rule_name: rule}) do
    {:error, "Expected root rule \"file\", got \"#{rule}\""}
  end

  # ============================================================================
  # File-Level Analysis
  # ============================================================================

  # Walk the "file" node to collect imports and find the component_decl.
  defp do_analyze_file(%ASTNode{children: children}) do
    {imports, component_decl} =
      Enum.reduce(children, {[], nil}, fn child, {acc_imports, acc_decl} ->
        case child do
          %ASTNode{rule_name: "import_decl"} ->
            {[analyze_import(child) | acc_imports], acc_decl}

          %ASTNode{rule_name: "component_decl"} ->
            {acc_imports, child}

          _ ->
            {acc_imports, acc_decl}
        end
      end)

    if is_nil(component_decl) do
      raise "No component declaration found in file"
    end

    imports = Enum.reverse(imports)
    analyze_component(component_decl, imports)
  end

  # ============================================================================
  # Import Analysis
  # ============================================================================

  # import_decl = KEYWORD NAME [ KEYWORD NAME ] KEYWORD STRING SEMICOLON
  # Tokens: "import" NAME [optional: "as" NAME] "from" STRING ";"
  #
  # The local name is the alias (second NAME) if present, otherwise the
  # component name (first NAME).
  defp analyze_import(%ASTNode{children: children}) do
    names = direct_token_values(children, "NAME")

    if Enum.empty?(names) do
      raise "import_decl missing component name"
    end

    # If two NAMEs exist, the second is the alias (used as local name).
    # e.g., import Card as InfoCard from "..."; → name = "InfoCard"
    # e.g., import Button from "..."; → name = "Button"
    local_name =
      if length(names) >= 2 do
        Enum.at(names, 1)
      else
        hd(names)
      end

    %MosaicImport{name: local_name}
  end

  # ============================================================================
  # Component Analysis
  # ============================================================================

  # component_decl = KEYWORD NAME LBRACE { slot_decl } node_tree RBRACE
  defp analyze_component(%ASTNode{children: children}, imports) do
    names = direct_token_values(children, "NAME")

    if Enum.empty?(names) do
      raise "component_decl missing name"
    end

    name = hd(names)

    {slots, tree_node} =
      Enum.reduce(children, {[], nil}, fn child, {acc_slots, acc_tree} ->
        case child do
          %ASTNode{rule_name: "slot_decl"} ->
            {[analyze_slot(child) | acc_slots], acc_tree}

          %ASTNode{rule_name: "node_tree"} ->
            {acc_slots, child}

          _ ->
            {acc_slots, acc_tree}
        end
      end)

    if is_nil(tree_node) do
      raise "component \"#{name}\" has no node tree"
    end

    slots = Enum.reverse(slots)
    root = analyze_node_tree(tree_node)

    %MosaicComponent{
      name: name,
      slots: slots,
      imports: imports,
      root: root
    }
  end

  # ============================================================================
  # Slot Analysis
  # ============================================================================

  # slot_decl = KEYWORD NAME COLON slot_type [ EQUALS default_value ] SEMICOLON
  defp analyze_slot(%ASTNode{children: children}) do
    names = direct_token_values(children, "NAME")

    if Enum.empty?(names) do
      raise "slot_decl missing name"
    end

    slot_name = hd(names)

    slot_type_node = find_child(children, "slot_type")

    if is_nil(slot_type_node) do
      raise "slot \"#{slot_name}\" missing type"
    end

    type = analyze_slot_type(slot_type_node)

    default_value_node = find_child(children, "default_value")

    default_value =
      if default_value_node do
        analyze_default_value(default_value_node)
      else
        nil
      end

    required = is_nil(default_value)

    %MosaicSlot{
      name: slot_name,
      type: type,
      default_value: default_value,
      required: required
    }
  end

  # slot_type = KEYWORD | NAME | list_type
  defp analyze_slot_type(%ASTNode{children: children}) do
    # Check for list_type first (it's the most specific alternative).
    case find_child(children, "list_type") do
      %ASTNode{} = list_node ->
        analyze_list_type(list_node)

      nil ->
        # A bare KEYWORD means a primitive type (text, number, bool, etc.)
        case first_token_value(children, "KEYWORD") do
          keyword when is_binary(keyword) ->
            parse_primitive_type(keyword)

          nil ->
            # A NAME means a component type from an import.
            case first_token_value(children, "NAME") do
              comp_name when is_binary(comp_name) ->
                {:component, comp_name}

              nil ->
                raise "slot_type has no recognizable content"
            end
        end
    end
  end

  # list_type = KEYWORD LANGLE slot_type RANGLE
  defp analyze_list_type(%ASTNode{children: children}) do
    inner_type_node = find_child(children, "slot_type")

    if is_nil(inner_type_node) do
      raise "list_type missing element type"
    end

    {:list, analyze_slot_type(inner_type_node)}
  end

  # Map primitive type keyword strings to {:primitive, keyword} tuples.
  # The six primitives are: text, number, bool, image, color, node.
  defp parse_primitive_type(keyword)
       when keyword in ["text", "number", "bool", "image", "color", "node"] do
    {:primitive, keyword}
  end

  defp parse_primitive_type(keyword) do
    raise "Unknown primitive type keyword: \"#{keyword}\""
  end

  # default_value = STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD
  defp analyze_default_value(%ASTNode{children: children}) do
    cond do
      (v = first_token_value(children, "STRING")) != nil ->
        {:string, v}

      (v = first_token_value(children, "DIMENSION")) != nil ->
        parse_dimension(v)

      (v = first_token_value(children, "NUMBER")) != nil ->
        {:number, parse_float(v)}

      (v = first_token_value(children, "COLOR_HEX")) != nil ->
        {:color_hex, v}

      (v = first_token_value(children, "KEYWORD")) != nil ->
        case v do
          "true" -> {:bool, true}
          "false" -> {:bool, false}
          _ -> {:ident, v}
        end

      true ->
        raise "default_value has no recognizable content"
    end
  end

  # ============================================================================
  # Node Tree Analysis
  # ============================================================================

  # node_tree = node_element
  defp analyze_node_tree(%ASTNode{children: children}) do
    el = find_child(children, "node_element")

    if is_nil(el) do
      raise "node_tree missing node_element"
    end

    analyze_node_element(el)
  end

  # node_element = NAME LBRACE { node_content } RBRACE
  defp analyze_node_element(%ASTNode{children: children}) do
    tag = first_token_value(children, "NAME")

    if is_nil(tag) do
      raise "node_element missing tag name"
    end

    is_primitive = tag in primitive_nodes()

    {properties, node_children} =
      children
      |> Enum.filter(fn child -> match?(%ASTNode{rule_name: "node_content"}, child) end)
      |> Enum.reduce({[], []}, fn content_node, {acc_props, acc_children} ->
        case analyze_node_content(content_node) do
          {:prop, prop} -> {[prop | acc_props], acc_children}
          {:child, child_item} -> {acc_props, [child_item | acc_children]}
          nil -> {acc_props, acc_children}
        end
      end)

    %MosaicNode{
      node_type: tag,
      is_primitive: is_primitive,
      properties: Enum.reverse(properties),
      children: Enum.reverse(node_children)
    }
  end

  # node_content = property_assignment | child_node | slot_reference | when_block | each_block
  defp analyze_node_content(%ASTNode{children: children}) do
    Enum.find_value(children, fn child ->
      case child do
        %ASTNode{rule_name: "property_assignment"} ->
          {:prop, analyze_property_assignment(child)}

        %ASTNode{rule_name: "child_node"} ->
          case find_child(child.children, "node_element") do
            %ASTNode{} = el -> {:child, {:node, analyze_node_element(el)}}
            nil -> nil
          end

        %ASTNode{rule_name: "slot_reference"} ->
          case first_token_value(child.children, "NAME") do
            ref_name when is_binary(ref_name) -> {:child, {:slot_ref_child, ref_name}}
            nil -> nil
          end

        %ASTNode{rule_name: "when_block"} ->
          {:child, analyze_when_block(child)}

        %ASTNode{rule_name: "each_block"} ->
          {:child, analyze_each_block(child)}

        _ ->
          nil
      end
    end)
  end

  # ============================================================================
  # Property Analysis
  # ============================================================================

  # property_assignment = ( NAME | KEYWORD ) COLON property_value SEMICOLON
  # Note: KEYWORD is allowed as a property name so that slot-type keywords
  # (e.g., "color", "text") can appear as style property names.
  defp analyze_property_assignment(%ASTNode{children: children}) do
    # Try NAME first; fall back to KEYWORD.
    prop_name =
      first_token_value(children, "NAME") ||
        first_token_value(children, "KEYWORD")

    if is_nil(prop_name) do
      raise "property_assignment missing name"
    end

    value_node = find_child(children, "property_value")

    if is_nil(value_node) do
      raise "property \"#{prop_name}\" missing value"
    end

    %MosaicProperty{
      name: prop_name,
      value: analyze_property_value(value_node)
    }
  end

  # property_value = slot_ref | STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD | NAME | enum_value
  defp analyze_property_value(%ASTNode{children: children}) do
    # Check ASTNode children first (slot_ref and enum_value are sub-rules).
    ast_result =
      Enum.find_value(children, fn child ->
        case child do
          %ASTNode{rule_name: "slot_ref"} ->
            case first_token_value(child.children, "NAME") do
              ref_name when is_binary(ref_name) -> {:slot_ref, ref_name}
              nil -> nil
            end

          %ASTNode{rule_name: "enum_value"} ->
            names = direct_token_values(child.children, "NAME")

            if length(names) >= 2 do
              {:enum_val, Enum.at(names, 0), Enum.at(names, 1)}
            else
              nil
            end

          _ ->
            nil
        end
      end)

    if ast_result do
      ast_result
    else
      # Fall back to leaf tokens.
      cond do
        (v = first_token_value(children, "STRING")) != nil ->
          {:string, v}

        (v = first_token_value(children, "DIMENSION")) != nil ->
          parse_dimension(v)

        (v = first_token_value(children, "NUMBER")) != nil ->
          {:number, parse_float(v)}

        (v = first_token_value(children, "COLOR_HEX")) != nil ->
          {:color_hex, v}

        (v = first_token_value(children, "KEYWORD")) != nil ->
          case v do
            "true" -> {:bool, true}
            "false" -> {:bool, false}
            _ -> {:ident, v}
          end

        (v = first_token_value(children, "NAME")) != nil ->
          {:ident, v}

        true ->
          raise "property_value has no recognizable content"
      end
    end
  end

  # ============================================================================
  # When / Each Block Analysis
  # ============================================================================

  # when_block = KEYWORD slot_ref LBRACE { node_content } RBRACE
  # The KEYWORD is "when"; the slot_ref gives the controlling bool slot.
  defp analyze_when_block(%ASTNode{children: children}) do
    slot_ref_node = find_child(children, "slot_ref")

    if is_nil(slot_ref_node) do
      raise "when_block missing slot_ref"
    end

    slot_name = first_token_value(slot_ref_node.children, "NAME")

    if is_nil(slot_name) do
      raise "when_block slot_ref missing name"
    end

    block_children = collect_node_content_children(children)
    {:when_block, slot_name, block_children}
  end

  # each_block = KEYWORD slot_ref KEYWORD NAME LBRACE { node_content } RBRACE
  # The first KEYWORD is "each"; the second KEYWORD is "as"; NAME is the loop variable.
  defp analyze_each_block(%ASTNode{children: children}) do
    slot_ref_node = find_child(children, "slot_ref")

    if is_nil(slot_ref_node) do
      raise "each_block missing slot_ref"
    end

    slot_name = first_token_value(slot_ref_node.children, "NAME")

    if is_nil(slot_name) do
      raise "each_block slot_ref missing slot name"
    end

    # Find the loop variable: a NAME token that appears after the "as" KEYWORD
    # in the direct children of each_block (not inside the slot_ref subtree).
    item_name = find_loop_variable(children, slot_ref_node)

    if is_nil(item_name) do
      raise "each_block missing loop variable name"
    end

    block_children = collect_node_content_children(children)
    {:each_block, slot_name, item_name, block_children}
  end

  # Collect all MosaicChild values from node_content children of a block node.
  defp collect_node_content_children(children) do
    children
    |> Enum.filter(fn child -> match?(%ASTNode{rule_name: "node_content"}, child) end)
    |> Enum.flat_map(fn content_node ->
      case analyze_node_content(content_node) do
        {:child, child_item} -> [child_item]
        _ -> []
      end
    end)
  end

  # Walk each_block's direct children to find the NAME token after "as".
  #
  # The structure is:
  #   KEYWORD("each")  slot_ref(...)  KEYWORD("as")  NAME(item)  LBRACE  ...
  #
  # We skip the slot_ref ASTNode and look for a NAME token that follows
  # a KEYWORD("as") token at the each_block level (not inside slot_ref).
  defp find_loop_variable(children, slot_ref_node) do
    {_, result} =
      Enum.reduce(children, {false, nil}, fn child, {after_as, found} ->
        if found do
          {after_as, found}
        else
          case child do
            # Skip the slot_ref subtree entirely.
            ^slot_ref_node ->
              {after_as, nil}

            # Direct token children.
            %Token{type: "KEYWORD", value: "as"} ->
              {true, nil}

            %Token{type: "NAME", value: name} when after_as ->
              {after_as, name}

            _ ->
              {after_as, nil}
          end
        end
      end)

    result
  end

  # ============================================================================
  # Value Parsing Helpers
  # ============================================================================

  # Parse a DIMENSION token like "16dp" → {:dimension, 16.0, "dp"}.
  # DIMENSION tokens always have a numeric part followed by a unit suffix.
  #
  #   "16dp"  → {:dimension, 16.0, "dp"}
  #   "1.5sp" → {:dimension, 1.5, "sp"}
  #   "100%"  → {:dimension, 100.0, "%"}
  defp parse_dimension(raw) do
    case Regex.run(~r/^(-?[0-9]*\.?[0-9]+)([a-zA-Z%]+)$/, raw) do
      [_, num_str, unit] ->
        {:dimension, parse_float(num_str), unit}

      nil ->
        raise "Invalid DIMENSION token: \"#{raw}\""
    end
  end

  defp parse_float(str) do
    {value, _} = Float.parse(str)
    value
  end

  # ============================================================================
  # AST Traversal Helpers
  # ============================================================================

  # The set of built-in layout and display elements.
  # When a node's tag name is in this set, is_primitive is true.
  defp primitive_nodes do
    MapSet.new([
      "Row", "Column", "Box", "Stack",
      "Text", "Image", "Icon",
      "Spacer", "Divider", "Scroll"
    ])
  end

  # Find the first direct-child ASTNode with the given rule_name.
  defp find_child(children, rule_name) do
    Enum.find(children, fn child ->
      match?(%ASTNode{rule_name: ^rule_name}, child)
    end)
  end

  # Collect all direct-child Token values with the given token type.
  defp direct_token_values(children, token_type) do
    children
    |> Enum.filter(fn child -> match?(%Token{type: ^token_type}, child) end)
    |> Enum.map(& &1.value)
  end

  # Get the first direct-child Token value with the given type, or nil.
  defp first_token_value(children, token_type) do
    case Enum.find(children, fn child -> match?(%Token{type: ^token_type}, child) end) do
      %Token{value: value} -> value
      nil -> nil
    end
  end
end
