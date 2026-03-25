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
    # Maximum number of iterations allowed in a @while loop.
    MAX_WHILE_ITERATIONS = 1000

    class LatticeTransformer
      def initialize(max_while_iterations: MAX_WHILE_ITERATIONS)
        @variables = ScopeChain.new
        @mixins = {}
        @functions = {}
        @mixin_stack = []
        @function_stack = []
        @max_while_iterations = max_while_iterations
        # Lattice v2: @extend tracking
        @extend_map = {}
        # Lattice v2: @at-root hoisted rules
        @at_root_rules = []
        # Lattice v2: @content block tracking
        @content_block_stack = []
        @content_scope_stack = []
      end

      # Transform a Lattice AST into a clean CSS AST.
      #
      # Runs the three-pass pipeline:
      #   1. Collect symbols (variables, mixins, functions, @extend)
      #   2. Expand (resolve variables, expand mixins, evaluate control flow,
      #      handle @while/@content/@at-root/property nesting)
      #   3. Cleanup + @extend selector merging + @at-root hoisting
      #
      # @param ast [ASTNode] the root stylesheet node
      # @return [ASTNode] the cleaned CSS AST
      def transform(ast)
        # Pass 1: Collect symbols
        collect_symbols(ast)

        # Pass 2: Expand
        result = expand_node(ast, @variables)

        # Pass 3: Cleanup + @extend + @at-root
        cleanup(result)
        remove_placeholder_rules(result) unless @extend_map.empty?
        splice_at_root_rules(result) unless @at_root_rules.empty?

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
        # variable_declaration = VARIABLE COLON value_list { variable_flag } SEMICOLON ;
        # Lattice v2 adds !default and !global flags.
        name = nil
        value_node = nil
        is_default = false
        is_global = false

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            case child.rule_name
            when "value_list"
              value_node = child
            when "variable_flag"
              child.children.each do |fc|
                ft = fc.respond_to?(:rule_name) ? "" : token_type_name(fc)
                is_default = true if ft == "BANG_DEFAULT"
                is_global = true if ft == "BANG_GLOBAL"
              end
            end
          else
            type = token_type_name(child)
            if type == "VARIABLE"
              name = child.value
            elsif type == "BANG_DEFAULT"
              is_default = true
            elsif type == "BANG_GLOBAL"
              is_global = true
            end
          end
        end

        return unless name && value_node

        if is_default && is_global
          root = @variables
          root = root.parent while root.parent
          @variables.set_global(name, value_node) if root.get(name).nil?
        elsif is_default
          @variables.set(name, value_node) if @variables.get(name).nil?
        elsif is_global
          @variables.set_global(name, value_node)
        else
          @variables.set(name, value_node)
        end
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
              default_value = pc if pc.rule_name == "value_list" || pc.rule_name == "mixin_value_list"
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
        # Lattice v2: resolve variables in selector positions
        when "compound_selector", "simple_selector", "class_selector"
          expand_selector_with_vars(node, scope)
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

            # Lattice v2: handle property_nesting inside declaration_or_nested
            if inner_rule == "declaration_or_nested"
              don_children = inner_children[0].children
              if !don_children.empty? && don_children[0].respond_to?(:rule_name) &&
                 don_children[0].rule_name == "property_nesting"
                result = expand_property_nesting(don_children[0], scope)
                return result.empty? ? nil : result
              end
            end
          end

          return expand_children(child, scope)
        end

        expand_children(child, scope)
      end

      def expand_lattice_block_item(node, scope)
        # lattice_block_item = variable_declaration | include_directive
        #                    | lattice_control | content_directive
        #                    | at_root_directive | extend_directive ;
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
        when "content_directive"
          expand_content(inner, scope)
        when "at_root_directive"
          expand_at_root(inner, scope)
          nil  # Hoisted to root in Pass 3
        when "extend_directive"
          collect_extend(inner, scope)
          nil  # Handled in Pass 3
        else
          expand_children(node, scope)
        end
      end

      def expand_variable_declaration(node, scope)
        # Sets the variable in the current scope. Removed from output.
        # Lattice v2: handles !default and !global flags.
        name = nil
        value_node = nil
        is_default = false
        is_global = false

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            case child.rule_name
            when "value_list"
              value_node = child
            when "variable_flag"
              child.children.each do |fc|
                ft = fc.respond_to?(:rule_name) ? "" : token_type_name(fc)
                is_default = true if ft == "BANG_DEFAULT"
                is_global = true if ft == "BANG_GLOBAL"
              end
            end
          else
            type = token_type_name(child)
            if type == "VARIABLE"
              name = child.value
            elsif type == "BANG_DEFAULT"
              is_default = true
            elsif type == "BANG_GLOBAL"
              is_global = true
            end
          end
        end

        return unless name && value_node

        expanded_value = expand_node(deep_copy(value_node), scope)

        # Try to evaluate as an expression (e.g. $i + 1 → LatticeNumber(2)).
        # This is critical for @while loops: without it, $i: $i + 1
        # stores unevaluated tokens instead of the computed number, causing
        # the loop condition to never change and looping forever.
        begin
          evaluator = make_evaluator(scope)
          evaluated = evaluator.evaluate(deep_copy(expanded_value))
          # Store the LatticeValue directly so substitute_variable can
          # convert it via the LATTICE_VALUE_TYPES check.
          expanded_value = evaluated if LATTICE_VALUE_TYPES.any? { |t| evaluated.is_a?(t) }
        rescue
          # Not a pure expression (e.g. Helvetica, sans-serif) — keep AST
        end

        if is_default && is_global
          root = scope
          root = root.parent while root.parent
          scope.set_global(name, expanded_value) if root.get(name).nil?
        elsif is_default
          scope.set(name, expanded_value) if scope.get(name).nil?
        elsif is_global
          scope.set_global(name, expanded_value)
        else
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

        # User-defined function ALWAYS takes priority — even over CSS built-ins
        # like scale(), translate(), etc. If the user defines @function scale(),
        # their definition wins. This matches Sass behavior.
        return evaluate_function_call(func_name, node, scope) if @functions.key?(func_name)

        # CSS built-in that does NOT overlap with Lattice built-ins — pass through.
        if LatticeAstToCss.css_function?(func_name) && !BUILTIN_FUNCTIONS.key?(func_name)
          return expand_children(node, scope)
        end

        # Lattice v2 built-in function.
        return evaluate_builtin_function(func_name, node, scope) if BUILTIN_FUNCTIONS.key?(func_name)

        # CSS built-in that overlaps with Lattice built-in names.
        return expand_children(node, scope) if LatticeAstToCss.css_function?(func_name)

        # Unknown function — pass through.
        expand_children(node, scope)
      end

      # ============================================================
      # @include Expansion
      # ============================================================

      def expand_include(node, scope)
        # include_directive = "@include" FUNCTION [ include_args ] RPAREN
        #                       ( SEMICOLON | block )
        #                   | "@include" IDENT ( SEMICOLON | block ) ;
        # Lattice v2: if @include has a trailing block, that block is the
        # content block -- it replaces @content; in the mixin body.
        mixin_name = nil
        args_node = nil
        content_block = nil

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            case child.rule_name
            when "include_args"
              args_node = child
            when "block"
              content_block = child
            end
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
        raw_args = args_node ? parse_include_args(args_node) : []

        # Normalise raw_args into positional (Array) + named (Hash).
        # parse_include_args returns either:
        #   - A Hash { positional: [...], named: {...} }  (new grammar)
        #   - An Array of value_list nodes                (legacy grammar)
        positional_args, named_args = if raw_args.is_a?(Hash)
          [raw_args[:positional] || [], raw_args[:named] || {}]
        else
          [raw_args, {}]
        end

        # Arity check against combined positional + named count.
        total_args = positional_args.size + named_args.size
        required = mixin_def.params.size - mixin_def.defaults.size
        if total_args < required || total_args > mixin_def.params.size
          raise LatticeWrongArityError.new("Mixin", mixin_name, mixin_def.params.size, total_args)
        end

        # Pre-evaluate each arg in the CALLER's scope before binding to the
        # mixin scope. This prevents infinite recursion when the mixin
        # parameter name matches the caller's variable name.
        #
        # Example of the problem this fixes:
        #   $color: red;
        #   @mixin tint($color) { color: $color; }
        #   .x { @include tint($color); }
        #
        # Without pre-evaluation: the mixin scope sets $color = $color AST node.
        # When the mixin body references $color, it re-resolves using the mixin
        # scope, which finds $color again → infinite recursion.
        # With pre-evaluation: $color is expanded to "red" (a token) in the
        # caller's scope first, so the mixin scope receives the concrete value.
        evaluate_arg = lambda do |arg_node|
          cloned = deep_copy(arg_node)
          expanded = expand_node(cloned, scope)  # caller's scope
          expanded || arg_node
        end

        # Create child scope with params bound (named takes priority over positional).
        mixin_scope = scope.child
        pos_idx = 0
        mixin_def.params.each do |param_name|
          if named_args.key?(param_name)
            mixin_scope.set(param_name, evaluate_arg.call(named_args[param_name]))
          elsif pos_idx < positional_args.size
            mixin_scope.set(param_name, evaluate_arg.call(positional_args[pos_idx]))
            pos_idx += 1
          elsif mixin_def.defaults.key?(param_name)
            mixin_scope.set(param_name, deep_copy(mixin_def.defaults[param_name]))
          end
        end

        # Lattice v2: push content block and caller scope for @content
        @content_block_stack.push(content_block)
        @content_scope_stack.push(scope)

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
          @content_block_stack.pop
          @content_scope_stack.pop
        end
      end

      def parse_include_args(node)
        # Two grammar forms:
        #
        # New grammar (v2+):
        #   include_args = include_arg { COMMA include_arg } ;
        #   include_arg  = VARIABLE COLON value_list | value_list ;
        #
        # Legacy grammar:
        #   include_args = value_list { COMMA value_list } ;
        #   (sometimes a single value_list with embedded commas)
        #
        # Returns { positional: [value_list, ...], named: { "$param" => value_list } }
        # for the new grammar, or a plain Array for the legacy grammar (for
        # backwards-compatible callers that expect an Array).
        #
        # expand_include handles both return shapes.

        include_arg_nodes = node.children.select do |c|
          c.respond_to?(:rule_name) && c.rule_name == "include_arg"
        end

        # New grammar: include_arg nodes present.
        if include_arg_nodes.any?
          positional = []
          named = {}

          include_arg_nodes.each do |arg_node|
            children = arg_node.children
            var_child = children.find { |c| !c.respond_to?(:rule_name) && token_type_name(c) == "VARIABLE" }
            colon_child = children.find { |c| !c.respond_to?(:rule_name) && token_type_name(c) == "COLON" }
            val_list = children.find { |c| c.respond_to?(:rule_name) && c.rule_name == "value_list" }

            if var_child && colon_child && val_list
              # Named arg: $param: value_list
              named[var_child.value] = val_list
            elsif val_list
              # Positional arg
              positional << val_list
            end
          end

          return { positional: positional, named: named }
        end

        # Legacy grammar: look for bare value_list children.
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
        # lattice_control = if_directive | for_directive | each_directive | while_directive ;
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
        when "while_directive"
          expand_while(inner, scope)
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
        # Lattice v2: @each $key, $value in $map destructures map entries.
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

        # Check if each_list references a variable holding a map or list
        resolved = resolve_each_list(each_list, scope)
        return expand_each_over_resolved(var_names, resolved, block, scope) if resolved

        # Extract list items from each_list.
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

      # Try to resolve an each_list to a LatticeMap or LatticeList.
      #
      # Three possible results:
      #   1. Variable bound to a LatticeMap or LatticeList → return it directly.
      #   2. Variable bound to an AST node that wraps a map_literal → convert
      #      the map_literal to a LatticeMap and return it. This handles:
      #        $colors: (primary: red, secondary: blue);
      #        @each $key, $val in $colors { ... }
      #      where $colors was stored as an un-evaluated AST node.
      #   3. Otherwise → return nil (caller falls back to token-by-token iteration).
      def resolve_each_list(each_list, scope)
        var_tokens = []
        each_list.children.each do |child|
          if child.respond_to?(:rule_name) && child.rule_name == "value"
            child.children.each do |vc|
              if !vc.respond_to?(:rule_name) && token_type_name(vc) == "VARIABLE"
                var_tokens << vc
              end
            end
          end
        end

        if var_tokens.size == 1
          val = scope.get(var_tokens[0].value)
          return val if val.is_a?(LatticeMap) || val.is_a?(LatticeList)

          # Bug #4: variable might be bound to an AST node that wraps a
          # map_literal (e.g., $colors: (primary: red, secondary: blue);).
          # The value stored in scope is a value_list or similar AST node.
          if val.respond_to?(:rule_name)
            map_lit = find_map_literal_in_ast(val)
            return convert_map_literal_to_lattice_map(map_lit, scope) if map_lit
          end
        end

        nil
      end

      # Recursively search an AST node for the first "map_literal" child.
      def find_map_literal_in_ast(node)
        return nil unless node.respond_to?(:rule_name)
        return node if node.rule_name == "map_literal"

        return nil unless node.respond_to?(:children)

        node.children.each do |child|
          result = find_map_literal_in_ast(child)
          return result if result
        end
        nil
      end

      # Convert a map_literal AST node to a LatticeMap.
      #
      # map_literal = LPAREN map_entry COMMA map_entry { COMMA map_entry } RPAREN ;
      # map_entry   = ( IDENT | STRING ) COLON lattice_expression ;
      #
      # Keys are extracted as plain strings (quotes stripped for STRING tokens).
      # Values are evaluated using ExpressionEvaluator in the given scope.
      def convert_map_literal_to_lattice_map(map_lit_node, scope)
        items = []
        evaluator = make_evaluator(scope)

        map_lit_node.children.each do |child|
          next unless child.respond_to?(:rule_name) && child.rule_name == "map_entry"

          key = nil
          value = nil

          child.children.each do |entry_child|
            if entry_child.respond_to?(:rule_name) && entry_child.rule_name == "lattice_expression"
              value = evaluator.evaluate(entry_child)
            elsif !entry_child.respond_to?(:rule_name)
              type = token_type_name(entry_child)
              if type == "IDENT"
                key = entry_child.value
              elsif type == "STRING"
                # Strip surrounding quotes from string keys.
                key = entry_child.value.gsub(/\A['"]|['"]\z/, "")
              end
            end
          end

          items << [key, value] if key && value
        end

        LatticeMap.new(items)
      end

      # Expand @each over a resolved LatticeMap or LatticeList.
      def expand_each_over_resolved(var_names, collection, block, scope)
        result = []
        if collection.is_a?(LatticeMap)
          collection.items.each do |key, value|
            loop_scope = scope.child
            loop_scope.set(var_names[0], LatticeIdent.new(key))
            loop_scope.set(var_names[1], value) if var_names.size >= 2
            expanded = expand_block_to_items(deep_copy(block), loop_scope)
            result.concat(expanded)
          end
        elsif collection.is_a?(LatticeList)
          collection.items.each do |item|
            loop_scope = scope.child
            loop_scope.set(var_names[0], item)
            expanded = expand_block_to_items(deep_copy(block), loop_scope)
            result.concat(expanded)
          end
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
      # Lattice v2: @while Loops
      # ============================================================

      def expand_while(node, scope)
        # while_directive = "@while" lattice_expression block ;
        #
        # Unlike @for (which creates a child scope per iteration), @while
        # uses the enclosing scope directly. Variable mutations inside the
        # body (e.g., $i: $i + 1) must persist across iterations so the
        # loop condition can change.
        #
        # We extract block_contents children once, then deep-copy and
        # expand each item per iteration using expand_block_item_inner
        # which processes lattice_block_item (variable_declaration,
        # include_directive, lattice_control) and CSS items directly
        # in the given scope — no child scope is created.
        condition = nil
        block_items = nil

        node.children.each do |child|
          next unless child.respond_to?(:rule_name)
          case child.rule_name
          when "lattice_expression"
            condition = child
          when "block"
            # Find block_contents inside the block node
            child.children.each do |bc|
              if bc.respond_to?(:rule_name) && bc.rule_name == "block_contents"
                block_items = bc.children
              end
            end
          end
        end

        return [] unless condition && block_items

        result = []
        iteration = 0

        loop do
          evaluator = make_evaluator(scope)
          cond_value = evaluator.evaluate(deep_copy(condition))
          break unless LatticeAstToCss.truthy?(cond_value)

          iteration += 1
          raise LatticeMaxIterationError.new(@max_while_iterations) if iteration > @max_while_iterations

          # Expand each block item directly in the enclosing scope.
          # Variable declarations update scope; CSS rules are collected.
          deep_copy(block_items).each do |item|
            expanded = expand_block_item_inner(item, scope)
            if expanded.is_a?(Array)
              result.concat(expanded)
            elsif expanded
              result << expanded
            end
          end
        end

        result
      end

      # ============================================================
      # Lattice v2: $var in Selectors
      # ============================================================

      def expand_selector_with_vars(node, scope)
        new_children = []
        node.children.each do |child|
          unless child.respond_to?(:rule_name)
            if token_type_name(child) == "VARIABLE"
              var_name = child.value
              value = scope.get(var_name)
              if value.nil?
                raise LatticeUndefinedVariableError.new(
                  var_name,
                  child.respond_to?(:line) ? child.line : 0,
                  child.respond_to?(:column) ? child.column : 0
                )
              end
              css_text = if LATTICE_VALUE_TYPES.any? { |t| value.is_a?(t) }
                LatticeAstToCss.value_to_css(value)
              elsif value.respond_to?(:rule_name)
                ev = make_evaluator(scope)
                v = ev.send(:extract_value_from_ast, value)
                LatticeAstToCss.value_to_css(v)
              else
                value.to_s
              end
              # Strip quotes from strings in selector context
              css_text = css_text.delete('"').delete("'")
              new_children << make_token(css_text, child)
            else
              new_children << child
            end
          else
            new_children << expand_node(child, scope)
          end
        end
        rebuild_node(node, new_children)
      end

      # ============================================================
      # Lattice v2: @content Blocks
      # ============================================================

      def expand_content(_node, scope)
        # content_directive = "@content" SEMICOLON ;
        return [] if @content_block_stack.empty?

        content_block = @content_block_stack.last
        return [] unless content_block

        caller_scope = @content_scope_stack.last || scope
        expand_block_to_items(deep_copy(content_block), caller_scope)
      end

      # ============================================================
      # Lattice v2: @at-root
      # ============================================================

      def expand_at_root(node, scope)
        # at_root_directive = "@at-root" ( block | selector_list block ) ;
        block = nil
        selector_list = nil

        node.children.each do |child|
          next unless child.respond_to?(:rule_name)

          case child.rule_name
          when "block" then block = child
          when "selector_list" then selector_list = child
          end
        end

        return unless block

        if selector_list
          expanded_sel = expand_node(deep_copy(selector_list), scope)
          expanded_block = expand_node(deep_copy(block), scope)
          qr = SimpleNode.new("qualified_rule", [expanded_sel, expanded_block])
          @at_root_rules << qr
        else
          expanded = expand_block_to_items(deep_copy(block), scope)
          @at_root_rules.concat(expanded)
        end
      end

      # ============================================================
      # Lattice v2: @extend and %placeholder
      # ============================================================

      def collect_extend(node, _scope)
        # extend_directive = "@extend" extend_target SEMICOLON ;
        target = ""
        node.children.each do |child|
          if child.respond_to?(:rule_name) && child.rule_name == "extend_target"
            parts = []
            child.children.each do |tc|
              parts << tc.value unless tc.respond_to?(:rule_name)
            end
            target = parts.join
          end
        end
        @extend_map[target] = [] if !target.empty? && !@extend_map.key?(target)
      end

      # ============================================================
      # Lattice v2: Property Nesting
      # ============================================================

      def expand_property_nesting(node, scope)
        # property_nesting = property COLON block ;
        parent_prop = ""
        block = nil

        node.children.each do |child|
          next unless child.respond_to?(:rule_name)

          case child.rule_name
          when "property"
            child.children.each { |pc| parent_prop = pc.value }
          when "block"
            block = child
          end
        end

        return [] if parent_prop.empty? || block.nil?

        expanded = expand_node(deep_copy(block), scope)
        result = []
        flatten_nested_props(expanded, parent_prop, result)
        result
      end

      def flatten_nested_props(node, prefix, result)
        return unless node.respond_to?(:children)

        node.children.each do |child|
          next unless child.respond_to?(:rule_name)

          case child.rule_name
          when "block_contents"
            flatten_nested_props(child, prefix, result)
          when "block_item"
            flatten_nested_block_item(child, prefix, result)
          when "declaration"
            rewrite_declaration_prefix(child, prefix, result)
          end
        end
      end

      def flatten_nested_block_item(node, prefix, result)
        return if node.children.empty?

        inner = node.children[0]
        return unless inner.respond_to?(:rule_name)

        if inner.rule_name == "declaration_or_nested"
          inner.children.each do |dc|
            next unless dc.respond_to?(:rule_name)

            case dc.rule_name
            when "declaration"
              rewrite_declaration_prefix(dc, prefix, result)
            when "property_nesting"
              sub = expand_property_nesting_with_prefix(dc, prefix)
              result.concat(sub)
            end
          end
        end
      end

      def rewrite_declaration_prefix(decl, prefix, result)
        # Rewrite the declaration's property name to include the prefix.
        # We need to rebuild the property node with the modified token.
        decl.children.each do |child|
          if child.respond_to?(:rule_name) && child.rule_name == "property"
            child.children.each do |pc|
              unless pc.respond_to?(:rule_name)
                old_name = pc.value
                # Create a new token with prefixed name
                new_tok = SyntheticToken.new(pc.respond_to?(:type) ? (pc.type.respond_to?(:name) ? pc.type.name : pc.type.to_s) : "IDENT",
                  "#{prefix}-#{old_name}",
                  pc.respond_to?(:line) ? pc.line : 0,
                  pc.respond_to?(:column) ? pc.column : 0)
                # Replace in-place - property nodes are mutable enough for this
                idx = child.children.index(pc)
                if child.respond_to?(:with)
                  new_children = child.children.dup
                  new_children[idx] = new_tok
                  # Rebuild immutable node
                else
                  child.children[idx] = new_tok if idx
                end
              end
            end
          end
        end
        result << decl
      end

      def expand_property_nesting_with_prefix(node, prefix)
        sub_prop = ""
        block = nil
        node.children.each do |child|
          next unless child.respond_to?(:rule_name)

          case child.rule_name
          when "property"
            child.children.each { |pc| sub_prop = pc.value }
          when "block"
            block = child
          end
        end
        new_prefix = "#{prefix}-#{sub_prop}"
        result = []
        flatten_nested_props(block, new_prefix, result) if block
        result
      end

      # ============================================================
      # Lattice v2: Built-in Function Evaluation
      # ============================================================

      def evaluate_builtin_function(func_name, node, scope)
        # Evaluate args using ExpressionEvaluator, then call the built-in handler.
        args = []
        node.children.each do |child|
          if child.respond_to?(:rule_name) && child.rule_name == "function_args"
            evaluator = make_evaluator(scope)
            args = evaluator.collect_function_args(child)
            break
          end
        end

        handler = BUILTIN_FUNCTIONS[func_name]
        result = handler.call(args, scope)

        return expand_children(node, scope) if result.is_a?(LatticeNull)

        css_text = LatticeAstToCss.value_to_css(result)
        make_value_node(css_text, node)
      end

      # ============================================================
      # Lattice v2: @extend Selector Merging (Pass 3)
      # ============================================================

      def remove_placeholder_rules(node)
        return unless node.respond_to?(:children)

        new_children = []
        node.children.each do |child|
          next if child.nil?
          next if placeholder_only_rule?(child)

          remove_placeholder_rules(child)
          new_children << child
        end
        rebuild_node(node, new_children)
      end

      def placeholder_only_rule?(node)
        return false unless node.respond_to?(:rule_name)

        if node.rule_name == "qualified_rule"
          selector_text = extract_selector_text(node)
          selectors = selector_text.split(",").map(&:strip)
          return selectors.all? { |s| s.start_with?("%") } if selectors.any?
        end
        if node.rule_name == "rule" && !node.children.empty? && node.children[0].respond_to?(:rule_name)
          return placeholder_only_rule?(node.children[0])
        end
        false
      end

      def extract_selector_text(node)
        return "" unless node.respond_to?(:children)

        node.children.each do |child|
          if child.respond_to?(:rule_name) && child.rule_name == "selector_list"
            return collect_text(child)
          end
        end
        ""
      end

      def collect_text(node)
        return node.value if !node.respond_to?(:rule_name) && node.respond_to?(:value)

        parts = []
        node.children.each { |c| parts << collect_text(c) } if node.respond_to?(:children)
        parts.join(" ")
      end

      # ============================================================
      # Lattice v2: @at-root Hoisting (Pass 3)
      # ============================================================

      def splice_at_root_rules(root)
        return unless root.respond_to?(:children)

        @at_root_rules.each do |rule|
          root.children << rule if rule
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
