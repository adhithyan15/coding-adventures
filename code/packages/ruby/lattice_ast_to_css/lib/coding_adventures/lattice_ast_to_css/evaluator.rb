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
    #
    # Provides conversion helpers for RGB and HSL color spaces,
    # needed by Lattice v2 built-in color functions (lighten, darken, etc.).
    LatticeColor = Struct.new(:value) do
      def to_s
        value
      end

      def truthy?
        true
      end

      # Parse hex string to [r, g, b, a] where r/g/b are 0-255, a is 0.0-1.0.
      # Handles #RGB (3-char), #RRGGBB (6-char), and #RRGGBBAA (8-char).
      def to_rgb
        h = value.sub("#", "")
        case h.length
        when 3
          r = (h[0] * 2).to_i(16)
          g = (h[1] * 2).to_i(16)
          b = (h[2] * 2).to_i(16)
          [r, g, b, 1.0]
        when 6
          [h[0..1].to_i(16), h[2..3].to_i(16), h[4..5].to_i(16), 1.0]
        when 8
          [h[0..1].to_i(16), h[2..3].to_i(16), h[4..5].to_i(16), h[6..7].to_i(16) / 255.0]
        else
          [0, 0, 0, 1.0]
        end
      end

      # Convert to [h, s, l, a] where h is 0-360, s/l are 0-100, a is 0-1.
      def to_hsl
        r, g, b, a = to_rgb
        rf = r / 255.0
        gf = g / 255.0
        bf = b / 255.0
        mx = [rf, gf, bf].max
        mn = [rf, gf, bf].min
        light = (mx + mn) / 2.0

        return [0.0, 0.0, light * 100.0, a] if mx == mn

        d = mx - mn
        sat = light > 0.5 ? d / (2.0 - mx - mn) : d / (mx + mn)

        hue = if mx == rf
               (gf - bf) / d + (gf < bf ? 6.0 : 0.0)
             elsif mx == gf
               (bf - rf) / d + 2.0
             else
               (rf - gf) / d + 4.0
             end
        hue *= 60.0

        [hue, sat * 100.0, light * 100.0, a]
      end
    end

    # Class-level helper to construct a LatticeColor from RGB(A) components.
    # Clamps each channel to its valid range before encoding as hex.
    def self.color_from_rgb(r, g, b, a = 1.0)
      r = [[0, r.round].max, 255].min
      g = [[0, g.round].max, 255].min
      b = [[0, b.round].max, 255].min
      a = [[0.0, a].max, 1.0].min
      if a >= 1.0
        LatticeColor.new(format("#%02x%02x%02x", r, g, b))
      else
        LatticeColor.new("rgba(#{r}, #{g}, #{b}, #{a})")
      end
    end

    # Class-level helper to construct a LatticeColor from HSL(A) components.
    # Uses the standard HSL-to-RGB algorithm.
    def self.color_from_hsl(h, s, l, a = 1.0)
      h = h % 360.0
      s = [[0.0, s].max, 100.0].min / 100.0
      l = [[0.0, l].max, 100.0].min / 100.0

      if s == 0.0
        v = (l * 255).round
        return color_from_rgb(v, v, v, a)
      end

      q = l < 0.5 ? l * (1 + s) : l + s - l * s
      p = 2 * l - q

      hue_to_rgb = lambda do |pp, qq, t|
        t += 1 if t < 0
        t -= 1 if t > 1
        return pp + (qq - pp) * 6 * t if t < 1.0 / 6
        return qq if t < 1.0 / 2
        return pp + (qq - pp) * (2.0 / 3 - t) * 6 if t < 2.0 / 3

        pp
      end

      h_norm = h / 360.0
      r = (hue_to_rgb.call(p, q, h_norm + 1.0 / 3) * 255).round
      g = (hue_to_rgb.call(p, q, h_norm) * 255).round
      b = (hue_to_rgb.call(p, q, h_norm - 1.0 / 3) * 255).round
      color_from_rgb(r, g, b, a)
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

    # An ordered key-value map -- Lattice v2 value type.
    #
    # Maps are written as parenthesized key-value pairs:
    #
    #   $theme: (
    #     primary: #4a90d9,
    #     secondary: #7b68ee,
    #   );
    #
    # Stored as an array of [key, value] pairs to maintain insertion order.
    # Access is exclusively through built-in functions: map-get, map-keys, etc.
    LatticeMap = Struct.new(:items) do
      # Look up a value by key. Returns nil if not found.
      def get(key)
        items.each { |k, v| return v if k == key }
        nil
      end

      # Return all keys in insertion order.
      def keys
        items.map { |k, _| k }
      end

      # Return all values in insertion order.
      def values
        items.map { |_, v| v }
      end

      # Check if a key exists.
      def has_key?(key)
        items.any? { |k, _| k == key }
      end

      def to_s
        entries = items.map { |k, v| "#{k}: #{v}" }.join(", ")
        "(#{entries})"
      end

      def truthy?
        true
      end
    end

    # Type alias: any of the 10 value types.
    LATTICE_VALUE_TYPES = [
      LatticeNumber, LatticeDimension, LatticePercentage,
      LatticeString, LatticeIdent, LatticeColor,
      LatticeBool, LatticeNull, LatticeList, LatticeMap
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

      # value_list — used when variable substitution produces a flat list of
      # tokens (e.g., `$i + 1` becomes a value_list with children
      # [NUMBER(2), PLUS, NUMBER(1)]). If arithmetic operators are present,
      # delegate to the additive handler; otherwise evaluate the first child.
      def eval_value_list(node)
        children = node.children
        return evaluate(children[0]) if children.size <= 1

        has_ops = children.any? do |c|
          !c.respond_to?(:rule_name) && c.respond_to?(:value) &&
            ["+", "-", "*"].include?(c.value)
        end
        return eval_lattice_additive(node) if has_ops

        evaluate(children[0])
      end

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
          when "LESS" then lv < rv
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
          if child.respond_to?(:value) && (child.value == "*" || child.value == "/")
            op = child.value
            i += 1
            right = evaluate(children[i])
            result = op == "*" ? multiply(result, right) : divide(result, right)
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

      # Division rules mirror multiplication in reverse.
      def divide(left, right)
        rv = right.respond_to?(:value) ? right.value : nil
        raise LatticeZeroDivisionError.new if rv == 0
        if left.is_a?(LatticeNumber) && right.is_a?(LatticeNumber)
          return LatticeNumber.new(left.value / right.value.to_f)
        end
        if left.is_a?(LatticeDimension) && right.is_a?(LatticeNumber)
          return LatticeDimension.new(left.value / right.value.to_f, left.unit)
        end
        if left.is_a?(LatticeDimension) && right.is_a?(LatticeDimension) && left.unit == right.unit
          return LatticeNumber.new(left.value / right.value.to_f)
        end
        if left.is_a?(LatticePercentage) && right.is_a?(LatticeNumber)
          return LatticePercentage.new(left.value / right.value.to_f)
        end
        raise LatticeTypeErrorInExpression.new("divide", left.to_s, right.to_s)
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

      # Collect and evaluate arguments from a function_args AST node.
      # Splits on COMMA tokens to produce individual argument values.
      # Public so the transformer can call it for built-in function evaluation.
      public

      def collect_function_args(node)
        args = []
        current_tokens = []

        node.children.each do |child|
          unless child.respond_to?(:rule_name)
            if LatticeAstToCss.token_type_name(child) == "COMMA"
              args << eval_arg_tokens(current_tokens) unless current_tokens.empty?
              current_tokens = []
              next
            end
          end

          if child.respond_to?(:rule_name) && child.rule_name == "function_arg"
            child.children.each do |ic|
              unless ic.respond_to?(:rule_name)
                if LatticeAstToCss.token_type_name(ic) == "COMMA"
                  args << eval_arg_tokens(current_tokens) unless current_tokens.empty?
                  current_tokens = []
                  next
                end
                current_tokens << ic
              else
                args << evaluate(ic)
                current_tokens = []
              end
            end
          end
        end

        args << eval_arg_tokens(current_tokens) unless current_tokens.empty?
        args
      end

      private

      def eval_arg_tokens(tokens)
        return LatticeNull.new if tokens.empty?

        if tokens.size == 1
          tok = tokens[0]
          type = LatticeAstToCss.token_type_name(tok)
          if type == "VARIABLE"
            result = @scope.get(tok.value)
            if result
              return result if LATTICE_VALUE_TYPES.any? { |t| result.is_a?(t) }
              return extract_value_from_ast(result) if result.respond_to?(:rule_name)
            end
          end
          return LatticeAstToCss.token_to_value(tok)
        end

        LatticeAstToCss.token_to_value(tokens[0])
      end
    end

    # ============================================================
    # Built-in Function Registry -- Lattice v2
    # ============================================================
    #
    # Built-in functions are registered in BUILTIN_FUNCTIONS hash.
    # Each function takes (args, scope) and returns a LatticeValue.
    #
    # Categories:
    #   Map:   map-get, map-keys, map-values, map-has-key, map-merge, map-remove
    #   Color: lighten, darken, saturate, desaturate, adjust-hue, complement,
    #          mix, rgba, red, green, blue, hue, saturation, lightness
    #   List:  nth, length, join, append, index
    #   Type:  type-of, unit, unitless, comparable
    #   Math:  math.div, math.floor, math.ceil, math.round, math.abs,
    #          math.min, math.max
    # ============================================================

    # Return the Lattice type name for a value.
    def self.type_name_of(value)
      case value
      when LatticeNumber, LatticeDimension, LatticePercentage then "number"
      when LatticeString, LatticeIdent then "string"
      when LatticeColor then "color"
      when LatticeBool then "bool"
      when LatticeNull then "null"
      when LatticeList then "list"
      when LatticeMap then "map"
      else "unknown"
      end
    end

    # Extract the numeric value from a number-like LatticeValue.
    def self.get_numeric_value(v)
      case v
      when LatticeNumber, LatticeDimension, LatticePercentage then v.value
      else
        raise LatticeTypeErrorInExpression.new("use", "Expected a number, got #{type_name_of(v)}", "")
      end
    end

    # Ensure a value is a LatticeColor.
    def self.ensure_color(v)
      unless v.is_a?(LatticeColor)
        raise LatticeTypeErrorInExpression.new("use", "Expected a color, got #{type_name_of(v)}", "")
      end
      v
    end

    # Extract a percentage amount (0-100) from a value.
    def self.ensure_amount(v)
      val = get_numeric_value(v)
      raise LatticeRangeError.new("Amount must be between 0% and 100%") if val < 0 || val > 100

      val
    end

    BUILTIN_FUNCTIONS = {
      # -- Map functions --
      "map-get" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "map-get requires 2 arguments", "") if args.size < 2
        m = args[0]
        raise LatticeTypeErrorInExpression.new("use", "Expected a map, got #{LatticeAstToCss.type_name_of(m)}", "") unless m.is_a?(LatticeMap)
        key = args[1].to_s.delete('"')
        result = m.get(key)
        result || LatticeNull.new
      },
      "map-keys" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "map-keys requires 1 argument", "") if args.empty?
        m = args[0]
        raise LatticeTypeErrorInExpression.new("use", "Expected a map, got #{LatticeAstToCss.type_name_of(m)}", "") unless m.is_a?(LatticeMap)
        LatticeList.new(m.keys.map { |k| LatticeIdent.new(k) })
      },
      "map-values" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "map-values requires 1 argument", "") if args.empty?
        m = args[0]
        raise LatticeTypeErrorInExpression.new("use", "Expected a map, got #{LatticeAstToCss.type_name_of(m)}", "") unless m.is_a?(LatticeMap)
        LatticeList.new(m.values)
      },
      "map-has-key" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "map-has-key requires 2 arguments", "") if args.size < 2
        m = args[0]
        raise LatticeTypeErrorInExpression.new("use", "Expected a map, got #{LatticeAstToCss.type_name_of(m)}", "") unless m.is_a?(LatticeMap)
        key = args[1].to_s.delete('"')
        LatticeBool.new(m.has_key?(key))
      },
      "map-merge" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "map-merge requires 2 arguments", "") if args.size < 2
        m1 = args[0]
        m2 = args[1]
        raise LatticeTypeErrorInExpression.new("use", "Expected a map, got #{LatticeAstToCss.type_name_of(m1)}", "") unless m1.is_a?(LatticeMap)
        raise LatticeTypeErrorInExpression.new("use", "Expected a map, got #{LatticeAstToCss.type_name_of(m2)}", "") unless m2.is_a?(LatticeMap)
        merged = m1.items.to_h
        m2.items.each { |k, v| merged[k] = v }
        LatticeMap.new(merged.to_a)
      },
      "map-remove" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "map-remove requires at least 1 argument", "") if args.empty?
        m = args[0]
        raise LatticeTypeErrorInExpression.new("use", "Expected a map, got #{LatticeAstToCss.type_name_of(m)}", "") unless m.is_a?(LatticeMap)
        keys_to_remove = args[1..].map { |a| a.to_s.delete('"') }
        LatticeMap.new(m.items.reject { |k, _| keys_to_remove.include?(k) })
      },

      # -- Color functions --
      "lighten" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        amount = LatticeAstToCss.ensure_amount(args[1])
        h, s, l, a = color.to_hsl
        l = [100.0, l + amount].min
        LatticeAstToCss.color_from_hsl(h, s, l, a)
      },
      "darken" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        amount = LatticeAstToCss.ensure_amount(args[1])
        h, s, l, a = color.to_hsl
        l = [0.0, l - amount].max
        LatticeAstToCss.color_from_hsl(h, s, l, a)
      },
      "saturate" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        amount = LatticeAstToCss.ensure_amount(args[1])
        h, s, l, a = color.to_hsl
        s = [100.0, s + amount].min
        LatticeAstToCss.color_from_hsl(h, s, l, a)
      },
      "desaturate" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        amount = LatticeAstToCss.ensure_amount(args[1])
        h, s, l, a = color.to_hsl
        s = [0.0, s - amount].max
        LatticeAstToCss.color_from_hsl(h, s, l, a)
      },
      "adjust-hue" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        degrees = LatticeAstToCss.get_numeric_value(args[1])
        h, s, l, a = color.to_hsl
        h = (h + degrees) % 360.0
        LatticeAstToCss.color_from_hsl(h, s, l, a)
      },
      "complement" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        h, s, l, a = color.to_hsl
        h = (h + 180.0) % 360.0
        LatticeAstToCss.color_from_hsl(h, s, l, a)
      },
      "mix" => lambda { |args, _scope|
        c1 = LatticeAstToCss.ensure_color(args[0])
        c2 = LatticeAstToCss.ensure_color(args[1])
        weight = args.size >= 3 ? LatticeAstToCss.get_numeric_value(args[2]) : 50.0
        w = weight / 100.0
        r1, g1, b1, a1 = c1.to_rgb
        r2, g2, b2, a2 = c2.to_rgb
        r = (r1 * w + r2 * (1 - w)).round
        g = (g1 * w + g2 * (1 - w)).round
        b = (b1 * w + b2 * (1 - w)).round
        a = a1 * w + a2 * (1 - w)
        LatticeAstToCss.color_from_rgb(r, g, b, a)
      },
      "rgba" => lambda { |args, _scope|
        if args.size == 2 && args[0].is_a?(LatticeColor)
          r, g, b, _ = args[0].to_rgb
          alpha = LatticeAstToCss.get_numeric_value(args[1])
          LatticeAstToCss.color_from_rgb(r, g, b, alpha)
        elsif args.size == 4
          r = LatticeAstToCss.get_numeric_value(args[0]).round
          g = LatticeAstToCss.get_numeric_value(args[1]).round
          b = LatticeAstToCss.get_numeric_value(args[2]).round
          a = LatticeAstToCss.get_numeric_value(args[3])
          LatticeAstToCss.color_from_rgb(r, g, b, a)
        else
          LatticeNull.new
        end
      },
      "red" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        r, _, _, _ = color.to_rgb
        LatticeNumber.new(r.to_f)
      },
      "green" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        _, g, _, _ = color.to_rgb
        LatticeNumber.new(g.to_f)
      },
      "blue" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        _, _, b, _ = color.to_rgb
        LatticeNumber.new(b.to_f)
      },
      "hue" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        h, _, _, _ = color.to_hsl
        LatticeDimension.new(h.round.to_f, "deg")
      },
      "saturation" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        _, s, _, _ = color.to_hsl
        LatticePercentage.new(s.round.to_f)
      },
      "lightness" => lambda { |args, _scope|
        color = LatticeAstToCss.ensure_color(args[0])
        _, _, l, _ = color.to_hsl
        LatticePercentage.new(l.round.to_f)
      },

      # -- List functions --
      "nth" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "nth requires 2 arguments", "") if args.size < 2
        lst = args[0]
        n = LatticeAstToCss.get_numeric_value(args[1]).to_i
        raise LatticeRangeError.new("List index must be 1 or greater") if n < 1
        if lst.is_a?(LatticeList)
          raise LatticeRangeError.new("Index #{n} out of bounds for list of length #{lst.items.size}") if n > lst.items.size
          lst.items[n - 1]
        elsif n == 1
          lst
        else
          raise LatticeRangeError.new("Index #{n} out of bounds for list of length 1")
        end
      },
      "length" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "length requires 1 argument", "") if args.empty?
        v = args[0]
        case v
        when LatticeList then LatticeNumber.new(v.items.size.to_f)
        when LatticeMap then LatticeNumber.new(v.items.size.to_f)
        else LatticeNumber.new(1.0)
        end
      },
      "join" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "join requires at least 2 arguments", "") if args.size < 2
        items1 = args[0].is_a?(LatticeList) ? args[0].items : [args[0]]
        items2 = args[1].is_a?(LatticeList) ? args[1].items : [args[1]]
        LatticeList.new(items1 + items2)
      },
      "append" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "append requires at least 2 arguments", "") if args.size < 2
        items = args[0].is_a?(LatticeList) ? args[0].items : [args[0]]
        LatticeList.new(items + [args[1]])
      },
      "index" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "index requires 2 arguments", "") if args.size < 2
        items = args[0].is_a?(LatticeList) ? args[0].items : [args[0]]
        target_str = args[1].to_s
        items.each_with_index do |item, i|
          return LatticeNumber.new((i + 1).to_f) if item.to_s == target_str
        end
        LatticeNull.new
      },

      # -- Type introspection functions --
      "type-of" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "type-of requires 1 argument", "") if args.empty?
        LatticeString.new(LatticeAstToCss.type_name_of(args[0]))
      },
      "unit" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "unit requires 1 argument", "") if args.empty?
        v = args[0]
        case v
        when LatticeDimension then LatticeString.new(v.unit)
        when LatticePercentage then LatticeString.new("%")
        when LatticeNumber then LatticeString.new("")
        else raise LatticeTypeErrorInExpression.new("use", "Expected a number, got #{LatticeAstToCss.type_name_of(v)}", "")
        end
      },
      "unitless" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "unitless requires 1 argument", "") if args.empty?
        LatticeBool.new(args[0].is_a?(LatticeNumber))
      },
      "comparable" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "comparable requires 2 arguments", "") if args.size < 2
        a, b = args[0], args[1]
        if a.instance_of?(b.class)
          if a.is_a?(LatticeDimension) && b.is_a?(LatticeDimension)
            LatticeBool.new(a.unit == b.unit)
          else
            LatticeBool.new(true)
          end
        elsif [LatticeNumber, LatticeDimension, LatticePercentage].any? { |t| a.is_a?(t) } &&
              [LatticeNumber, LatticeDimension, LatticePercentage].any? { |t| b.is_a?(t) }
          LatticeBool.new(a.is_a?(LatticeNumber) || b.is_a?(LatticeNumber))
        else
          LatticeBool.new(false)
        end
      },

      # -- Math functions --
      "math.div" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "math.div requires 2 arguments", "") if args.size < 2
        a, b = args[0], args[1]
        b_val = LatticeAstToCss.get_numeric_value(b)
        raise LatticeZeroDivisionError.new if b_val == 0
        a_val = LatticeAstToCss.get_numeric_value(a)
        if a.is_a?(LatticeDimension) && b.is_a?(LatticeNumber)
          LatticeDimension.new(a_val / b_val, a.unit)
        elsif a.is_a?(LatticeDimension) && b.is_a?(LatticeDimension) && a.unit == b.unit
          LatticeNumber.new(a_val / b_val)
        elsif a.is_a?(LatticePercentage) && b.is_a?(LatticeNumber)
          LatticePercentage.new(a_val / b_val)
        else
          LatticeNumber.new(a_val / b_val)
        end
      },
      "math.floor" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "math.floor requires 1 argument", "") if args.empty?
        v = args[0]
        val = LatticeAstToCss.get_numeric_value(v)
        result = val.floor.to_f
        case v
        when LatticeDimension then LatticeDimension.new(result, v.unit)
        when LatticePercentage then LatticePercentage.new(result)
        else LatticeNumber.new(result)
        end
      },
      "math.ceil" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "math.ceil requires 1 argument", "") if args.empty?
        v = args[0]
        val = LatticeAstToCss.get_numeric_value(v)
        result = val.ceil.to_f
        case v
        when LatticeDimension then LatticeDimension.new(result, v.unit)
        when LatticePercentage then LatticePercentage.new(result)
        else LatticeNumber.new(result)
        end
      },
      "math.round" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "math.round requires 1 argument", "") if args.empty?
        v = args[0]
        val = LatticeAstToCss.get_numeric_value(v)
        result = val.round.to_f
        case v
        when LatticeDimension then LatticeDimension.new(result, v.unit)
        when LatticePercentage then LatticePercentage.new(result)
        else LatticeNumber.new(result)
        end
      },
      "math.abs" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "math.abs requires 1 argument", "") if args.empty?
        v = args[0]
        val = LatticeAstToCss.get_numeric_value(v)
        result = val.abs
        case v
        when LatticeDimension then LatticeDimension.new(result, v.unit)
        when LatticePercentage then LatticePercentage.new(result)
        else LatticeNumber.new(result)
        end
      },
      "math.min" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "math.min requires at least 1 argument", "") if args.empty?
        best = args[0]
        best_val = LatticeAstToCss.get_numeric_value(best)
        args[1..].each do |arg|
          val = LatticeAstToCss.get_numeric_value(arg)
          if val < best_val
            best = arg
            best_val = val
          end
        end
        best
      },
      "math.max" => lambda { |args, _scope|
        raise LatticeTypeErrorInExpression.new("call", "math.max requires at least 1 argument", "") if args.empty?
        best = args[0]
        best_val = LatticeAstToCss.get_numeric_value(best)
        args[1..].each do |arg|
          val = LatticeAstToCss.get_numeric_value(arg)
          if val > best_val
            best = arg
            best_val = val
          end
        end
        best
      }
    }.freeze
  end
end
