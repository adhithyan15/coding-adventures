# frozen_string_literal: true

# ================================================================
# Lattice AST Transformer -- Expands Lattice Constructs to CSS
# ================================================================
#
# This is the core of the Lattice-to-CSS compiler. It takes a
# Lattice AST (containing both CSS and Lattice nodes) and produces
# a clean CSS AST (containing only CSS nodes) by expanding all
# Lattice constructs.
#
# Three-Pass Architecture
#
# Pass 1: Symbol Collection
#   Walk the top-level AST and collect:
#     - Variable declarations -> variable registry
#     - Mixin definitions     -> mixin registry
#     - Function definitions  -> function registry
#   Remove definition nodes from the AST (they produce no CSS).
#
# Pass 2: Expansion
#   Recursively walk remaining AST nodes with a scope chain:
#     - Replace VARIABLE tokens with their resolved values
#     - Expand @include directives by cloning mixin bodies
#     - Evaluate @if/@for/@each control flow
#     - Evaluate Lattice function calls and replace with return values
#   After this pass, the AST contains only pure CSS nodes.
#
# Pass 3: Cleanup
#   Remove any empty blocks or nil children from the tree.
#
# Why Not a Single Pass?
#
# Mixins and functions can be defined AFTER they're used:
#
#   .btn { @include button(red); }   <- used first
#   @mixin button($bg) { ... }       <- defined later
#
# Pass 1 collects all definitions up front, so Pass 2 can resolve
# them regardless of source order.
#
# Cycle Detection
#
# Mixin and function expansion tracks a call stack. If a name
# appears twice in the stack, CircularReferenceError is raised:
#
#   @mixin a { @include b; }
#   @mixin b { @include a; }    <- Circular mixin: a -> b -> a
# ================================================================

require "deep_clone" if defined?(DeepClone)

module CodingAdventures
  module LatticeAstToCss
    # ============================================================
    # Definition Records
    # ============================================================

    # Stored definition of a @mixin.
    MixinDef = Struct.new(:name, :params, :defaults, :body)

    # Stored definition of a @function.
    FunctionDef = Struct.new(:name, :params, :defaults, :body)

    # Internal signal for @return inside function evaluation.
    # Not a real error — used to unwind the function body when
    # a @return is hit. The value is the LatticeValue to return.
    class ReturnSignal < StandardError
      attr_reader :value

      def initialize(value)
        @value = value
        super()
      end
    end

    # ============================================================
    # CSS Built-in Functions
    # ============================================================
    #
    # These are CSS built-in functions that should NOT be resolved
    # as Lattice functions. When a function_call node uses one of
    # these names, it's passed through unchanged.
    CSS_FUNCTIONS = %w[
      rgb rgba hsl hsla hwb lab lch oklch oklab color color-mix
      calc min max clamp abs sign round mod rem
      sin cos tan asin acos atan atan2 pow sqrt hypot log exp
      var env
      url format local
      linear-gradient radial-gradient conic-gradient
      repeating-linear-gradient repeating-radial-gradient
      repeating-conic-gradient
      counter counters attr element
      translate translateX translateY translateZ
      rotate rotateX rotateY rotateZ
      scale scaleX scaleY scaleZ
      skew skewX skewY
      matrix matrix3d perspective
      cubic-bezier steps
      path polygon circle ellipse inset
      image-set cross-fade
      fit-content minmax repeat
      blur brightness contrast drop-shadow grayscale
      hue-rotate invert opacity saturate sepia
    ].freeze

    def self.css_function?(name)
      # FUNCTION token includes "(" at the end: "rgb(" -> "rgb"
      CSS_FUNCTIONS.include?(name.chomp("("))
    end

    # ============================================================
    # Transformer
    # ============================================================

    # Transforms a Lattice AST into a clean CSS AST.
    #
    # Usage:
    #   transformer = LatticeTransformer.new
    #   css_ast = transformer.transform(lattice_ast)
    #
    # The returned AST contains only CSS nodes. Pass it to
    # CSSEmitter to produce CSS text.
    class LatticeTransformer
      def initialize
        @variables = ScopeChain.new
        @mixins = {}
        @functions = {}
        @mixin_stack = []
        @function_stack = []
      end

      # Transform a Lattice AST into a clean CSS AST.
      #
      # Runs the three-pass pipeline.
      #
      # @param ast [ASTNode] the root stylesheet node
      # @return [ASTNode] the cleaned CSS AST
      def transform(ast)
        # Pass 1: Collect symbols
        collect_symbols(ast)

        # Pass 2: Expand
        result = expand_node(ast, @variables)

        # Pass 3: Cleanup
        cleanup(result)

        result
      end

      # ============================================================
      # Pass 1: Symbol Collection
      # ============================================================

      private

      def collect_symbols(ast)
        return unless ast.respond_to?(:children)

        new_children = []
        ast.children.each do |child|
          unless child.respond_to?(:rule_name)
            new_children << child
            next
          end

          if child.rule_name == "rule"
            inner = child.children[0]
            inner_rule = inner.respond_to?(:rule_name) ? inner.rule_name : nil

            if inner_rule == "lattice_rule"
              lattice_child = inner.children[0]
              lattice_rule_name = lattice_child.respond_to?(:rule_name) ? lattice_child.rule_name : nil

              case lattice_rule_name
              when "variable_declaration"
                collect_variable(lattice_child)
                next  # Don't add to output
              when "mixin_definition"
                collect_mixin(lattice_child)
                next
              when "function_definition"
                collect_function(lattice_child)
                next
              when "use_directive"
                next  # Skip @use (module resolution not implemented)
              end
            end

          end

          new_children << child
        end

        # NOTE: collect_symbols does NOT rebuild the AST here. It only reads
        # the tree to register symbols (@mixins, @functions, variables) in
        # the symbol tables. The actual stripping of Lattice constructs from
        # the output happens in Pass 2 (expand_node / expand_children).
        # This avoids the need to mutate the immutable Data.define ASTNode.
      end

      def collect_variable(node)
        # variable_declaration = VARIABLE COLON value_list SEMICOLON ;
        name = nil
        value_node = nil

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            value_node = child if child.rule_name == "value_list"
          elsif token_type_name(child) == "VARIABLE"
            name = child.value
          end
        end

        @variables.set(name, value_node) if name && value_node
      end

      def collect_mixin(node)
        # Two grammar forms:
        #   mixin_definition = "@mixin" FUNCTION [ mixin_params ] RPAREN block
        #                    | "@mixin" IDENT block ;
        #
        # In the FUNCTION form, the name token is a FUNCTION with trailing "(",
        # e.g., "button(" — we strip the "(" to get "button".
        # In the IDENT form, the name token is a plain IDENT, e.g., "centered".
        name = nil
        params = []
        defaults = {}
        body = nil

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            case child.rule_name
            when "mixin_params"
              params, defaults = extract_params(child)
            when "block"
              body = child
            end
          else
            case token_type_name(child)
            when "FUNCTION"
              name = child.value.chomp("(")
            when "IDENT"
              # Only use IDENT as name if we haven't found a FUNCTION name yet
              # and the value isn't the keyword "@mixin" itself.
              # (The "@mixin" AT_KEYWORD token comes before the name.)
              name ||= child.value unless child.value.start_with?("@")
            end
          end
        end

        @mixins[name] = MixinDef.new(name, params, defaults, body) if name && body
      end

      def collect_function(node)
        # Two grammar forms:
        #   function_definition = "@function" FUNCTION [ mixin_params ] RPAREN function_body
        #                       | "@function" IDENT function_body ;
        name = nil
        params = []
        defaults = {}
        body = nil

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            case child.rule_name
            when "mixin_params"
              params, defaults = extract_params(child)
            when "function_body"
              body = child
            end
          else
            case token_type_name(child)
            when "FUNCTION"
              name = child.value.chomp("(")
            when "IDENT"
              name ||= child.value unless child.value.start_with?("@")
            end
          end
        end

        @functions[name] = FunctionDef.new(name, params, defaults, body) if name && body
      end

      def extract_params(node)
        # mixin_params = mixin_param { COMMA mixin_param } ;
        # mixin_param = VARIABLE [ COLON value_list ] ;
        params = []
        defaults = {}

        node.children.each do |child|
          next unless child.respond_to?(:rule_name) && child.rule_name == "mixin_param"

          param_name = nil
          default_value = nil

          child.children.each do |pc|
            if pc.respond_to?(:rule_name)
              default_value = pc if pc.rule_name == "value_list"
            elsif token_type_name(pc) == "VARIABLE"
              param_name = pc.value
            end
          end

          if param_name
            params << param_name
            defaults[param_name] = default_value if default_value
          end
        end

        [params, defaults]
      end

      # ============================================================
      # Pass 2: Expansion
      # ============================================================

      def expand_node(node, scope)
        # Token — check for variable substitution.
        unless node.respond_to?(:rule_name)
          return substitute_variable(node, scope) if token_type_name(node) == "VARIABLE"

          return node
        end

        case node.rule_name
        when "block"
          expand_block(node, scope)
        when "block_contents"
          expand_block_contents(node, scope)
        when "block_item"
          expand_block_item(node, scope)
        when "value_list"
          expand_value_list(node, scope)
        when "value"
          expand_value(node, scope)
        when "function_call"
          expand_function_call(node, scope)
        when "function_arg", "function_args"
          expand_children(node, scope)
        when "stylesheet"
          expand_stylesheet(node, scope)
        when "rule"
          expand_rule(node, scope)
        else
          expand_children(node, scope)
        end
      end

      def expand_children(node, scope)
        return node unless node.respond_to?(:children)

        new_children = []
        node.children.each do |child|
          expanded = expand_node(child, scope)
          new_children << expanded if expanded
        end
        rebuild_node(node, new_children)
      end

      # Expand the top-level stylesheet node.
      #
      # The stylesheet is a flat list of rule nodes. We must:
      #   - Skip lattice_rule nodes whose inner construct is a symbol
      #     definition (variable, mixin, function, @use) — these were
      #     collected in Pass 1 and produce no CSS output.
      #   - Expand lattice_rule nodes that contain lattice_control (@if,
      #     @for, @each) — these DO produce CSS output.
      #   - Pass through all other rules (qualified rules, CSS at-rules).
      def expand_stylesheet(node, scope)
        new_children = []
        node.children.each do |child|
          result = expand_rule(child, scope)
          if result.is_a?(Array)
            new_children.concat(result)
          elsif result
            new_children << result
          end
        end
        rebuild_node(node, new_children)
      end

      # Expand a single top-level rule node.
      #
      # rule = lattice_rule | at_rule | qualified_rule ;
      #
      # For lattice_rule, we inspect the inner construct:
      #   variable_declaration, mixin_definition, function_definition, use_directive
      #     → nil (stripped; already collected in Pass 1)
      #   lattice_control (@if/@for/@each)
      #     → expanded to CSS nodes
      #
      # For at_rule and qualified_rule, recurse normally.
      def expand_rule(node, scope)
        return expand_children(node, scope) unless node.respond_to?(:rule_name)
        return expand_children(node, scope) unless node.rule_name == "rule"

        # rule has one child: lattice_rule | at_rule | qualified_rule
        return expand_children(node, scope) if node.children.empty?

        inner = node.children[0]
        return expand_children(node, scope) unless inner.respond_to?(:rule_name)

        if inner.rule_name == "lattice_rule"
          return expand_top_lattice_rule(inner, scope)
        end

        expand_children(node, scope)
      end

      # Expand a top-level lattice_rule node.
      #
      # lattice_rule = variable_declaration | mixin_definition
      #              | function_definition | use_directive | lattice_control ;
      def expand_top_lattice_rule(node, scope)
        return nil if node.children.empty?

        inner = node.children[0]
        return nil unless inner.respond_to?(:rule_name)

        case inner.rule_name
        when "variable_declaration"
          expand_variable_declaration(inner, scope)
          nil  # Remove from output — already collected in Pass 1
        when "mixin_definition", "function_definition", "use_directive"
          nil  # Remove from output — already collected in Pass 1
        when "lattice_control"
          expand_control(inner, scope)
        else
          expand_children(node, scope)
        end
      end

      def substitute_variable(token, scope)
        # Replace a VARIABLE token with its resolved value.
        name = token.value
        value = scope.get(name)

        if value.nil?
          raise LatticeUndefinedVariableError.new(
            name,
            token.respond_to?(:line) ? token.line : 0,
            token.respond_to?(:column) ? token.column : 0
          )
        end

        # If the value is an AST node (value_list), deep-copy and expand it.
        if value.respond_to?(:rule_name)
          cloned = deep_copy(value)
          return expand_node(cloned, scope)
        end

        # If it's a LatticeValue, convert to a synthetic token.
        if LATTICE_VALUE_TYPES.any? { |t| value.is_a?(t) }
          css_text = LatticeAstToCss.value_to_css(value)
          return make_token(css_text, token)
        end

        token
      end

      def expand_block(node, scope)
        # Each block creates a new child scope.
        child_scope = scope.child
        expand_children(node, child_scope)
      end

      def expand_block_contents(node, scope)
        # Block contents can include Lattice block items that expand
        # to multiple children or return nil (variable declarations).
        new_children = []

        node.children.each do |child|
          expanded = expand_block_item_inner(child, scope)
          if expanded.is_a?(Array)
            new_children.concat(expanded)
          elsif expanded
            new_children << expanded
          end
        end

        rebuild_node(node, new_children)
      end

      def expand_block_item(node, scope)
        children = node.children
        return node if children.empty?

        inner = children[0]
        return expand_children(node, scope) unless inner.respond_to?(:rule_name)

        if inner.rule_name == "lattice_block_item"
          result = expand_lattice_block_item(inner, scope)
          return nil if result.nil?
          return result if result.is_a?(Array)

          return rebuild_node(node, [result])
        end

        expand_children(node, scope)
      end

      def expand_block_item_inner(child, scope)
        return child unless child.respond_to?(:rule_name)

        if child.rule_name == "block_item"
          inner_children = child.children
          if !inner_children.empty? && inner_children[0].respond_to?(:rule_name)
            inner_rule = inner_children[0].rule_name

            if inner_rule == "lattice_block_item"
              result = expand_lattice_block_item(inner_children[0], scope)
              return nil if result.nil?
              return result if result.is_a?(Array)

              # Rebuild the lattice_block_item wrapper with the new result
              # and then rebuild the block_item.
              new_lbi = rebuild_node(inner_children[0], [result])
              return rebuild_node(child, [new_lbi])
            end
          end

          return expand_children(child, scope)
        end

        expand_children(child, scope)
      end

      def expand_lattice_block_item(node, scope)
        # lattice_block_item = variable_declaration | include_directive | lattice_control ;
        return node if node.children.empty?

        inner = node.children[0]
        return node unless inner.respond_to?(:rule_name)

        case inner.rule_name
        when "variable_declaration"
          expand_variable_declaration(inner, scope)
          nil  # Remove from output
        when "include_directive"
          expand_include(inner, scope)
        when "lattice_control"
          expand_control(inner, scope)
        else
          expand_children(node, scope)
        end
      end

      def expand_variable_declaration(node, scope)
        # Sets the variable in the current scope. Removed from output.
        name = nil
        value_node = nil

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            value_node = child if child.rule_name == "value_list"
          elsif token_type_name(child) == "VARIABLE"
            name = child.value
          end
        end

        if name && value_node
          # Expand the value first (it might contain variables).
          expanded_value = expand_node(deep_copy(value_node), scope)
          scope.set(name, expanded_value)
        end
      end

      def expand_value_list(node, scope)
        new_children = []
        node.children.each do |child|
          expanded = expand_node(child, scope)
          next unless expanded

          # If expansion returns a value_list, splice its children.
          if expanded.respond_to?(:rule_name) && expanded.rule_name == "value_list"
            new_children.concat(expanded.children)
          else
            new_children << expanded
          end
        end
        rebuild_node(node, new_children)
      end

      def expand_value(node, scope)
        children = node.children
        return node if children.empty?

        # Check if it's a VARIABLE token.
        if children.size == 1 && !children[0].respond_to?(:rule_name)
          if token_type_name(children[0]) == "VARIABLE"
            result = substitute_variable(children[0], scope)
            if result.respond_to?(:rule_name)
              return result  # Return the value_list directly
            end

            return rebuild_node(node, [result])
          end
        end

        expand_children(node, scope)
      end

      def expand_function_call(node, scope)
        # Find the FUNCTION token to get the function name.
        func_name = nil
        node.children.each do |child|
          unless child.respond_to?(:rule_name)
            if token_type_name(child) == "FUNCTION"
              func_name = child.value.chomp("(")
              break
            end
          end
        end

        # URL_TOKEN or unknown — pass through.
        return expand_children(node, scope) if func_name.nil?

        # Lattice function — evaluated FIRST, even if the name happens to
        # collide with a CSS built-in (e.g., a user-defined @function scale
        # takes precedence over the CSS scale() transform function).
        return evaluate_function_call(func_name, node, scope) if @functions.key?(func_name)

        # CSS built-in — expand args but keep structure.
        return expand_children(node, scope) if LatticeAstToCss.css_function?(func_name)

        # Unknown function — pass through.
        expand_children(node, scope)
      end

      # ============================================================
      # @include Expansion
      # ============================================================

      def expand_include(node, scope)
        # include_directive = "@include" FUNCTION include_args RPAREN ( SEMICOLON | block )
        #                   | "@include" IDENT ( SEMICOLON | block ) ;
        mixin_name = nil
        args_node = nil

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            args_node = child if child.respond_to?(:rule_name) && child.rule_name == "include_args"
          else
            type = token_type_name(child)
            if type == "FUNCTION"
              mixin_name = child.value.chomp("(")
            elsif type == "IDENT"
              mixin_name = child.value
            end
          end
        end

        return [] if mixin_name.nil?

        unless @mixins.key?(mixin_name)
          raise LatticeUndefinedMixinError.new(mixin_name)
        end

        # Cycle detection.
        if @mixin_stack.include?(mixin_name)
          raise LatticeCircularReferenceError.new("mixin", @mixin_stack + [mixin_name])
        end

        mixin_def = @mixins[mixin_name]
        args = args_node ? parse_include_args(args_node) : []

        # Arity check.
        required = mixin_def.params.size - mixin_def.defaults.size
        if args.size < required || args.size > mixin_def.params.size
          raise LatticeWrongArityError.new("Mixin", mixin_name, mixin_def.params.size, args.size)
        end

        # Create child scope with params bound.
        mixin_scope = scope.child
        mixin_def.params.each_with_index do |param_name, i|
          if i < args.size
            mixin_scope.set(param_name, args[i])
          elsif mixin_def.defaults.key?(param_name)
            mixin_scope.set(param_name, deep_copy(mixin_def.defaults[param_name]))
          end
        end

        # Clone and expand the mixin body.
        @mixin_stack.push(mixin_name)
        begin
          body_clone = deep_copy(mixin_def.body)
          expanded = expand_node(body_clone, mixin_scope)

          # Extract block_contents children.
          if expanded.respond_to?(:children)
            expanded.children.each do |child|
              if child.respond_to?(:rule_name) && child.rule_name == "block_contents"
                return child.children.to_a
              end
            end
          end
          []
        ensure
          @mixin_stack.pop
        end
      end

      def parse_include_args(node)
        # include_args = value_list { COMMA value_list } ;
        value_lists = node.children.select do |c|
          c.respond_to?(:rule_name) && c.rule_name == "value_list"
        end

        # If there's only one value_list, check if it contains commas
        # and split on them.
        return split_value_list_on_commas(value_lists[0]) if value_lists.size == 1

        value_lists
      end

      def split_value_list_on_commas(node)
        children = node.children

        # Check if any value node contains a COMMA.
        has_comma = children.any? do |child|
          if child.respond_to?(:rule_name) && child.rule_name == "value"
            child.children.any? do |vc|
              !vc.respond_to?(:rule_name) && token_type_name(vc) == "COMMA"
            end
          end
        end

        return [node] unless has_comma

        # Split on comma value nodes.
        groups = [[]]
        children.each do |child|
          if child.respond_to?(:rule_name) && child.rule_name == "value"
            inner = child.children
            if inner.size == 1 && !inner[0].respond_to?(:rule_name) && token_type_name(inner[0]) == "COMMA"
              groups << []
              next
            end
          end
          groups.last << child
        end

        # Create new value_list nodes for each group.
        groups.filter_map do |group|
          next if group.empty?

          SimpleNode.new("value_list", group)
        end
      end

      # ============================================================
      # Control Flow
      # ============================================================

      def expand_control(node, scope)
        # lattice_control = if_directive | for_directive | each_directive ;
        return nil if node.children.empty?

        inner = node.children[0]
        return nil unless inner.respond_to?(:rule_name)

        case inner.rule_name
        when "if_directive"
          expand_if(inner, scope)
        when "for_directive"
          expand_for(inner, scope)
        when "each_directive"
          expand_each(inner, scope)
        end
      end

      def expand_if(node, scope)
        # if_directive = "@if" lattice_expression block
        #                { "@else" "if" lattice_expression block }
        #                [ "@else" block ] ;
        children = node.children

        # Parse the if/else-if/else structure.
        branches = []  # [[condition_or_nil, block], ...]
        i = 0
        while i < children.size
          child = children[i]
          val = token_value(child)

          if val == "@if"
            expr = children[i + 1]
            block = children[i + 2]
            branches << [expr, block]
            i += 3
          elsif val == "@else"
            if i + 1 < children.size && token_value(children[i + 1]) == "if"
              expr = children[i + 2]
              block = children[i + 3]
              branches << [expr, block]
              i += 4
            else
              block = children[i + 1]
              branches << [nil, block]
              i += 2
            end
          else
            i += 1
          end
        end

        # Evaluate branches.
        evaluator = make_evaluator(scope)
        branches.each do |condition, block|
          if condition.nil?
            # @else — always matches.
            return expand_block_to_items(block, scope)
          else
            result = evaluator.evaluate(condition)
            return expand_block_to_items(block, scope) if LatticeAstToCss.truthy?(result)
          end
        end

        []
      end

      def expand_for(node, scope)
        # for_directive = "@for" VARIABLE "from" lattice_expression
        #                 ( "through" | "to" ) lattice_expression block ;
        children = node.children

        var_name = nil
        from_expr = nil
        to_expr = nil
        is_through = false
        block = nil

        i = 0
        while i < children.size
          child = children[i]
          val = token_value(child)

          if val && token_type_name(child) == "VARIABLE"
            var_name = val
          elsif val == "from"
            from_expr = children[i + 1]
            i += 1
          elsif val == "through"
            is_through = true
            to_expr = children[i + 1]
            i += 1
          elsif val == "to"
            is_through = false
            to_expr = children[i + 1]
            i += 1
          elsif child.respond_to?(:rule_name) && child.rule_name == "block"
            block = child
          end
          i += 1
        end

        return [] unless var_name && from_expr && to_expr && block

        evaluator = make_evaluator(scope)
        from_val = evaluator.evaluate(from_expr)
        to_val = evaluator.evaluate(to_expr)

        from_num = from_val.respond_to?(:value) ? from_val.value.to_i : 0
        to_num = to_val.respond_to?(:value) ? to_val.value.to_i : 0

        end_val = is_through ? to_num + 1 : to_num

        result = []
        (from_num...end_val).each do |i_val|
          loop_scope = scope.child
          loop_scope.set(var_name, LatticeNumber.new(i_val.to_f))
          expanded = expand_block_to_items(deep_copy(block), loop_scope)
          result.concat(expanded)
        end
        result
      end

      def expand_each(node, scope)
        # each_directive = "@each" VARIABLE { COMMA VARIABLE } "in" each_list block ;
        var_names = []
        each_list = nil
        block = nil

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            case child.rule_name
            when "each_list" then each_list = child
            when "block" then block = child
            end
          elsif token_type_name(child) == "VARIABLE"
            var_names << child.value
          end
        end

        return [] unless !var_names.empty? && each_list && block

        # Extract list items from each_list.
        # each_list = value { COMMA value } ;
        items = each_list.children.select do |c|
          c.respond_to?(:rule_name) && c.rule_name == "value"
        end

        result = []
        items.each do |item|
          loop_scope = scope.child
          item_value = extract_value_token(item)
          loop_scope.set(var_names[0], item_value)

          expanded = expand_block_to_items(deep_copy(block), loop_scope)
          result.concat(expanded)
        end
        result
      end

      def extract_value_token(node)
        if node.respond_to?(:children)
          children = node.children
          if children.size == 1
            child = children[0]
            return LatticeAstToCss.token_to_value(child) unless child.respond_to?(:rule_name)

            return child
          end
        end
        node
      end

      def expand_block_to_items(block, scope)
        expanded = expand_node(block, scope)
        if expanded.respond_to?(:children)
          expanded.children.each do |child|
            if child.respond_to?(:rule_name) && child.rule_name == "block_contents"
              return child.children.compact
            end
          end
        end
        []
      end

      # ============================================================
      # Function Evaluation
      # ============================================================

      def evaluate_function_call(func_name, node, scope)
        func_def = @functions[func_name]
        args = []

        # Parse arguments from function_args.
        node.children.each do |child|
          if child.respond_to?(:rule_name) && child.rule_name == "function_args"
            args = parse_function_call_args(child)
            break
          end
        end

        # Arity check.
        required = func_def.params.size - func_def.defaults.size
        if args.size < required || args.size > func_def.params.size
          raise LatticeWrongArityError.new("Function", func_name, func_def.params.size, args.size)
        end

        # Cycle detection.
        if @function_stack.include?(func_name)
          raise LatticeCircularReferenceError.new("function", @function_stack + [func_name])
        end

        # Create isolated scope (parent = global scope only).
        func_scope = @variables.child
        func_def.params.each_with_index do |param_name, i|
          if i < args.size
            func_scope.set(param_name, args[i])
          elsif func_def.defaults.key?(param_name)
            func_scope.set(param_name, deep_copy(func_def.defaults[param_name]))
          end
        end

        @function_stack.push(func_name)
        begin
          body_clone = deep_copy(func_def.body)
          begin
            evaluate_function_body(body_clone, func_scope)
          rescue ReturnSignal => ret
            css_text = LatticeAstToCss.value_to_css(ret.value)
            return make_value_node(css_text, node)
          end
          raise LatticeMissingReturnError.new(func_name)
        ensure
          @function_stack.pop
        end
      end

      def evaluate_function_body(body, scope)
        # function_body = LBRACE { function_body_item } RBRACE ;
        # function_body_item = variable_declaration | return_directive | lattice_control ;
        return unless body.respond_to?(:children)

        body.children.each do |child|
          next unless child.respond_to?(:rule_name)
          next unless child.rule_name == "function_body_item"

          inner = child.children.empty? ? nil : child.children[0]
          next unless inner&.respond_to?(:rule_name)

          case inner.rule_name
          when "variable_declaration"
            expand_variable_declaration(inner, scope)
          when "return_directive"
            evaluate_return(inner, scope)
          when "lattice_control"
            evaluate_control_in_function(inner, scope)
          end
        end
      end

      def evaluate_return(node, scope)
        # return_directive = "@return" lattice_expression SEMICOLON ;
        node.children.each do |child|
          if child.respond_to?(:rule_name) && child.rule_name == "lattice_expression"
            evaluator = make_evaluator(scope)
            result = evaluator.evaluate(child)
            raise ReturnSignal.new(result)
          end
        end
      end

      def evaluate_control_in_function(node, scope)
        return if node.children.empty?

        inner = node.children[0]
        return unless inner.respond_to?(:rule_name)

        evaluate_if_in_function(inner, scope) if inner.rule_name == "if_directive"
      end

      def evaluate_if_in_function(node, scope)
        children = node.children
        branches = []
        i = 0
        while i < children.size
          child = children[i]
          val = token_value(child)
          if val == "@if"
            branches << [children[i + 1], children[i + 2]]
            i += 3
          elsif val == "@else"
            if i + 1 < children.size && token_value(children[i + 1]) == "if"
              branches << [children[i + 2], children[i + 3]]
              i += 4
            else
              branches << [nil, children[i + 1]]
              i += 2
            end
          else
            i += 1
          end
        end

        evaluator = make_evaluator(scope)
        matched_block = nil
        branches.each do |condition, block|
          if condition.nil? || LatticeAstToCss.truthy?(evaluator.evaluate(condition))
            matched_block = block
            break
          end
        end
        evaluate_block_in_function(matched_block, scope) if matched_block
      end

      def evaluate_block_in_function(block, scope)
        # Evaluate a block inside a function, handling @return at-rules.
        return unless block.respond_to?(:children)

        block.children.each do |child|
          next unless child.respond_to?(:rule_name)

          case child.rule_name
          when "block_contents"
            evaluate_block_in_function(child, scope)
          when "block_item"
            next if child.children.empty?

            inner = child.children[0]
            next unless inner.respond_to?(:rule_name)

            case inner.rule_name
            when "at_rule"
              maybe_evaluate_return_at_rule(inner, scope)
            when "lattice_block_item"
              inner.children.each do |lbc|
                if lbc.respond_to?(:rule_name) && lbc.rule_name == "variable_declaration"
                  expand_variable_declaration(lbc, scope)
                end
              end
            end
          end
        end
      end

      def maybe_evaluate_return_at_rule(node, scope)
        # at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;
        keyword = nil
        prelude = nil

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            prelude = child if child.rule_name == "at_prelude"
          elsif token_type_name(child) == "AT_KEYWORD"
            keyword = child.value
          end
        end

        return if keyword != "@return" || prelude.nil?

        # Collect tokens from at_prelude.
        tokens = []
        collect_tokens(prelude, tokens)

        if tokens.empty?
          raise ReturnSignal.new(LatticeNull.new)
        end

        # For single token, convert directly.
        if tokens.size == 1
          tok = tokens[0]
          if token_type_name(tok) == "VARIABLE"
            var_val = scope.get(tok.value)
            if var_val
              if LATTICE_VALUE_TYPES.any? { |t| var_val.is_a?(t) }
                raise ReturnSignal.new(var_val)
              end
              if var_val.respond_to?(:rule_name)
                evaluator = make_evaluator(scope)
                raise ReturnSignal.new(evaluator.send(:extract_value_from_ast, var_val))
              end
            end
          end
          raise ReturnSignal.new(LatticeAstToCss.token_to_value(tok))
        end

        # Multi-token: take the first for simplicity.
        raise ReturnSignal.new(LatticeAstToCss.token_to_value(tokens[0]))
      end

      def collect_tokens(node, tokens)
        return unless node.respond_to?(:children)

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            collect_tokens(child, tokens)
          else
            tokens << child
          end
        end
      end

      def parse_function_call_args(node)
        # function_args = { function_arg } ;
        # Arguments are separated by COMMA tokens.
        args = [[]]

        node.children.each do |child|
          unless child.respond_to?(:rule_name)
            args << [] if token_type_name(child) == "COMMA"
            next
          end

          if child.rule_name == "function_arg"
            child.children.each do |ic|
              if ic.respond_to?(:rule_name)
                args.last << ic
              elsif token_type_name(ic) == "COMMA"
                args << []
              else
                args.last << LatticeAstToCss.token_to_value(ic)
              end
            end
          end
        end

        # Convert each arg group to a single value.
        args.filter_map do |group|
          group.empty? ? nil : group[0]
        end
      end

      # ============================================================
      # Pass 3: Cleanup
      # ============================================================

      def cleanup(node)
        return node unless node.respond_to?(:children)

        new_children = []
        node.children.each do |child|
          next if child.nil?

          cleaned = cleanup(child)
          new_children << cleaned if cleaned
        end
        rebuild_node(node, new_children)
      end

      # ============================================================
      # Utility Helpers
      # ============================================================

      def token_type_name(token)
        return "" if token.respond_to?(:rule_name)

        t = token.type
        t.respond_to?(:name) ? t.name : t.to_s
      end

      def token_value(token)
        return nil if token.respond_to?(:rule_name)

        token.respond_to?(:value) ? token.value : nil
      end

      # Deep copy an AST node by marshaling. This is a simple portable
      # approach that works for any serializable tree.
      def deep_copy(node)
        Marshal.load(Marshal.dump(node))
      rescue TypeError
        # If Marshal fails (e.g., singleton objects), fall back to dup.
        node.dup
      end

      # Create a synthetic token node with the given CSS text value.
      #
      # We determine the token type from the value string to ensure
      # the emitter formats it correctly.
      def make_token(css_text, template)
        type_name = if css_text.start_with?("#")
          "HASH"
        elsif css_text.start_with?('"', "'")
          "STRING"
        elsif css_text.end_with?("%")
          "PERCENTAGE"
        elsif css_text.match?(/\A-?[0-9]/) && css_text.match?(/[a-zA-Z]/)
          "DIMENSION"
        elsif css_text.match?(/\A-?[0-9.]/)
          "NUMBER"
        else
          "IDENT"
        end

        line = template.respond_to?(:line) ? template.line : 0
        column = template.respond_to?(:column) ? template.column : 0
        SyntheticToken.new(type_name, css_text, line, column)
      end

      # Create a value node wrapping a synthetic token.
      def make_value_node(css_text, template)
        token = make_token(css_text, template)
        SimpleNode.new("value", [token])
      end

      # Create an ExpressionEvaluator with the transformer's function
      # resolver injected.
      #
      # The resolver allows @return expressions containing Lattice function
      # calls (e.g., @return b($x)) to be evaluated at compile time.
      # Without the resolver the evaluator would treat user-defined function
      # names as CSS pass-throughs.
      def make_evaluator(scope)
        resolver = method(:resolve_function_in_expr)
        ExpressionEvaluator.new(scope, function_resolver: resolver)
      end

      # Resolve a Lattice function call found inside an expression.
      #
      # Called by ExpressionEvaluator when it encounters a function_call
      # node in an expression. Delegates to evaluate_function_call if the
      # name is a known Lattice function, otherwise returns LatticeNull
      # (so CSS built-ins pass through unchanged to the emitter).
      def resolve_function_in_expr(func_name, node, scope)
        if @functions.key?(func_name)
          result_node = evaluate_function_call(func_name, node, scope)
          # evaluate_function_call returns a SimpleNode (value node).
          # We need to extract the LatticeValue from it for the evaluator.
          if result_node.respond_to?(:children) && !result_node.children.empty?
            child = result_node.children[0]
            if child.respond_to?(:rule_name)
              # value_list or value node — extract the inner token
              return LatticeAstToCss.token_to_value(collect_tokens_array(child).first)
            elsif child.respond_to?(:value)
              return LatticeAstToCss.token_to_value(child)
            end
          end
        end
        LatticeNull.new
      end

      # Collect all leaf tokens from a node into a flat array.
      def collect_tokens_array(node)
        result = []
        return result unless node.respond_to?(:children)

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            result.concat(collect_tokens_array(child))
          else
            result << child
          end
        end
        result
      end

      # Rebuild a node with new children.
      #
      # ASTNode is a Data.define (immutable). Data#with returns a new
      # instance with only the specified attributes changed — exactly what
      # we need when the transformer wants to produce a modified tree without
      # mutating the original. SimpleNode is a plain mutable class, so we
      # mutate it in-place and return the same object.
      #
      # Callers MUST use the return value, not the original node:
      #   node = rebuild_node(node, new_children)   # correct
      #   rebuild_node(node, new_children)           # wrong — drops result
      def rebuild_node(node, new_children)
        if node.respond_to?(:with)
          # Data.define (ASTNode): returns a new frozen instance.
          node.with(children: new_children)
        else
          # SimpleNode or other mutable node: mutate and return self.
          node.children = new_children
          node
        end
      end
    end

    # ============================================================
    # Helper Node Classes
    # ============================================================

    # A minimal AST node used for synthetic constructs.
    # Used when we need to create new nodes (e.g., value_list from
    # comma-split arguments) without a full parser.
    class SimpleNode
      attr_accessor :rule_name, :children

      def initialize(rule_name, children = [])
        @rule_name = rule_name
        @children = children
      end

      def inspect
        "SimpleNode(#{rule_name}, #{children.size} children)"
      end
    end

    # A minimal token used for synthetic substitution results.
    # When we evaluate a LatticeValue (e.g., 16px from $n * 8px),
    # we create a SyntheticToken so the emitter can format it.
    class SyntheticToken
      attr_reader :type, :value, :line, :column

      def initialize(type, value, line = 0, column = 0)
        @type = type
        @value = value
        @line = line
        @column = column
      end

      def inspect
        "SyntheticToken(#{type}, #{value.inspect})"
      end
    end
  end
end
