defmodule CodingAdventures.LatticeAstToCss.Transformer do
  @moduledoc """
  Three-pass Lattice AST transformer — expands Lattice constructs into pure CSS.

  This is the core of the Lattice-to-CSS compiler. It takes a Lattice AST
  (containing both CSS and Lattice nodes) and produces a clean CSS AST
  (containing only CSS nodes) by expanding all Lattice constructs.

  ## Three-Pass Architecture

  **Pass 1 — Symbol Collection:**
  Walk the top-level AST and collect definitions:
  - Variable declarations → variable registry (the global `Scope`)
  - Mixin definitions → mixin registry (a `%{}` map)
  - Function definitions → function registry (a `%{}` map)
  Remove definition nodes from the AST (they produce no CSS output).

  **Pass 2 — Expansion:**
  Recursively walk remaining AST nodes with a scope chain:
  - Replace `VARIABLE` tokens with their resolved values
  - Expand `@include` directives by cloning mixin bodies
  - Evaluate `@if`/`@for`/`@each` control flow
  - Evaluate Lattice function calls and replace with return values

  After this pass, the AST contains only pure CSS nodes.

  **Pass 3 — Cleanup:**
  Remove any empty blocks or rules that resulted from transformation.

  ## Why Not a Single Pass?

  Mixins and functions can be defined after they're used:

      .btn { @include button(red); }   <- used first
      @mixin button($bg) { ... }       <- defined later

  Pass 1 collects all definitions up front, so Pass 2 can resolve them
  regardless of source order.

  ## Functional Design

  In Python, the transformer uses mutable state (dictionaries, object attributes).
  In Elixir, we thread all state through function arguments as part of a
  `%State{}` struct. This makes the data flow explicit and avoids side effects.

  ## Usage

      {:ok, css_ast} = Transformer.transform(lattice_ast)
      # css_ast is a clean ASTNode tree with no Lattice nodes
  """

  alias CodingAdventures.LatticeAstToCss.{Scope, Values, Evaluator, Builtins}
  alias CodingAdventures.LatticeAstToCss.Errors.{
    UndefinedVariableError,
    UndefinedMixinError,
    CircularReferenceError,
    WrongArityError,
    MissingReturnError,
    MaxIterationError
  }
  alias CodingAdventures.Parser.ASTNode
  alias CodingAdventures.Lexer.Token

  # ---------------------------------------------------------------------------
  # CSS built-in function names
  # ---------------------------------------------------------------------------
  #
  # These functions should NOT be resolved as Lattice functions. When a
  # function_call node uses one of these names, it's passed through unchanged.

  @css_functions MapSet.new([
    "rgb", "rgba", "hsl", "hsla", "hwb", "lab", "lch", "oklch", "oklab",
    "color", "color-mix",
    "calc", "min", "max", "clamp", "abs", "sign", "round", "mod", "rem",
    "sin", "cos", "tan", "asin", "acos", "atan", "atan2", "pow", "sqrt",
    "hypot", "log", "exp",
    "var", "env",
    "url", "format", "local",
    "linear-gradient", "radial-gradient", "conic-gradient",
    "repeating-linear-gradient", "repeating-radial-gradient",
    "repeating-conic-gradient",
    "counter", "counters", "attr", "element",
    "translate", "translateX", "translateY", "translateZ",
    "rotate", "rotateX", "rotateY", "rotateZ",
    "scale", "scaleX", "scaleY", "scaleZ",
    "skew", "skewX", "skewY",
    "matrix", "matrix3d", "perspective",
    "cubic-bezier", "steps",
    "path", "polygon", "circle", "ellipse", "inset",
    "image-set", "cross-fade",
    "fit-content", "minmax", "repeat",
    "blur", "brightness", "contrast", "drop-shadow", "grayscale",
    "hue-rotate", "invert", "opacity", "saturate", "sepia"
  ])

  defp css_function?(name) do
    # FUNCTION token includes "(" at the end: "rgb(" → "rgb"
    clean = String.trim_trailing(name, "(")
    MapSet.member?(@css_functions, clean)
  end

  # ---------------------------------------------------------------------------
  # Transformer state
  # ---------------------------------------------------------------------------

  # We thread this struct through the expansion phase.
  # - `variables` is the global Scope for variable bindings
  # - `mixins` maps name → %{params, defaults, body}
  # - `functions` maps name → %{params, defaults, body}
  # - `mixin_stack` tracks current call stack for cycle detection
  # - `function_stack` tracks function call stack for cycle detection

  # Maximum iterations for @while loops (prevents infinite loops)
  @max_while_iterations 1000

  defmodule State do
    @moduledoc false
    defstruct [
      variables: nil,    # Scope.t() — global scope
      mixins: %{},
      functions: %{},
      mixin_stack: [],
      function_stack: [],
      # Lattice v2: @content block tracking
      content_block_stack: [],   # stack of content blocks (or nil)
      content_scope_stack: [],   # stack of caller scopes for @content
      # Lattice v2: @extend tracking
      extend_map: %{},           # target selector -> list of extending selectors
      # Lattice v2: @at-root hoisted rules
      at_root_rules: [],         # rules collected from @at-root
      # Lattice v2: max while iterations
      max_while_iterations: 1000
    ]
  end

  # ---------------------------------------------------------------------------
  # Public API
  # ---------------------------------------------------------------------------

  @doc """
  Transform a Lattice AST into a clean CSS AST.

  Runs the three-pass pipeline:
  1. Collect symbols (variables, mixins, functions)
  2. Expand all Lattice constructs
  3. Clean up empty nodes

  ## Parameters

  - `ast` — the root `stylesheet` `%ASTNode{}` from the Lattice parser

  ## Returns

  - `{:ok, css_ast}` — a clean CSS AST with no Lattice nodes
  - `{:error, message}` — if a Lattice error occurred (undefined variable, etc.)

  ## Example

      {:ok, ast} = CodingAdventures.LatticeParser.parse(source)
      {:ok, css_ast} = Transformer.transform(ast)
  """
  @spec transform(ASTNode.t()) :: {:ok, ASTNode.t()} | {:error, String.t()}
  def transform(%ASTNode{} = ast) do
    try do
      state = %State{variables: Scope.new(), max_while_iterations: @max_while_iterations}

      # Pass 1: collect symbols (variables, mixins, functions, @extend)
      {pruned_ast, state} = collect_symbols(ast, state)

      # Pass 2: expand Lattice constructs
      {expanded_ast, final_state} = expand_node(pruned_ast, state.variables, state)

      # Pass 3: cleanup + @extend selector merging + @at-root hoisting
      cleaned = cleanup(expanded_ast)

      # Remove placeholder-only rules if @extend was used
      cleaned = if map_size(final_state.extend_map) > 0 do
        remove_placeholder_rules(cleaned)
      else
        cleaned
      end

      # Splice @at-root hoisted rules into root stylesheet
      cleaned = if final_state.at_root_rules != [] do
        splice_at_root_rules(cleaned, final_state.at_root_rules)
      else
        cleaned
      end

      {:ok, cleaned}
    catch
      {:lattice_error, err} ->
        {:error, err.message}
    end
  end

  # ---------------------------------------------------------------------------
  # Pass 1: Symbol Collection
  # ---------------------------------------------------------------------------

  # Walk top-level stylesheet children and collect variable/mixin/function
  # definitions. These nodes produce no CSS output — they're removed from
  # the AST children after collection.

  defp collect_symbols(%ASTNode{rule_name: "stylesheet", children: children} = ast, state) do
    {kept_children, state} = Enum.reduce(children, {[], state}, fn child, {kept, st} ->
      case try_collect_definition(child, st) do
        {:collected, new_state} ->
          # Definition node — don't keep it in the output
          {kept, new_state}
        :not_definition ->
          # Regular node — keep it
          {[child | kept], st}
      end
    end)

    new_ast = %{ast | children: Enum.reverse(kept_children)}
    {new_ast, state}
  end

  defp collect_symbols(ast, state), do: {ast, state}

  # Try to collect a top-level definition node.
  # Returns {:collected, new_state} if this node is a definition,
  # or :not_definition if it should be kept in the output.

  defp try_collect_definition(%ASTNode{rule_name: "rule", children: [inner]} = _rule, state) do
    case inner do
      %ASTNode{rule_name: "lattice_rule", children: [lattice_child]} ->
        case lattice_child do
          %ASTNode{rule_name: "variable_declaration"} ->
            new_state = collect_variable(lattice_child, state)
            {:collected, new_state}

          %ASTNode{rule_name: "mixin_definition"} ->
            new_state = collect_mixin(lattice_child, state)
            {:collected, new_state}

          %ASTNode{rule_name: "function_definition"} ->
            new_state = collect_function(lattice_child, state)
            {:collected, new_state}

          %ASTNode{rule_name: "use_directive"} ->
            # @use — skip for now (module resolution not implemented)
            {:collected, state}

          _ ->
            :not_definition
        end

      _ ->
        :not_definition
    end
  end

  defp try_collect_definition(_, _state), do: :not_definition

  # variable_declaration = VARIABLE COLON value_list { variable_flag } SEMICOLON ;
  # Lattice v2: handles !default and !global flags
  defp collect_variable(%ASTNode{children: children}, state) do
    name = find_token_value(children, "VARIABLE")
    value_node = find_child_by_rule(children, "value_list")
    {is_default, is_global} = extract_variable_flags(children)

    if name && value_node do
      cond do
        is_default and is_global ->
          # Check global scope only -- if not defined there, set globally
          root = get_root_scope(state.variables)
          if Scope.get(root, name) == :error do
            new_vars = Scope.set_global(state.variables, name, value_node)
            %{state | variables: new_vars}
          else
            state
          end
        is_default ->
          # Only set if not already defined anywhere
          if Scope.get(state.variables, name) == :error do
            new_vars = Scope.set(state.variables, name, value_node)
            %{state | variables: new_vars}
          else
            state
          end
        is_global ->
          # Always set in global scope
          new_vars = Scope.set_global(state.variables, name, value_node)
          %{state | variables: new_vars}
        true ->
          new_vars = Scope.set(state.variables, name, value_node)
          %{state | variables: new_vars}
      end
    else
      state
    end
  end

  # Extract !default and !global flags from variable_declaration children
  defp extract_variable_flags(children) do
    Enum.reduce(children, {false, false}, fn child, {is_def, is_glob} ->
      case child do
        %Token{type: "BANG_DEFAULT"} -> {true, is_glob}
        %Token{type: "BANG_GLOBAL"} -> {is_def, true}
        %ASTNode{rule_name: "variable_flag", children: flag_children} ->
          Enum.reduce(flag_children, {is_def, is_glob}, fn fc, {d, g} ->
            case fc do
              %Token{type: "BANG_DEFAULT"} -> {true, g}
              %Token{type: "BANG_GLOBAL"} -> {d, true}
              _ -> {d, g}
            end
          end)
        _ -> {is_def, is_glob}
      end
    end)
  end

  # Walk up the scope chain to find the root (global) scope
  defp get_root_scope(%Scope{parent: nil} = scope), do: scope
  defp get_root_scope(%Scope{parent: parent}), do: get_root_scope(parent)

  # mixin_definition = "@mixin" FUNCTION [ mixin_params ] RPAREN block ;
  defp collect_mixin(%ASTNode{children: children}, state) do
    func_token = find_token(children, "FUNCTION")
    name = if func_token, do: String.trim_trailing(func_token.value, "("), else: nil
    params_node = find_child_by_rule(children, "mixin_params")
    body = find_child_by_rule(children, "block")

    if name && body do
      {params, defaults} = extract_params(params_node)
      mixin_def = %{name: name, params: params, defaults: defaults, body: body}
      %{state | mixins: Map.put(state.mixins, name, mixin_def)}
    else
      state
    end
  end

  # function_definition = "@function" FUNCTION [ mixin_params ] RPAREN function_body ;
  defp collect_function(%ASTNode{children: children}, state) do
    func_token = find_token(children, "FUNCTION")
    name = if func_token, do: String.trim_trailing(func_token.value, "("), else: nil
    params_node = find_child_by_rule(children, "mixin_params")
    body = find_child_by_rule(children, "function_body")

    if name && body do
      {params, defaults} = extract_params(params_node)
      func_def = %{name: name, params: params, defaults: defaults, body: body}
      %{state | functions: Map.put(state.functions, name, func_def)}
    else
      state
    end
  end

  # mixin_params = mixin_param { COMMA mixin_param } ;
  # mixin_param = VARIABLE [ COLON value_list ] ;
  defp extract_params(nil), do: {[], %{}}

  defp extract_params(%ASTNode{children: children}) do
    param_nodes = Enum.filter(children, fn
      %ASTNode{rule_name: "mixin_param"} -> true
      _ -> false
    end)

    Enum.reduce(param_nodes, {[], %{}}, fn %ASTNode{children: pc}, {params, defaults} ->
      var_token = find_token(pc, "VARIABLE")
      default_node = find_child_by_rule(pc, "mixin_value_list") || find_child_by_rule(pc, "value_list")

      if var_token do
        new_params = params ++ [var_token.value]
        new_defaults =
          if default_node do
            Map.put(defaults, var_token.value, default_node)
          else
            defaults
          end
        {new_params, new_defaults}
      else
        {params, defaults}
      end
    end)
  end

  # ---------------------------------------------------------------------------
  # Pass 2: Expansion
  # ---------------------------------------------------------------------------
  #
  # expand_node/3 walks the AST, dispatching on rule_name for Lattice-specific
  # nodes. For CSS nodes, it just recursively expands children.
  #
  # Because Elixir is immutable, we can't mutate the AST in place. Instead,
  # we build new AST nodes with updated children lists.
  #
  # Returns {expanded_node, state} where state may have been updated by
  # variable declarations encountered during expansion.

  defp expand_node(%Token{type: "VARIABLE"} = token, scope, state) do
    # Variable reference — resolve to its value
    {substitute_variable(token, scope), state}
  end

  defp expand_node(%Token{} = token, _scope, state) do
    {token, state}
  end

  defp expand_node(%ASTNode{rule_name: rule_name} = node, scope, state) do
    case rule_name do
      "block" ->
        # Create a child scope for block contents
        child_scope = Scope.child(scope)
        expand_children(node, child_scope, state)

      "block_contents" ->
        expand_block_contents(node, scope, state)

      "block_item" ->
        expand_block_item(node, scope, state)

      "value_list" ->
        expand_value_list(node, scope, state)

      "value" ->
        expand_value_node(node, scope, state)

      "function_call" ->
        expand_function_call(node, scope, state)

      "function_arg" ->
        expand_children(node, scope, state)

      "function_args" ->
        expand_children(node, scope, state)

      # Lattice v2: resolve variables in selector positions
      r when r in ["compound_selector", "simple_selector", "class_selector"] ->
        expand_selector_with_vars(node, scope, state)

      # ---------------------------------------------------------------------------
      # Top-level Lattice nodes (in stylesheet context — not inside blocks)
      # ---------------------------------------------------------------------------
      #
      # A "rule" that wraps a "lattice_rule" containing lattice_control or
      # include_directive must be expanded here, at the top level.
      # The result is a flat list of CSS rule nodes (or nil for no output).

      "rule" ->
        case node.children do
          [%ASTNode{rule_name: "lattice_rule"} = lr] ->
            expand_top_level_lattice_rule(lr, scope, state)
          _ ->
            expand_children(node, scope, state)
        end

      "lattice_rule" ->
        # Should have been handled by "rule" above, but handle directly too
        expand_top_level_lattice_rule(node, scope, state)

      "lattice_control" ->
        expand_control(node, scope, state)

      "include_directive" ->
        # @include at top level (unusual but handle it)
        expand_include(node, scope, state)

      _ ->
        expand_children(node, scope, state)
    end
  end

  defp expand_node(other, _scope, state), do: {other, state}

  # Expand a top-level lattice_rule (one that appears directly in the stylesheet,
  # not inside a block). These contain:
  #   - lattice_control  (@if, @for, @each) — produces a list of CSS rule nodes
  #   - include_directive (@include) — produces a list of CSS rule nodes
  #   - variable_declaration — already consumed in Pass 1, but handle gracefully
  #   - mixin/function/use definitions — already consumed in Pass 1
  #
  # At the top level, the result from control flow / @include is a list of
  # block_item nodes (from expand_block_to_items). We need to "lift" those to
  # stylesheet-level rule nodes by extracting the CSS content and wrapping it.
  defp expand_top_level_lattice_rule(%ASTNode{children: [inner | _]}, scope, state) do
    case inner do
      %ASTNode{rule_name: "lattice_control"} ->
        {block_items, new_state} = expand_control(inner, scope, state)
        items = lift_block_items_to_rules(block_items, scope, new_state)
        {items, new_state}

      %ASTNode{rule_name: "include_directive"} ->
        {block_items, new_state} = expand_include(inner, scope, state)
        items = lift_block_items_to_rules(block_items, scope, new_state)
        {items, new_state}

      %ASTNode{rule_name: "variable_declaration"} ->
        # Variable declaration at top level was already handled in Pass 1
        {nil, state}

      %ASTNode{rule_name: name} when name in ["mixin_definition", "function_definition", "use_directive"] ->
        # Already consumed in Pass 1
        {nil, state}

      _ ->
        {nil, state}
    end
  end

  defp expand_top_level_lattice_rule(_, _scope, state), do: {nil, state}

  # Convert a list of block_item nodes (from @if/@for/@each expansion) into
  # top-level CSS rule nodes suitable for the stylesheet context.
  #
  # A block_item at the top level contains:
  #   block_item → declaration_or_nested → qualified_rule  (most common)
  #   block_item → at_rule
  #   block_item → lattice_block_item  (shouldn't occur after expansion)
  #
  # We extract the actual CSS rule and wrap it in a `rule` node.
  defp lift_block_items_to_rules(block_items, _scope, _state) when is_list(block_items) do
    Enum.flat_map(block_items, fn item ->
      case item do
        %ASTNode{rule_name: "block_item", children: [inner | _]} ->
          lift_inner_to_rule(inner)

        %ASTNode{rule_name: "rule"} ->
          # Already a rule node — keep as-is
          [item]

        %ASTNode{rule_name: "qualified_rule"} ->
          [%ASTNode{rule_name: "rule", children: [item]}]

        %ASTNode{rule_name: "at_rule"} ->
          [%ASTNode{rule_name: "rule", children: [item]}]

        _ ->
          []
      end
    end)
  end

  defp lift_block_items_to_rules(nil, _scope, _state), do: []
  defp lift_block_items_to_rules([], _scope, _state), do: []

  # Lift a block_item's inner node to a top-level rule
  defp lift_inner_to_rule(%ASTNode{rule_name: "declaration_or_nested", children: [inner2 | _]}) do
    case inner2 do
      %ASTNode{rule_name: "qualified_rule"} ->
        [%ASTNode{rule_name: "rule", children: [inner2]}]
      %ASTNode{rule_name: "at_rule"} ->
        [%ASTNode{rule_name: "rule", children: [inner2]}]
      _ ->
        []
    end
  end

  defp lift_inner_to_rule(%ASTNode{rule_name: "at_rule"} = n) do
    [%ASTNode{rule_name: "rule", children: [n]}]
  end

  defp lift_inner_to_rule(%ASTNode{rule_name: "qualified_rule"} = n) do
    [%ASTNode{rule_name: "rule", children: [n]}]
  end

  defp lift_inner_to_rule(_), do: []

  # Expand all children of a node, collecting state updates
  defp expand_children(%ASTNode{children: children} = node, scope, state) do
    {new_children, new_state} = Enum.reduce(children, {[], state}, fn child, {acc, st} ->
      {expanded, new_st} = expand_node(child, scope, st)
      case expanded do
        nil -> {acc, new_st}
        items when is_list(items) -> {acc ++ items, new_st}
        item -> {acc ++ [item], new_st}
      end
    end)

    {%{node | children: new_children}, new_state}
  end

  # Substitute a VARIABLE token with its resolved value
  defp substitute_variable(%Token{value: name} = token, scope) do
    case Scope.get(scope, name) do
      :error ->
        throw({:lattice_error, UndefinedVariableError.new(
          name,
          Map.get(token, :line, 0),
          Map.get(token, :column, 0)
        )})

      {:ok, %ASTNode{} = value_node} ->
        # Deep-copy the node (we may expand it multiple times)
        value_node

      {:ok, value} when is_tuple(value) or value == :null ->
        # It's already a lattice_value — make a synthetic token
        make_synthetic_token(Values.to_css(value), token)

      {:ok, %Token{} = resolved} ->
        resolved

      {:ok, other} ->
        make_synthetic_token(to_string(other), token)
    end
  end

  # Create a synthetic token with the given text, copying position from a template
  defp make_synthetic_token(text, template_token) do
    %Token{
      type: "IDENT",
      value: text,
      line: Map.get(template_token, :line, 0),
      column: Map.get(template_token, :column, 0)
    }
  end

  # Expand block_contents, handling Lattice block items specially.
  # Variable declarations, @include, and control flow can produce lists
  # of block items or no output at all.
  defp expand_block_contents(%ASTNode{children: children} = node, scope, state) do
    {new_children, new_state} = Enum.reduce(children, {[], state}, fn child, {acc, st} ->
      {result, new_st} = expand_block_item_inner(child, scope, st)

      case result do
        nil -> {acc, new_st}
        items when is_list(items) -> {acc ++ items, new_st}
        item -> {acc ++ [item], new_st}
      end
    end)

    {%{node | children: new_children}, new_state}
  end

  defp expand_block_item_inner(%ASTNode{rule_name: "block_item", children: [inner | _]} = item, scope, state) do
    case inner do
      %ASTNode{rule_name: "lattice_block_item", children: [lattice_inner | _]} ->
        case expand_lattice_block_item(lattice_inner, scope, state) do
          {nil, new_st} -> {nil, new_st}
          {items, new_st} when is_list(items) -> {items, new_st}
          {result, new_st} ->
            new_item = %{item | children: [%{inner | children: [result]}]}
            {new_item, new_st}
        end

      # Lattice v2: handle property_nesting inside declaration_or_nested
      %ASTNode{rule_name: "declaration_or_nested", children: [%ASTNode{rule_name: "property_nesting"} = pn | _]} ->
        {result, new_st} = expand_property_nesting(pn, scope, state)
        {result, new_st}

      _ ->
        expand_children(item, scope, state)
    end
  end

  defp expand_block_item_inner(child, scope, state) do
    expand_node(child, scope, state)
  end

  defp expand_block_item(%ASTNode{children: [inner | _]} = node, scope, state) do
    case inner do
      %ASTNode{rule_name: "lattice_block_item", children: [lattice_inner | _]} ->
        case expand_lattice_block_item(lattice_inner, scope, state) do
          {nil, new_st} -> {nil, new_st}
          {items, new_st} when is_list(items) -> {items, new_st}
          {result, new_st} ->
            {%{node | children: [%{inner | children: [result]}]}, new_st}
        end

      _ ->
        expand_children(node, scope, state)
    end
  end

  defp expand_block_item(node, scope, state), do: expand_children(node, scope, state)

  # Expand a lattice_block_item (variable_declaration | include_directive | lattice_control)
  defp expand_lattice_block_item(%ASTNode{rule_name: "variable_declaration"} = node, scope, state) do
    # Process variable declaration: set in scope, remove from output
    new_scope = expand_variable_declaration(node, scope)
    # Return nil (don't output) but pass updated scope via state
    # NOTE: Since scope is immutable and threaded separately, we update state.variables
    # For block-level variables, we need a different approach — we thread the scope
    # through in state as a "current scope override"
    # We store updated scope in state for the current block only
    {nil, %{state | variables: new_scope}}
  end

  defp expand_lattice_block_item(%ASTNode{rule_name: "include_directive"} = node, scope, state) do
    {items, new_state} = expand_include(node, scope, state)
    {items, new_state}
  end

  defp expand_lattice_block_item(%ASTNode{rule_name: "lattice_control"} = node, scope, state) do
    expand_control(node, scope, state)
  end

  # Lattice v2: @content directive
  defp expand_lattice_block_item(%ASTNode{rule_name: "content_directive"}, _scope, state) do
    expand_content(state)
  end

  # Lattice v2: @at-root directive
  defp expand_lattice_block_item(%ASTNode{rule_name: "at_root_directive"} = node, scope, state) do
    expand_at_root(node, scope, state)
  end

  # Lattice v2: @extend directive
  defp expand_lattice_block_item(%ASTNode{rule_name: "extend_directive"} = node, scope, state) do
    new_state = collect_extend(node, scope, state)
    {nil, new_state}
  end

  defp expand_lattice_block_item(node, scope, state) do
    expand_children(node, scope, state)
  end

  # Expand a variable_declaration by binding in the current scope.
  # Returns the updated scope (the node is not added to output).
  # Lattice v2: handles !default and !global flags.
  defp expand_variable_declaration(%ASTNode{children: children}, scope) do
    name = find_token_value(children, "VARIABLE")
    value_node = find_child_by_rule(children, "value_list")
    {is_default, is_global} = extract_variable_flags(children)

    if name && value_node do
      # Try to evaluate the value as an expression (e.g. $i + 1 → {:number, 2}).
      # This is critical for @while loops: without it, $i: $i + 1
      # stores unevaluated AST tokens instead of the computed number, causing
      # the loop condition to never change and looping forever.
      stored_value =
        try do
          evaluated = Evaluator.evaluate(deep_copy(value_node), scope)
          if is_tuple(evaluated) or evaluated == :null, do: evaluated, else: value_node
        rescue
          _ -> value_node
        catch
          _, _ -> value_node
        end

      cond do
        is_default and is_global ->
          root = get_root_scope(scope)
          if Scope.get(root, name) == :error do
            Scope.set_global(scope, name, stored_value)
          else
            scope
          end
        is_default ->
          if Scope.get(scope, name) == :error do
            Scope.set(scope, name, stored_value)
          else
            scope
          end
        is_global ->
          Scope.set_global(scope, name, stored_value)
        true ->
          Scope.set(scope, name, stored_value)
      end
    else
      scope
    end
  end

  # Expand value_list: expand variables within the list
  defp expand_value_list(%ASTNode{children: children} = node, scope, state) do
    {new_children, new_state} = Enum.reduce(children, {[], state}, fn child, {acc, st} ->
      {expanded, new_st} = expand_node(child, scope, st)

      case expanded do
        %ASTNode{rule_name: "value_list", children: sub_children} ->
          # Splice the inner value_list's children into the outer one
          {acc ++ sub_children, new_st}
        nil -> {acc, new_st}
        item -> {acc ++ [item], new_st}
      end
    end)

    {%{node | children: new_children}, new_state}
  end

  # Expand a single value node
  defp expand_value_node(%ASTNode{children: [%Token{type: "VARIABLE"} = var_token]} = node, scope, state) do
    result = substitute_variable(var_token, scope)

    case result do
      %ASTNode{rule_name: "value_list"} = vl ->
        # Return the value_list directly — it will be spliced by expand_value_list
        {vl, state}
      _ ->
        {%{node | children: [result]}, state}
    end
  end

  defp expand_value_node(node, scope, state), do: expand_children(node, scope, state)

  # Expand a function_call node.
  # CSS built-ins → expand args but keep structure.
  # Lattice functions → evaluate and replace with return value.
  defp expand_function_call(%ASTNode{children: children} = node, scope, state) do
    func_token = find_token(children, "FUNCTION")

    case func_token do
      nil ->
        # URL_TOKEN — pass through
        expand_children(node, scope, state)

      %Token{value: func_name_with_paren} ->
        func_name = String.trim_trailing(func_name_with_paren, "(")

        cond do
          # User-defined function ALWAYS takes priority — even over CSS built-ins
          # like scale(), translate(), etc. If the user defines @function scale(),
          # their definition wins. This matches Sass behavior.
          Map.has_key?(state.functions, func_name) ->
            evaluate_function_call(func_name, node, scope, state)

          # CSS built-in that is NOT also a Lattice built-in — pass through
          css_function?(func_name) and not Builtins.builtin?(func_name) ->
            expand_children(node, scope, state)

          # Lattice v2 built-in function
          Builtins.builtin?(func_name) ->
            evaluate_builtin_function(func_name, node, scope, state)

          # CSS built-in that overlaps with Lattice built-in names
          css_function?(func_name) ->
            expand_children(node, scope, state)

          true ->
            # Unknown function — pass through
            expand_children(node, scope, state)
        end
    end
  end

  # ---------------------------------------------------------------------------
  # @include Expansion
  # ---------------------------------------------------------------------------

  # include_directive = "@include" FUNCTION include_args RPAREN ( SEMICOLON | block )
  #                   | "@include" IDENT ( SEMICOLON | block ) ;
  defp expand_include(%ASTNode{children: children}, scope, state) do
    # Extract mixin name, args, and content block
    mixin_name =
      case find_token(children, "FUNCTION") do
        nil ->
          ident = find_token(children, "IDENT")
          if ident, do: ident.value, else: nil
        %Token{value: name_with_paren} ->
          String.trim_trailing(name_with_paren, "(")
      end

    # Lattice v2: detect content block (trailing block after args)
    content_block = find_child_by_rule(children, "block")

    if is_nil(mixin_name) do
      {[], state}
    else
      unless Map.has_key?(state.mixins, mixin_name) do
        throw({:lattice_error, UndefinedMixinError.new(mixin_name)})
      end

      if mixin_name in state.mixin_stack do
        throw({:lattice_error, CircularReferenceError.new(
          "mixin",
          state.mixin_stack ++ [mixin_name]
        )})
      end

      mixin_def = state.mixins[mixin_name]
      args_node = find_child_by_rule(children, "include_args")
      raw_args = if args_node, do: parse_include_args(args_node), else: []

      # Expand variable references in the args using the current (caller) scope.
      args = Enum.map(raw_args, fn arg ->
        {expanded_arg, _} = expand_value_list(arg, scope, state)
        expanded_arg
      end)

      # Check arity
      required = length(mixin_def.params) - map_size(mixin_def.defaults)
      if length(args) < required or length(args) > length(mixin_def.params) do
        throw({:lattice_error, WrongArityError.new(
          "Mixin", mixin_name, length(mixin_def.params), length(args)
        )})
      end

      # Create child scope with params bound
      mixin_scope = bind_params(mixin_def.params, mixin_def.defaults, args, scope)

      # Track call stack for cycle detection
      # Lattice v2: push content block and caller scope for @content
      new_state = %{state |
        mixin_stack: [mixin_name | state.mixin_stack],
        content_block_stack: [content_block | state.content_block_stack],
        content_scope_stack: [scope | state.content_scope_stack]
      }

      # Clone and expand the mixin body
      body_clone = deep_copy(mixin_def.body)
      {expanded_body, _new_state2} = expand_node(body_clone, mixin_scope, new_state)

      # Pop the call stack and content stacks after expansion
      final_state = %{state |
        mixin_stack: state.mixin_stack,
        content_block_stack: state.content_block_stack,
        content_scope_stack: state.content_scope_stack
      }

      # Extract the block_contents children (the actual CSS to splice in)
      items = extract_block_contents_children(expanded_body)
      {items, final_state}
    end
  end

  # Extract all children from block_contents of a block node
  defp extract_block_contents_children(%ASTNode{children: children}) do
    Enum.flat_map(children, fn
      %ASTNode{rule_name: "block_contents", children: items} -> items
      _ -> []
    end)
  end

  # Parse include_args = value_list { COMMA value_list }
  # Due to grammar ambiguity, commas may appear inside a single value_list.
  # We split on commas to produce multiple arg value_lists.
  defp parse_include_args(%ASTNode{children: children}) do
    value_lists = Enum.filter(children, fn
      %ASTNode{rule_name: "value_list"} -> true
      _ -> false
    end)

    case value_lists do
      [] -> []
      [single] -> split_value_list_on_commas(single)
      multiple -> multiple
    end
  end

  # Split a value_list into multiple at COMMA boundaries
  defp split_value_list_on_commas(%ASTNode{children: children} = node) do
    has_comma = Enum.any?(children, fn
      %ASTNode{rule_name: "value", children: [%Token{type: "COMMA"}]} -> true
      _ -> false
    end)

    unless has_comma do
      [node]
    else
      # Group children by commas
      groups =
        Enum.reduce(children, [[]], fn child, [current | rest] ->
          case child do
            %ASTNode{rule_name: "value", children: [%Token{type: "COMMA"}]} ->
              [[] | [current | rest]]
            _ ->
              [[child | current] | rest]
          end
        end)
        |> Enum.map(&Enum.reverse/1)
        |> Enum.reverse()

      # Create a value_list node for each group
      groups
      |> Enum.filter(fn g -> g != [] end)
      |> Enum.map(fn group ->
        %ASTNode{rule_name: "value_list", children: group}
      end)
    end
  end

  # Bind parameter names to argument values in a new child scope
  defp bind_params(params, defaults, args, parent_scope) do
    scope = Scope.child(parent_scope)

    Enum.reduce(Enum.with_index(params), scope, fn {param_name, i}, sc ->
      value =
        cond do
          i < length(args) -> Enum.at(args, i)
          Map.has_key?(defaults, param_name) -> deep_copy(defaults[param_name])
          true -> :null
        end

      Scope.set(sc, param_name, value)
    end)
  end

  # ---------------------------------------------------------------------------
  # Control Flow Expansion
  # ---------------------------------------------------------------------------

  # lattice_control = if_directive | for_directive | each_directive | while_directive ;
  # Lattice v2 adds while_directive.
  defp expand_control(%ASTNode{children: [inner | _]}, scope, state) do
    case inner do
      %ASTNode{rule_name: "if_directive"} -> expand_if(inner, scope, state)
      %ASTNode{rule_name: "for_directive"} -> expand_for(inner, scope, state)
      %ASTNode{rule_name: "each_directive"} -> expand_each(inner, scope, state)
      %ASTNode{rule_name: "while_directive"} -> expand_while(inner, scope, state)
      _ -> {[], state}
    end
  end

  defp expand_control(_, _scope, state), do: {[], state}

  # if_directive = "@if" lattice_expression block
  #               { "@else" "if" lattice_expression block }
  #               [ "@else" block ] ;
  defp expand_if(%ASTNode{children: children}, scope, state) do
    # Parse the if/else-if/else structure into branches
    branches = parse_if_branches(children)

    # Evaluate each branch condition until one is truthy
    result =
      Enum.reduce_while(branches, nil, fn {condition, block}, _acc ->
        case condition do
          nil ->
            # @else — always matches
            {:halt, block}
          expr_node ->
            value = Evaluator.evaluate(expr_node, scope)
            if Values.truthy?(value) do
              {:halt, block}
            else
              {:cont, nil}
            end
        end
      end)

    case result do
      nil ->
        {[], state}
      block ->
        expand_block_to_items(block, scope, state)
    end
  end

  # Parse the children of an if_directive into [{condition | nil, block}] tuples.
  # nil condition means @else (always matches).
  defp parse_if_branches(children) do
    parse_if_branches(children, [])
  end

  defp parse_if_branches([], acc), do: Enum.reverse(acc)

  defp parse_if_branches([%Token{value: "@if"} | rest], acc) do
    case rest do
      [expr, block | remaining] ->
        parse_if_branches(remaining, [{expr, block} | acc])
      _ ->
        Enum.reverse(acc)
    end
  end

  defp parse_if_branches([%Token{value: "@else"} | rest], acc) do
    case rest do
      [%Token{value: "if"}, expr, block | remaining] ->
        # @else if
        parse_if_branches(remaining, [{expr, block} | acc])
      [block | remaining] ->
        # @else
        parse_if_branches(remaining, [{nil, block} | acc])
      _ ->
        Enum.reverse(acc)
    end
  end

  defp parse_if_branches([_ | rest], acc), do: parse_if_branches(rest, acc)

  # for_directive = "@for" VARIABLE "from" lattice_expression
  #                 ( "through" | "to" ) lattice_expression block ;
  defp expand_for(%ASTNode{children: children}, scope, state) do
    var_name = find_token_value(children, "VARIABLE")
    block = find_child_by_rule(children, "block")

    # Find "from", then an expression, then "through"/"to", then another expression
    {from_expr, to_expr, is_through} = parse_for_bounds(children)

    if is_nil(var_name) or is_nil(from_expr) or is_nil(to_expr) or is_nil(block) do
      {[], state}
    else
      from_val = Evaluator.evaluate(from_expr, scope)
      to_val = Evaluator.evaluate(to_expr, scope)

      from_num = extract_number(from_val)
      to_num = extract_number(to_val)

      end_val = if is_through, do: to_num + 1, else: to_num

      {items, final_state} = Enum.reduce(from_num..(end_val - 1), {[], state}, fn i, {acc, st} ->
        loop_scope = Scope.set(Scope.child(scope), var_name, {:number, i * 1.0})
        {new_items, new_st} = expand_block_to_items(deep_copy(block), loop_scope, st)
        {acc ++ new_items, new_st}
      end)

      {items, final_state}
    end
  end

  defp parse_for_bounds(children) do
    # Walk children to find: from <expr> (through|to) <expr>
    parse_for_bounds(children, nil, nil, nil, false)
  end

  defp parse_for_bounds([], from_expr, to_expr, _state, is_through) do
    {from_expr, to_expr, is_through}
  end

  defp parse_for_bounds([%Token{value: "from"} | [next | rest]], _from, to_expr, _st, is_through) do
    parse_for_bounds(rest, next, to_expr, :found_from, is_through)
  end

  defp parse_for_bounds([%Token{value: "through"} | [next | rest]], from_expr, _to, :found_from, _) do
    parse_for_bounds(rest, from_expr, next, :found_to, true)
  end

  defp parse_for_bounds([%Token{value: "to"} | [next | rest]], from_expr, _to, :found_from, _) do
    parse_for_bounds(rest, from_expr, next, :found_to, false)
  end

  defp parse_for_bounds([_ | rest], from_expr, to_expr, st, is_through) do
    parse_for_bounds(rest, from_expr, to_expr, st, is_through)
  end

  defp extract_number({:number, n}), do: trunc(n)
  defp extract_number({:dimension, n, _}), do: trunc(n)
  defp extract_number(_), do: 0

  # each_directive = "@each" VARIABLE { COMMA VARIABLE } "in" each_list block ;
  defp expand_each(%ASTNode{children: children}, scope, state) do
    var_names =
      children
      |> Enum.filter(fn %Token{type: "VARIABLE"} -> true; _ -> false end)
      |> Enum.map(fn %Token{value: v} -> v end)

    each_list = find_child_by_rule(children, "each_list")
    block = find_child_by_rule(children, "block")

    if var_names == [] or is_nil(each_list) or is_nil(block) do
      {[], state}
    else
      items = extract_each_list_items(each_list)
      primary_var = hd(var_names)

      {result, final_state} = Enum.reduce(items, {[], state}, fn item, {acc, st} ->
        item_value = extract_value_token(item)
        loop_scope = Scope.set(Scope.child(scope), primary_var, item_value)
        {new_items, new_st} = expand_block_to_items(deep_copy(block), loop_scope, st)
        {acc ++ new_items, new_st}
      end)

      {result, final_state}
    end
  end

  # each_list = value { COMMA value } ;
  # Returns the value nodes (not the COMMA tokens)
  defp extract_each_list_items(%ASTNode{children: children}) do
    Enum.filter(children, fn
      %ASTNode{rule_name: "value"} -> true
      _ -> false
    end)
  end

  # Extract a lattice_value from a value node
  defp extract_value_token(%ASTNode{children: [%Token{} = token | _]}) do
    Values.token_to_value(token)
  end

  defp extract_value_token(%ASTNode{children: [child | _]}), do: extract_value_token(child)
  defp extract_value_token(_), do: :null

  # Expand a block and return its block_contents children as a flat list
  defp expand_block_to_items(%ASTNode{} = block, scope, state) do
    {expanded, new_state} = expand_node(block, scope, state)

    items = Enum.flat_map(Map.get(expanded, :children, []), fn
      %ASTNode{rule_name: "block_contents", children: items} -> items
      _ -> []
    end)

    {items, new_state}
  end

  # ---------------------------------------------------------------------------
  # Function Evaluation
  # ---------------------------------------------------------------------------

  # Evaluate a Lattice function call and replace the call with the return value.
  # The function body is evaluated in an isolated scope (parent = globals only).
  defp evaluate_function_call(func_name, %ASTNode{children: children} = node, _scope, state) do
    func_def = state.functions[func_name]

    # Parse arguments
    args_node = find_child_by_rule(children, "function_args")
    args = if args_node, do: parse_function_call_args(args_node), else: []

    # Check arity
    required = length(func_def.params) - map_size(func_def.defaults)
    if length(args) < required or length(args) > length(func_def.params) do
      throw({:lattice_error, WrongArityError.new(
        "Function", func_name, length(func_def.params), length(args)
      )})
    end

    # Cycle detection
    if func_name in state.function_stack do
      throw({:lattice_error, CircularReferenceError.new(
        "function",
        state.function_stack ++ [func_name]
      )})
    end

    # Create isolated scope (parent = global scope only)
    func_scope = bind_params(func_def.params, func_def.defaults, args, state.variables)

    new_state = %{state | function_stack: [func_name | state.function_stack]}

    body_clone = deep_copy(func_def.body)

    case evaluate_function_body(body_clone, func_scope, new_state) do
      {:return, return_value} ->
        # Convert the return value to a synthetic value node
        css_text = Values.to_css(return_value)
        value_node = make_value_node(css_text, node)
        final_state = %{state | function_stack: state.function_stack}
        {value_node, final_state}

      :no_return ->
        throw({:lattice_error, MissingReturnError.new(func_name)})
    end
  end

  # Evaluate the body of a @function.
  # function_body = LBRACE { function_body_item } RBRACE ;
  # function_body_item = variable_declaration | return_directive | lattice_control ;
  defp evaluate_function_body(%ASTNode{children: children}, scope, state) do
    Enum.reduce_while(children, {scope, state}, fn child, {sc, st} ->
      case child do
        %ASTNode{rule_name: "function_body_item", children: [inner | _]} ->
          case evaluate_function_body_item(inner, sc, st) do
            {:return, value} -> {:halt, {:return, value}}
            {:ok, new_scope, new_state} -> {:cont, {new_scope, new_state}}
            _ -> {:cont, {sc, st}}
          end

        _ ->
          {:cont, {sc, st}}
      end
    end)
    |> case do
      {:return, value} -> {:return, value}
      _ -> :no_return
    end
  end

  defp evaluate_function_body_item(%ASTNode{rule_name: "variable_declaration"} = node, scope, state) do
    new_scope = expand_variable_declaration(node, scope)
    {:ok, new_scope, state}
  end

  defp evaluate_function_body_item(%ASTNode{rule_name: "return_directive", children: children}, scope, _state) do
    # return_directive = "@return" lattice_expression SEMICOLON ;
    expr = find_child_by_rule(children, "lattice_expression")
    if expr do
      value = Evaluator.evaluate(expr, scope)
      {:return, value}
    else
      {:return, :null}
    end
  end

  defp evaluate_function_body_item(%ASTNode{rule_name: "lattice_control"} = node, scope, state) do
    evaluate_control_in_function(node, scope, state)
  end

  defp evaluate_function_body_item(_, scope, state), do: {:ok, scope, state}

  # Evaluate control flow inside a function (may return a value via @return)
  defp evaluate_control_in_function(%ASTNode{children: [inner | _]}, scope, state) do
    case inner do
      %ASTNode{rule_name: "if_directive"} ->
        evaluate_if_in_function(inner, scope, state)
      _ ->
        {:ok, scope, state}
    end
  end

  defp evaluate_control_in_function(_, scope, state), do: {:ok, scope, state}

  defp evaluate_if_in_function(%ASTNode{children: children}, scope, state) do
    branches = parse_if_branches(children)

    result = Enum.reduce_while(branches, nil, fn {condition, block}, _acc ->
      case condition do
        nil ->
          {:halt, block}
        expr ->
          value = Evaluator.evaluate(expr, scope)
          if Values.truthy?(value), do: {:halt, block}, else: {:cont, nil}
      end
    end)

    case result do
      nil -> {:ok, scope, state}
      block ->
        # Evaluate the block, looking for @return
        evaluate_block_in_function(block, scope, state)
    end
  end

  # Evaluate a block inside a function, propagating @return
  defp evaluate_block_in_function(%ASTNode{children: children}, scope, state) do
    Enum.reduce_while(children, {:ok, scope, state}, fn child, {_ok, sc, st} ->
      case evaluate_block_for_return(child, sc, st) do
        {:return, value} -> {:halt, {:return, value}}
        {:ok, new_sc, new_st} -> {:cont, {:ok, new_sc, new_st}}
        _ -> {:cont, {:ok, sc, st}}
      end
    end)
  end

  defp evaluate_block_for_return(%ASTNode{rule_name: "block_contents", children: children}, scope, state) do
    evaluate_block_in_function(%ASTNode{rule_name: "block_contents", children: children}, scope, state)
  end

  defp evaluate_block_for_return(%ASTNode{rule_name: "block_item", children: [inner | _]}, scope, state) do
    case inner do
      %ASTNode{rule_name: "at_rule", children: at_children} ->
        # @return might be parsed as an at_rule inside @if blocks
        maybe_evaluate_return_at_rule(at_children, scope)
      %ASTNode{rule_name: "lattice_block_item", children: [lbc | _]} ->
        case lbc do
          %ASTNode{rule_name: "variable_declaration"} ->
            new_scope = expand_variable_declaration(lbc, scope)
            {:ok, new_scope, state}
          _ ->
            {:ok, scope, state}
        end
      _ ->
        {:ok, scope, state}
    end
  end

  defp evaluate_block_for_return(_, scope, state), do: {:ok, scope, state}

  # Check if an at_rule is actually @return, and evaluate if so
  defp maybe_evaluate_return_at_rule(children, scope) do
    keyword = find_token_value(children, "AT_KEYWORD")
    prelude = find_child_by_rule(children, "at_prelude")

    if keyword == "@return" and prelude do
      # Collect tokens from the at_prelude
      tokens = collect_tokens_from_node(prelude)
      case tokens do
        [] -> {:return, :null}
        [token | _] ->
          # Simple single-token return — convert directly
          if token.type == "VARIABLE" do
            case Scope.get(scope, token.value) do
              {:ok, val} when is_tuple(val) or val == :null -> {:return, val}
              {:ok, %ASTNode{} = node} ->
                val = extract_value_from_ast_node(node, scope)
                {:return, val}
              _ -> {:return, Values.token_to_value(token)}
            end
          else
            {:return, Values.token_to_value(token)}
          end
      end
    else
      {:ok, scope, %{}}
    end
  end

  defp collect_tokens_from_node(%ASTNode{children: children}) do
    Enum.flat_map(children, fn
      %Token{} = t -> [t]
      %ASTNode{} = n -> collect_tokens_from_node(n)
      _ -> []
    end)
  end

  defp extract_value_from_ast_node(%ASTNode{children: children}, scope) do
    Enum.reduce_while(children, :null, fn
      %Token{} = token, _acc -> {:halt, Values.token_to_value(token)}
      %ASTNode{} = child, _acc ->
        v = extract_value_from_ast_node(child, scope)
        if v == :null, do: {:cont, :null}, else: {:halt, v}
      _, acc -> {:cont, acc}
    end)
  end

  # Parse function_args into individual argument values (split on commas)
  defp parse_function_call_args(%ASTNode{children: children}) do
    # Gather function_arg nodes, split on COMMA tokens
    Enum.reduce(children, [[]], fn child, [current | rest] ->
      case child do
        %Token{type: "COMMA"} ->
          [[] | [current | rest]]

        %ASTNode{rule_name: "function_arg", children: arg_children} ->
          # Check if the arg itself contains a COMMA
          comma_idx = Enum.find_index(arg_children, fn
            %Token{type: "COMMA"} -> true
            _ -> false
          end)

          if comma_idx do
            # Split at the comma
            {before_comma, [_comma | after_comma]} = Enum.split(arg_children, comma_idx)
            [[after_comma | [current ++ before_comma | rest]]]
            |> List.flatten()
          else
            # Add to current group
            arg_value = extract_arg_value(arg_children)
            case arg_value do
              nil -> [current | rest]
              val -> [[val | current] | rest]
            end
          end

        _ ->
          [current | rest]
      end
    end)
    |> Enum.map(fn group ->
      case Enum.reverse(group) do
        [single | _] -> single
        [] -> :null
      end
    end)
    |> Enum.reverse()
    |> Enum.reject(&is_nil/1)
  end

  defp extract_arg_value([%Token{type: type} | _]) when type in ["COMMA"] do
    nil
  end

  defp extract_arg_value([%Token{} = token | _]) do
    Values.token_to_value(token)
  end

  defp extract_arg_value([%ASTNode{} = node | _]) do
    node
  end

  defp extract_arg_value(_), do: nil

  # Create a value node that wraps a CSS text string
  defp make_value_node(css_text, _template_node) do
    token = %Token{type: "IDENT", value: css_text, line: 0, column: 0}
    %ASTNode{
      rule_name: "value",
      children: [token]
    }
  end

  # =====================================================================
  # Lattice v2: @while Loops
  # =====================================================================

  # while_directive = "@while" lattice_expression block ;
  # Evaluates condition, expands body, repeats. Uses enclosing scope
  # (variable mutations persist across iterations).
  defp expand_while(%ASTNode{children: children}, scope, state) do
    condition = find_child_by_rule(children, "lattice_expression")
    block = find_child_by_rule(children, "block")

    if is_nil(condition) or is_nil(block) do
      {[], state}
    else
      do_while_loop(condition, block, scope, state, 0, [])
    end
  end

  defp do_while_loop(condition, block, scope, state, iteration, acc) do
    cond_value = Evaluator.evaluate(deep_copy(condition), scope)

    if not Values.truthy?(cond_value) do
      {acc, state}
    else
      new_iteration = iteration + 1
      if new_iteration > state.max_while_iterations do
        throw({:lattice_error, MaxIterationError.new(state.max_while_iterations)})
      end

      {items, new_state} = expand_block_to_items(deep_copy(block), scope, state)
      # Update the scope from state (variable mutations inside the body)
      new_scope = new_state.variables
      do_while_loop(condition, block, new_scope, new_state, new_iteration, acc ++ items)
    end
  end

  # =====================================================================
  # Lattice v2: $var in Selectors
  # =====================================================================

  # Resolve VARIABLE tokens in selector positions to their string values.
  defp expand_selector_with_vars(%ASTNode{children: children} = node, scope, state) do
    new_children = Enum.map(children, fn child ->
      case child do
        %Token{type: "VARIABLE", value: var_name} ->
          case Scope.get(scope, var_name) do
            :error ->
              throw({:lattice_error, UndefinedVariableError.new(
                var_name,
                Map.get(child, :line, 0),
                Map.get(child, :column, 0)
              )})
            {:ok, value} ->
              css_text = cond do
                is_tuple(value) or value == :null -> Values.to_css(value)
                match?(%ASTNode{}, value) ->
                  v = extract_value_from_ast_node(value, scope)
                  Values.to_css(v)
                true -> to_string(value)
              end
              css_text = css_text |> String.trim("\"") |> String.trim("'")
              make_synthetic_token(css_text, child)
          end

        %ASTNode{} ->
          {expanded, _} = expand_node(child, scope, state)
          expanded

        _ -> child
      end
    end)

    {%{node | children: new_children}, state}
  end

  # =====================================================================
  # Lattice v2: @content Blocks
  # =====================================================================

  # Replaces @content; with the content block from the current @include call.
  # The content block is evaluated in the caller's scope.
  defp expand_content(state) do
    case state.content_block_stack do
      [] -> {nil, state}
      [nil | _] -> {nil, state}
      [content_block | _] ->
        caller_scope = case state.content_scope_stack do
          [sc | _] -> sc
          [] -> state.variables
        end
        {items, new_state} = expand_block_to_items(deep_copy(content_block), caller_scope, state)
        {items, new_state}
    end
  end

  # =====================================================================
  # Lattice v2: @at-root
  # =====================================================================

  # Rules inside @at-root are hoisted to the stylesheet root level during Pass 3.
  defp expand_at_root(%ASTNode{children: children}, scope, state) do
    block = find_child_by_rule(children, "block")
    selector_list = find_child_by_rule(children, "selector_list")

    if is_nil(block) do
      {nil, state}
    else
      if selector_list do
        # Inline form: @at-root .selector { ... }
        {expanded_sel, _} = expand_node(deep_copy(selector_list), scope, state)
        {expanded_block, _} = expand_node(deep_copy(block), scope, state)
        qr = %ASTNode{rule_name: "qualified_rule", children: [expanded_sel, expanded_block]}
        new_state = %{state | at_root_rules: state.at_root_rules ++ [qr]}
        {nil, new_state}
      else
        # Block form: @at-root { ... multiple rules ... }
        {items, _} = expand_block_to_items(deep_copy(block), scope, state)
        new_state = %{state | at_root_rules: state.at_root_rules ++ items}
        {nil, new_state}
      end
    end
  end

  # =====================================================================
  # Lattice v2: @extend and %placeholder
  # =====================================================================

  # Collect an @extend directive for later selector merging.
  defp collect_extend(%ASTNode{children: children}, _scope, state) do
    target = Enum.reduce(children, "", fn child, acc ->
      case child do
        %ASTNode{rule_name: "extend_target", children: target_children} ->
          Enum.map_join(target_children, "", fn
            %Token{value: v} -> v
            _ -> ""
          end)
        _ -> acc
      end
    end)

    if target != "" do
      new_map = Map.update(state.extend_map, target, [], fn existing -> existing end)
      %{state | extend_map: new_map}
    else
      state
    end
  end

  # =====================================================================
  # Lattice v2: Property Nesting
  # =====================================================================

  # property_nesting = property COLON block ;
  # Flattens nested property declarations by prepending parent property name.
  defp expand_property_nesting(%ASTNode{children: children}, scope, state) do
    parent_prop = Enum.reduce(children, "", fn child, acc ->
      case child do
        %ASTNode{rule_name: "property", children: [%Token{value: v} | _]} -> v
        _ -> acc
      end
    end)

    block = find_child_by_rule(children, "block")

    if parent_prop == "" or is_nil(block) do
      {[], state}
    else
      {expanded_block, _} = expand_node(deep_copy(block), scope, state)
      result = flatten_nested_props(expanded_block, parent_prop)
      {result, state}
    end
  end

  defp flatten_nested_props(%ASTNode{children: children}, prefix) do
    Enum.flat_map(children, fn child ->
      case child do
        %ASTNode{rule_name: "block_contents"} ->
          flatten_nested_props(child, prefix)
        %ASTNode{rule_name: "block_item", children: [inner | _]} ->
          flatten_nested_block_item(inner, prefix)
        %ASTNode{rule_name: "declaration"} ->
          [rewrite_declaration_prefix(child, prefix)]
        _ -> []
      end
    end)
  end

  defp flatten_nested_block_item(%ASTNode{rule_name: "declaration_or_nested", children: [inner | _]}, prefix) do
    case inner do
      %ASTNode{rule_name: "declaration"} ->
        [rewrite_declaration_prefix(inner, prefix)]
      %ASTNode{rule_name: "property_nesting"} ->
        expand_property_nesting_with_prefix(inner, prefix)
      _ -> []
    end
  end

  defp flatten_nested_block_item(_, _prefix), do: []

  defp rewrite_declaration_prefix(%ASTNode{children: children} = decl, prefix) do
    new_children = Enum.map(children, fn child ->
      case child do
        %ASTNode{rule_name: "property", children: [%Token{value: old_name} = t | rest]} ->
          %{child | children: [%{t | value: "#{prefix}-#{old_name}"} | rest]}
        _ -> child
      end
    end)
    %{decl | children: new_children}
  end

  defp expand_property_nesting_with_prefix(%ASTNode{children: children}, prefix) do
    sub_prop = Enum.reduce(children, "", fn child, acc ->
      case child do
        %ASTNode{rule_name: "property", children: [%Token{value: v} | _]} -> v
        _ -> acc
      end
    end)

    block = find_child_by_rule(children, "block")
    new_prefix = "#{prefix}-#{sub_prop}"

    if block do
      flatten_nested_props(block, new_prefix)
    else
      []
    end
  end

  # =====================================================================
  # Lattice v2: Built-in Function Evaluation
  # =====================================================================

  defp evaluate_builtin_function(func_name, %ASTNode{children: children} = node, scope, state) do
    # Collect and evaluate arguments
    args_node = find_child_by_rule(children, "function_args")
    args = if args_node do
      collect_builtin_args(args_node, scope)
    else
      []
    end

    case Builtins.call(func_name, args) do
      {:ok, :null} ->
        # Null result -- pass through as CSS function
        expand_children(node, scope, state)
      {:ok, result} ->
        css_text = Values.to_css(result)
        {make_value_node(css_text, node), state}
      {:error, _msg} ->
        # On error, pass through as CSS
        expand_children(node, scope, state)
      :not_found ->
        expand_children(node, scope, state)
    end
  end

  # Collect evaluated arguments from function_args for built-in calls
  defp collect_builtin_args(%ASTNode{children: children}, scope) do
    # Split on COMMA tokens, evaluate each group
    {args, current} = Enum.reduce(children, {[], []}, fn child, {args_acc, current_acc} ->
      case child do
        %Token{type: "COMMA"} ->
          arg = evaluate_arg_group(current_acc, scope)
          {[arg | args_acc], []}
        %ASTNode{rule_name: "function_arg", children: arg_children} ->
          # Check for COMMA inside function_arg
          {sub_args, sub_current} = Enum.reduce(arg_children, {[], current_acc}, fn ac, {sa, sc} ->
            case ac do
              %Token{type: "COMMA"} ->
                arg = evaluate_arg_group(sc, scope)
                {[arg | sa], []}
              _ ->
                {sa, sc ++ [ac]}
            end
          end)
          {sub_args ++ args_acc, sub_current}
        _ ->
          {args_acc, current_acc ++ [child]}
      end
    end)

    final_args = if current != [] do
      [evaluate_arg_group(current, scope) | args]
    else
      args
    end

    Enum.reverse(final_args)
  end

  defp evaluate_arg_group([], _scope), do: :null

  defp evaluate_arg_group([single], scope) do
    case single do
      %Token{type: "VARIABLE", value: var_name} ->
        case Scope.get(scope, var_name) do
          {:ok, val} when is_tuple(val) or val == :null -> val
          {:ok, %ASTNode{} = node} -> extract_value_from_ast_node(node, scope)
          _ -> Values.token_to_value(single)
        end
      %Token{} -> Values.token_to_value(single)
      %ASTNode{rule_name: "lattice_expression"} -> Evaluator.evaluate(single, scope)
      %ASTNode{} -> Evaluator.evaluate(single, scope)
      _ -> :null
    end
  end

  defp evaluate_arg_group([first | _], scope) do
    evaluate_arg_group([first], scope)
  end

  # =====================================================================
  # Lattice v2: @extend Pass 3 — Remove placeholder-only rules
  # =====================================================================

  defp remove_placeholder_rules(nil), do: nil
  defp remove_placeholder_rules(%Token{} = t), do: t

  defp remove_placeholder_rules(%ASTNode{children: children} = node) do
    new_children =
      children
      |> Enum.reject(&placeholder_only_rule?/1)
      |> Enum.map(&remove_placeholder_rules/1)
      |> Enum.reject(&is_nil/1)

    %{node | children: new_children}
  end

  defp remove_placeholder_rules(other), do: other

  defp placeholder_only_rule?(%ASTNode{rule_name: "qualified_rule"} = node) do
    selector_text = extract_selector_text(node)
    selectors = selector_text |> String.split(",") |> Enum.map(&String.trim/1) |> Enum.filter(& &1 != "")
    selectors != [] and Enum.all?(selectors, &String.starts_with?(&1, "%"))
  end

  defp placeholder_only_rule?(%ASTNode{rule_name: "rule", children: [inner | _]}) do
    placeholder_only_rule?(inner)
  end

  defp placeholder_only_rule?(_), do: false

  defp extract_selector_text(%ASTNode{children: children}) do
    sel_node = find_child_by_rule(children, "selector_list")
    if sel_node, do: collect_text(sel_node), else: ""
  end

  defp collect_text(%Token{value: v}), do: v

  defp collect_text(%ASTNode{children: children}) do
    Enum.map_join(children, " ", &collect_text/1)
  end

  defp collect_text(_), do: ""

  # =====================================================================
  # Lattice v2: @at-root Pass 3 — Splice hoisted rules
  # =====================================================================

  defp splice_at_root_rules(%ASTNode{rule_name: "stylesheet", children: children} = root, rules) do
    wrapped_rules = Enum.map(rules, fn rule ->
      case rule do
        %ASTNode{rule_name: "rule"} -> rule
        %ASTNode{rule_name: r} when r in ["qualified_rule", "at_rule"] ->
          %ASTNode{rule_name: "rule", children: [rule]}
        _ -> rule
      end
    end)

    %{root | children: children ++ wrapped_rules}
  end

  defp splice_at_root_rules(node, _rules), do: node

  # =====================================================================
  # Lattice v2: property_nesting inside block_item_inner
  # =====================================================================

  # Update expand_block_item_inner to handle property_nesting inside declaration_or_nested
  # (This is already handled by the block_item dispatch above, but we need
  # to also check for property_nesting in the declaration_or_nested path)

  # ---------------------------------------------------------------------------
  # Pass 3: Cleanup
  # ---------------------------------------------------------------------------

  # Remove empty blocks, nil children, and empty rules from the expanded AST.

  defp cleanup(nil), do: nil

  defp cleanup(%Token{} = token), do: token

  defp cleanup(%ASTNode{children: children} = node) do
    cleaned_children =
      children
      |> Enum.map(&cleanup/1)
      |> Enum.reject(&is_nil/1)

    %{node | children: cleaned_children}
  end

  defp cleanup(other), do: other

  # ---------------------------------------------------------------------------
  # Helpers
  # ---------------------------------------------------------------------------

  # Find a token of the given type among a list of children
  defp find_token(children, type) do
    Enum.find(children, fn
      %Token{type: ^type} -> true
      _ -> false
    end)
  end

  # Find the value of the first token of the given type
  defp find_token_value(children, type) do
    case find_token(children, type) do
      %Token{value: v} -> v
      nil -> nil
    end
  end

  # Find the first ASTNode child with the given rule_name
  defp find_child_by_rule(children, rule_name) do
    Enum.find(children, fn
      %ASTNode{rule_name: ^rule_name} -> true
      _ -> false
    end)
  end

  # Deep copy an ASTNode or Token (since Elixir is immutable, this is
  # effectively a no-op at the data level, but we keep it for semantic clarity
  # and to match the Python implementation's deep copy usage)
  defp deep_copy(%ASTNode{children: children} = node) do
    %{node | children: Enum.map(children, &deep_copy/1)}
  end

  defp deep_copy(%Token{} = token), do: token
  defp deep_copy(other), do: other
end
