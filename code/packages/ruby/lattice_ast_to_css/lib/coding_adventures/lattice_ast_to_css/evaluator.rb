# frozen_string_literal: true

# ================================================================
# Expression Evaluator -- Compile-time Lattice Expression Evaluation
# ================================================================
#
# Lattice expressions appear in three contexts:
#
# 1. @if conditions:  @if $theme == dark { ... }
# 2. @for bounds:    @for $i from 1 through $count { ... }
# 3. @return values: @return $n * 8px;
#
# The evaluator walks `lattice_expression` AST nodes and computes
# their values at compile time. This is similar to constant folding
# in a compiler, but Lattice evaluates ALL expressions at compile
# time (there's no runtime).
#
# Value Types
#
#   LatticeNumber     -- pure numbers: 42, 3.14
#   LatticeDimension  -- with units: 16px, 2em, 50vh
#   LatticePercentage -- percentages: 50%, 100%
#   LatticeString     -- quoted strings: "hello", 'world'
#   LatticeIdent      -- unquoted idents: red, bold, dark
#   LatticeColor      -- hash colors: #4a90d9, #fff
#   LatticeBool       -- booleans: true, false
#   LatticeNull       -- null (falsy, like Sass's null)
#   LatticeList       -- comma-separated lists (for @each)
#
# Operator Precedence (matching grammar nesting):
#
#   1. Unary minus:        -$x
#   2. Multiplication:     $a * $b
#   3. Addition/sub:       $a + $b, $a - $b
#   4. Comparison:         ==, !=, >, >=, <=
#   5. Logical AND:        $a and $b
#   6. Logical OR:         $a or $b
#
# The grammar's rule nesting already encodes precedence, so the
# evaluator just recursively evaluates each node without needing
# its own precedence table.
#
# Arithmetic Rules
#
#   Number + Number -> Number
#   Dimension + Dimension (same unit) -> Dimension
#   Percentage + Percentage -> Percentage
#
#   Number * Number -> Number
#   Number * Dimension -> Dimension
#   Dimension * Number -> Dimension
#   Number * Percentage -> Percentage
#   Percentage * Number -> Percentage
#
# Everything else raises LatticeTypeErrorInExpression.
# ================================================================

module CodingAdventures
  module LatticeAstToCss
    # ============================================================
    # Value Type Classes
    # ============================================================
    #
    # Why not use native Ruby types? Because we need to track units
    # (10px vs 10em), distinguish idents from strings, and handle
    # CSS-specific semantics. Struct gives us immutability and
    # structural equality for free.

    # A pure number without units. Maps to CSS NUMBER token.
    # Examples: 42, 3.14, 0, -1
    LatticeNumber = Struct.new(:value) do
      # Emit integers without decimal point: 42 not 42.0.
      def to_s
        (value == value.to_i) ? value.to_i.to_s : value.to_s
      end

      def truthy?
        value != 0
      end
    end

    # A number with a CSS unit. Maps to CSS DIMENSION token.
    # Examples: 16px, 2em, 1.5rem, 100vh, 300ms
    # Arithmetic is only valid between dimensions with the same unit.
    LatticeDimension = Struct.new(:value, :unit) do
      def to_s
        num = (value == value.to_i) ? value.to_i.to_s : value.to_s
        "#{num}#{unit}"
      end

      def truthy?
        true
      end
    end

    # A percentage value. Maps to CSS PERCENTAGE token.
    # Examples: 50%, 100%, 33.33%
    LatticePercentage = Struct.new(:value) do
      def to_s
        num = (value == value.to_i) ? value.to_i.to_s : value.to_s
        "#{num}%"
      end

      def truthy?
        true
      end
    end

    # A quoted string value. Maps to CSS STRING token.
    # The quotes are not stored — they're added back during emission.
    # Examples: "hello", 'world'
    LatticeString = Struct.new(:value) do
      def to_s
        "\"#{value}\""
      end

      def truthy?
        true
      end
    end

    # An unquoted identifier. Maps to CSS IDENT token.
    # CSS color keywords (red, blue) are idents, not a special type.
    # Examples: red, bold, dark, sans-serif, transparent
    LatticeIdent = Struct.new(:value) do
      def to_s
        value
      end

      def truthy?
        value != "null"
      end
    end

    # A hex color value. Maps to CSS HASH token in color context.
    # Stored as the raw string including the # prefix.
    # Examples: #4a90d9, #fff, #00000080
    LatticeColor = Struct.new(:value) do
      def to_s
        value
      end

      def truthy?
        true
      end
    end

    # A boolean value — true or false.
    # Lattice boolean literals are matched as "true" and "false".
    # Truthiness: false is falsy, true is truthy.
    LatticeBool = Struct.new(:value) do
      def to_s
        value ? "true" : "false"
      end

      def truthy?
        value
      end
    end

    # The null value.
    # null is falsy and stringifies to empty string (like Sass).
    # Used for optional parameters and missing values.
    LatticeNull = Struct.new do
      def to_s
        ""
      end

      def truthy?
        false
      end
    end

    # A comma-separated list of values. Used in @each directives.
    # Each item is a LatticeValue.
    LatticeList = Struct.new(:items) do
      def to_s
        items.map(&:to_s).join(", ")
      end

      def truthy?
        true
      end
    end

    # Type alias: any of the 9 value types above.
    LATTICE_VALUE_TYPES = [
      LatticeNumber, LatticeDimension, LatticePercentage,
      LatticeString, LatticeIdent, LatticeColor,
      LatticeBool, LatticeNull, LatticeList
    ].freeze

    # ============================================================
    # Truthiness Helper
    # ============================================================

    # Determine whether a Lattice value is truthy.
    #
    # Truthiness rules (matching Sass conventions):
    #   false  -> falsy
    #   null   -> falsy
    #   0      -> falsy (LatticeNumber with value 0)
    #   everything else -> truthy
    #
    # @param value [LatticeValue] the value to test
    # @return [Boolean]
    def self.truthy?(value)
      return value.truthy? if value.respond_to?(:truthy?)

      true
    end

    # ============================================================
    # Token <-> Value Conversion
    # ============================================================

    # Get the type name string from a token (handles both String
    # type names and enum-style objects with a .name or .to_s method).
    #
    # @param token [Object] a Token from the parser
    # @return [String] the type name as an uppercase string
    def self.token_type_name(token)
      t = token.type
      t.respond_to?(:name) ? t.name : t.to_s
    end

    # Convert a parser Token to a LatticeValue.
    #
    # Maps token types to value classes:
    #   NUMBER     -> LatticeNumber
    #   DIMENSION  -> LatticeDimension  (splits "16px" into 16, "px")
    #   PERCENTAGE -> LatticePercentage (strips "%" suffix)
    #   STRING     -> LatticeString
    #   HASH       -> LatticeColor
    #   IDENT      -> LatticeIdent (or LatticeBool / LatticeNull for literals)
    #
    # @param token [Object] a Token from the parser
    # @return [LatticeValue]
    def self.token_to_value(token)
      type = token_type_name(token)
      val = token.value

      case type
      when "NUMBER"
        LatticeNumber.new(val.to_f)
      when "DIMENSION"
        # Split "16px" into numeric part (16) and unit part ("px").
        # Find where the digit part ends and the unit begins.
        # The regex matches optional negative sign, digits, optional decimal.
        i = 0
        i += 1 if i < val.length && val[i] == "-"
        i += 1 while i < val.length && (val[i] =~ /[0-9.]/)
        num = val[0...i].to_f
        unit = val[i..]
        LatticeDimension.new(num, unit)
      when "PERCENTAGE"
        # "50%" -> LatticePercentage(50)
        LatticePercentage.new(val.chomp("%").to_f)
      when "STRING"
        # String tokens already have quotes stripped by the lexer.
        LatticeString.new(val)
      when "HASH"
        LatticeColor.new(val)
      when "IDENT"
        case val
        when "true" then LatticeBool.new(true)
        when "false" then LatticeBool.new(false)
        when "null" then LatticeNull.new
        else LatticeIdent.new(val)
        end
      else
        # Fallback for unexpected token types — treat as ident.
        LatticeIdent.new(val.to_s)
      end
    end

    # Convert a LatticeValue to its CSS text representation.
    #
    # Used when substituting evaluated values back into CSS output.
    #
    # @param value [LatticeValue]
    # @return [String] CSS text
    def self.value_to_css(value)
      value.to_s
    end

    # ============================================================
    # Expression Evaluator
    # ============================================================

    # Evaluates Lattice expression AST nodes at compile time.
    #
    # The evaluator walks the AST produced by the grammar parser's
    # expression rules (lattice_expression, lattice_or_expr, etc.)
    # and computes a LatticeValue result.
    #
    # The grammar's nesting of rules already encodes operator
    # precedence, so the evaluator just recursively evaluates each
    # node without needing its own precedence table.
    #
    # Usage:
    #   evaluator = ExpressionEvaluator.new(scope)
    #   result = evaluator.evaluate(expression_node)
    #   # result is a LatticeValue like LatticeNumber.new(42)
    class ExpressionEvaluator
      # @param scope [ScopeChain] the current scope for variable lookup
      # @param function_resolver [#call, nil] optional callback for evaluating
      #   Lattice function calls found in expressions. When non-nil, called as
      #   function_resolver.call(func_name, node, scope) and expected to return
      #   a LatticeValue. If nil, function calls in expressions are treated as
      #   CSS pass-throughs (returning LatticeIdent with the function token).
      def initialize(scope, function_resolver: nil)
        @scope = scope
        @function_resolver = function_resolver
      end

      # Evaluate an expression AST node and return a LatticeValue.
      #
      # Dispatches on rule_name to the appropriate handler. If the
      # node is a token (leaf), converts it directly to a value.
      #
      # @param node [Object] an ASTNode from the parser
      # @return [LatticeValue]
      def evaluate(node)
        # If it's a raw token (not an ASTNode), convert directly.
        unless node.respond_to?(:rule_name)
          return LatticeAstToCss.token_to_value(node)
        end

        rule = node.rule_name

        # Dispatch to a specific handler method if one exists.
        handler = :"eval_#{rule}"
        return send(handler, node) if respond_to?(handler, true)

        # For wrapper rules with a single child, unwrap and recurse.
        children = node.children
        return evaluate(children[0]) if children.size == 1

        # Default: evaluate the first meaningful child.
        children.each do |child|
          if child.respond_to?(:rule_name) || child.respond_to?(:type)
            return evaluate(child)
          end
        end

        LatticeNull.new
      end

      private

      # lattice_expression = lattice_or_expr ;
      def eval_lattice_expression(node)
        evaluate(node.children[0])
      end

      # lattice_or_expr = lattice_and_expr { "or" lattice_and_expr } ;
      #
      # Short-circuit evaluation: returns first truthy operand, or last.
      def eval_lattice_or_expr(node)
        children = node.children
        result = evaluate(children[0])
        i = 1
        while i < children.size
          child = children[i]
          if child.respond_to?(:value) && child.value == "or"
            i += 1
            next
          end
          return result if LatticeAstToCss.truthy?(result)

          result = evaluate(children[i])
          i += 1
        end
        result
      end

      # lattice_and_expr = lattice_comparison { "and" lattice_comparison } ;
      #
      # Short-circuit evaluation: returns first falsy operand, or last.
      def eval_lattice_and_expr(node)
        children = node.children
        result = evaluate(children[0])
        i = 1
        while i < children.size
          child = children[i]
          if child.respond_to?(:value) && child.value == "and"
            i += 1
            next
          end
          return result unless LatticeAstToCss.truthy?(result)

          result = evaluate(children[i])
          i += 1
        end
        result
      end

      # lattice_comparison = lattice_additive [ comparison_op lattice_additive ] ;
      def eval_lattice_comparison(node)
        children = node.children
        left = evaluate(children[0])
        return left if children.size == 1

        # Find the comparison_op node and right operand.
        op_node = nil
        right_node = nil
        children[1..].each_with_index do |child, idx|
          if child.respond_to?(:rule_name) && child.rule_name == "comparison_op"
            op_node = child
          elsif op_node && right_node.nil?
            right_node = child
          end
        end

        return left unless op_node && right_node

        right = evaluate(right_node)
        op_token = op_node.children[0]
        op_type = LatticeAstToCss.token_type_name(op_token)
        compare(left, right, op_type)
      end

      # Perform a comparison operation.
      def compare(left, right, op)
        # Numeric comparison for same types.
        numeric_types = [LatticeNumber, LatticeDimension, LatticePercentage]
        if numeric_types.any? { |t| left.is_a?(t) } && left.instance_of?(right.class)
          lv = left.value
          rv = right.value

          # For dimensions, units must match for ordering comparisons.
          if left.is_a?(LatticeDimension) && right.is_a?(LatticeDimension)
            if left.unit != right.unit && op !~ /EQUALS/
              return LatticeBool.new(op == "NOT_EQUALS")
            end
          end

          result = case op
          when "EQUALS_EQUALS"
            if left.is_a?(LatticeDimension) && right.is_a?(LatticeDimension)
              lv == rv && left.unit == right.unit
            else
              lv == rv
            end
          when "NOT_EQUALS"
            if left.is_a?(LatticeDimension) && right.is_a?(LatticeDimension)
              lv != rv || left.unit != right.unit
            else
              lv != rv
            end
          when "GREATER" then lv > rv
          when "GREATER_EQUALS" then lv >= rv
          when "LESS_EQUALS" then lv <= rv
          else false
          end
          return LatticeBool.new(result)
        end

        # Equality comparison via string representation for mixed types.
        left_str = left.to_s
        right_str = right.to_s
        case op
        when "EQUALS_EQUALS" then LatticeBool.new(left_str == right_str)
        when "NOT_EQUALS" then LatticeBool.new(left_str != right_str)
        else LatticeBool.new(false)
        end
      end

      # lattice_additive = lattice_multiplicative { (PLUS|MINUS) lattice_multiplicative } ;
      def eval_lattice_additive(node)
        children = node.children
        result = evaluate(children[0])
        i = 1
        while i < children.size
          child = children[i]
          if child.respond_to?(:value) && (child.value == "+" || child.value == "-")
            op = child.value
            i += 1
            right = evaluate(children[i])
            result = (op == "+") ? add(result, right) : subtract(result, right)
          end
          i += 1
        end
        result
      end

      # Addition: Number + Number, Dimension + Dimension (same unit), etc.
      def add(left, right)
        if left.is_a?(LatticeNumber) && right.is_a?(LatticeNumber)
          return LatticeNumber.new(left.value + right.value)
        end
        if left.is_a?(LatticeDimension) && right.is_a?(LatticeDimension)
          if left.unit == right.unit
            return LatticeDimension.new(left.value + right.value, left.unit)
          end
          raise LatticeTypeErrorInExpression.new("add", left.to_s, right.to_s)
        end
        if left.is_a?(LatticePercentage) && right.is_a?(LatticePercentage)
          return LatticePercentage.new(left.value + right.value)
        end
        if left.is_a?(LatticeString) && right.is_a?(LatticeString)
          return LatticeString.new(left.value + right.value)
        end
        raise LatticeTypeErrorInExpression.new("add", left.to_s, right.to_s)
      end

      # Subtraction: mirrors addition but subtracts.
      def subtract(left, right)
        if left.is_a?(LatticeNumber) && right.is_a?(LatticeNumber)
          return LatticeNumber.new(left.value - right.value)
        end
        if left.is_a?(LatticeDimension) && right.is_a?(LatticeDimension)
          if left.unit == right.unit
            return LatticeDimension.new(left.value - right.value, left.unit)
          end
          raise LatticeTypeErrorInExpression.new("subtract", left.to_s, right.to_s)
        end
        if left.is_a?(LatticePercentage) && right.is_a?(LatticePercentage)
          return LatticePercentage.new(left.value - right.value)
        end
        raise LatticeTypeErrorInExpression.new("subtract", left.to_s, right.to_s)
      end

      # lattice_multiplicative = lattice_unary { STAR lattice_unary } ;
      def eval_lattice_multiplicative(node)
        children = node.children
        result = evaluate(children[0])
        i = 1
        while i < children.size
          child = children[i]
          if child.respond_to?(:value) && child.value == "*"
            i += 1
            right = evaluate(children[i])
            result = multiply(result, right)
          end
          i += 1
        end
        result
      end

      # Multiplication rules.
      def multiply(left, right)
        if left.is_a?(LatticeNumber) && right.is_a?(LatticeNumber)
          return LatticeNumber.new(left.value * right.value)
        end
        if left.is_a?(LatticeNumber) && right.is_a?(LatticeDimension)
          return LatticeDimension.new(left.value * right.value, right.unit)
        end
        if left.is_a?(LatticeDimension) && right.is_a?(LatticeNumber)
          return LatticeDimension.new(left.value * right.value, left.unit)
        end
        if left.is_a?(LatticeNumber) && right.is_a?(LatticePercentage)
          return LatticePercentage.new(left.value * right.value)
        end
        if left.is_a?(LatticePercentage) && right.is_a?(LatticeNumber)
          return LatticePercentage.new(left.value * right.value)
        end
        raise LatticeTypeErrorInExpression.new("multiply", left.to_s, right.to_s)
      end

      # lattice_unary = MINUS lattice_unary | lattice_primary ;
      def eval_lattice_unary(node)
        children = node.children
        if children.size >= 2 && children[0].respond_to?(:value) && children[0].value == "-"
          operand = evaluate(children[1])
          return negate(operand)
        end
        evaluate(children[0])
      end

      # Negate a numeric value.
      def negate(value)
        case value
        when LatticeNumber then LatticeNumber.new(-value.value)
        when LatticeDimension then LatticeDimension.new(-value.value, value.unit)
        when LatticePercentage then LatticePercentage.new(-value.value)
        else
          raise LatticeTypeErrorInExpression.new("negate", value.to_s, "")
        end
      end

      # lattice_primary = VARIABLE | NUMBER | DIMENSION | PERCENTAGE
      #                 | STRING | IDENT | HASH
      #                 | "true" | "false" | "null"
      #                 | function_call
      #                 | LPAREN lattice_expression RPAREN ;
      def eval_lattice_primary(node)
        node.children.each do |child|
          unless child.respond_to?(:rule_name)
            type = LatticeAstToCss.token_type_name(child)
            val = child.value

            # Skip parentheses — they're structural, not values.
            next if type == "LPAREN" || type == "RPAREN"

            if type == "VARIABLE"
              result = @scope.get(val)
              if result.nil?
                # Return ident for now; transformer will raise proper error.
                return LatticeIdent.new(val)
              end
              if LATTICE_VALUE_TYPES.any? { |t| result.is_a?(t) }
                return result
              end
              # If bound to an ASTNode (value_list), extract its value.
              if result.respond_to?(:rule_name)
                return extract_value_from_ast(result)
              end
              return LatticeAstToCss.token_to_value(result)
            end

            return LatticeAstToCss.token_to_value(child)
          end

          # It's an ASTNode — recurse.
          return evaluate(child) if child.respond_to?(:rule_name)
        end

        LatticeNull.new
      end

      # Extract a LatticeValue from an AST node.
      #
      # When a variable is bound to a value_list node (from the parser),
      # we extract the actual value. For multi-token value_lists
      # (e.g., "Helvetica, sans-serif"), we take the first token.
      def extract_value_from_ast(node)
        if node.respond_to?(:children)
          node.children.each do |child|
            unless child.respond_to?(:rule_name)
              return LatticeAstToCss.token_to_value(child)
            end
            result = extract_value_from_ast(child)
            return result unless result.is_a?(LatticeNull)
          end
        end
        LatticeNull.new
      end

      # function_call nodes inside lattice_expression.
      #
      # function_call = FUNCTION function_args RPAREN | URL_TOKEN ;
      #
      # If a function_resolver was provided (by the transformer), delegate to
      # it so that user-defined Lattice functions can be evaluated at compile
      # time and cycle detection can run. Otherwise treat the call as a CSS
      # built-in and return a LatticeIdent representing the raw call text.
      def eval_function_call(node)
        if @function_resolver
          # Extract the function name from the FUNCTION token.
          func_name = nil
          node.children.each do |child|
            unless child.respond_to?(:rule_name)
              if LatticeAstToCss.token_type_name(child) == "FUNCTION"
                func_name = child.value.chomp("(")
                break
              end
            end
          end
          return @function_resolver.call(func_name, node, @scope) if func_name
        end

        # CSS built-in pass-through: return null (the CSS emitter renders it).
        LatticeNull.new
      end

      # comparison_op is handled by eval_lattice_comparison.
      def eval_comparison_op(node)
        LatticeAstToCss.token_to_value(node.children[0])
      end
    end
  end
end
