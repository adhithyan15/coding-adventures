# frozen_string_literal: true

# ================================================================
# CSS Emitter -- Reconstructs CSS Text from a Clean AST
# ================================================================
#
# After the transformer has expanded all Lattice nodes (variables,
# mixins, control flow, functions), the AST contains only pure CSS
# nodes. The emitter walks this tree and produces formatted CSS text.
#
# CSS node types the emitter handles:
#   stylesheet       -- the root
#   qualified_rule   -- selector + block (h1 { color: red; })
#   at_rule          -- @-rules (@media, @import)
#   selector_list    -- comma-separated selectors
#   complex_selector -- compound selectors with combinators
#   compound_selector -- type/class/id/pseudo selectors
#   block            -- { declarations }
#   declaration      -- property: value;
#   value_list       -- space-separated values
#   function_call    -- rgb(255, 0, 0)
#   priority         -- !important
#
# Two formatting modes:
#
#   Pretty-print (default):
#     - 2-space indentation per nesting level
#     - Newlines between declarations
#     - Blank lines between rules
#
#   Minified:
#     - No unnecessary whitespace
#
# Unknown rules fall through to a default handler that concatenates
# children with spaces. Lattice nodes (if any remain) are silently
# skipped.
#
# Example:
#
#   emitter = CSSEmitter.new
#   css = emitter.emit(ast)
#   # => "h1 {\n  color: red;\n}\n"
# ================================================================

module CodingAdventures
  module LatticeAstToCss
    # Emits CSS text from a clean AST (no Lattice nodes).
    class CSSEmitter
      # @param indent [String] indentation per nesting level (default: 2 spaces)
      # @param minified [Boolean] if true, emit minified CSS
      def initialize(indent: "  ", minified: false)
        @indent = indent
        @minified = minified
      end

      # Emit CSS text from an AST node.
      #
      # Pass the root stylesheet node to get a complete CSS string.
      #
      # @param node [Object] an ASTNode (typically the root stylesheet)
      # @return [String] formatted CSS text
      def emit(node)
        result = emit_node(node, 0)
        stripped = result.strip
        return "" if stripped.empty?

        # Minified output is returned as-is (no trailing newline).
        # Pretty-printed output ends with a single newline for clean
        # file output (POSIX convention: text files end with newline).
        @minified ? stripped : "#{stripped}\n"
      end

      private

      # Dispatch to the appropriate handler based on rule_name.
      #
      # If the node is a token (no rule_name), return its text value.
      # If the rule has a specific handler, use it. Otherwise fall
      # through to the default handler.
      def emit_node(node, depth)
        # Raw token — return its text value.
        unless node.respond_to?(:rule_name)
          return node.respond_to?(:value) ? node.value.to_s : ""
        end

        rule = node.rule_name

        # Dispatch to specific handler method.
        handler = :"emit_#{rule}"
        return send(handler, node, depth) if respond_to?(handler, true)

        # Default: recurse children and concatenate.
        emit_default(node, depth)
      end

      # ============================================================
      # Top-Level Structure
      # ============================================================

      # stylesheet = { rule } ;
      # Join rules with blank lines (pretty) or nothing (minified).
      def emit_stylesheet(node, depth)
        parts = []
        node.children.each do |child|
          text = emit_node(child, depth)
          parts << text if text.strip.length.positive?
        end

        @minified ? parts.join : parts.join("\n\n")
      end

      # rule = lattice_rule | at_rule | qualified_rule ;
      # A rule is a wrapper — just emit the single child.
      def emit_rule(node, depth)
        node.children.empty? ? "" : emit_node(node.children[0], depth)
      end

      # ============================================================
      # Qualified Rules (selector + block)
      # ============================================================

      # qualified_rule = selector_list block ;
      def emit_qualified_rule(node, depth)
        selector = ""
        block_text = ""

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            case child.rule_name
            when "selector_list"
              selector = emit_node(child, depth)
            when "block"
              block_text = emit_block(child, depth)
            else
              emit_node(child, depth)
            end
          end
        end

        return block_text if selector.empty?

        @minified ? "#{selector}#{block_text}" : "#{selector} #{block_text}"
      end

      # ============================================================
      # At-Rules
      # ============================================================

      # at_rule = AT_KEYWORD at_prelude ( SEMICOLON | block ) ;
      def emit_at_rule(node, depth)
        keyword = ""
        prelude = ""
        block_text = ""
        has_semicolon = false

        node.children.each do |child|
          if child.respond_to?(:rule_name)
            case child.rule_name
            when "at_prelude"
              prelude = emit_at_prelude(child, depth)
            when "block"
              block_text = emit_block(child, depth)
            end
          else
            type_name = token_type(child)
            if type_name == "AT_KEYWORD"
              keyword = child.value
            elsif type_name == "SEMICOLON"
              has_semicolon = true
            end
          end
        end

        if @minified
          has_semicolon ? "#{keyword}#{prelude};" : "#{keyword}#{prelude}#{block_text}"
        elsif has_semicolon
          prelude_part = prelude.strip.empty? ? "" : " #{prelude}"
          "#{keyword}#{prelude_part};"
        else
          prelude_part = prelude.strip.empty? ? "" : " #{prelude}"
          "#{keyword}#{prelude_part} #{block_text}"
        end
      end

      # at_prelude = { at_prelude_token } ;
      def emit_at_prelude(node, depth)
        parts = node.children.map { |c| emit_node(c, depth) }
        parts.join(" ")
      end

      def emit_at_prelude_token(node, depth)
        emit_default(node, depth)
      end

      def emit_at_prelude_tokens(node, depth)
        parts = node.children.map { |c| emit_node(c, depth) }
        parts.join(" ")
      end

      # function_in_prelude = FUNCTION at_prelude_tokens RPAREN ;
      def emit_function_in_prelude(node, depth)
        parts = []
        node.children.each do |child|
          parts << if child.respond_to?(:rule_name)
            emit_node(child, depth)
          else
            ((token_type(child) == "RPAREN") ? ")" : child.value)
          end
        end
        parts.join
      end

      # paren_block = LPAREN at_prelude_tokens RPAREN ;
      def emit_paren_block(node, depth)
        parts = []
        node.children.each do |child|
          parts << if child.respond_to?(:rule_name)
            emit_node(child, depth)
          else
            case token_type(child)
            when "LPAREN" then "("
            when "RPAREN" then ")"
            else child.value
            end
          end
        end
        parts.join
      end

      # ============================================================
      # Selectors
      # ============================================================

      # selector_list = complex_selector { COMMA complex_selector } ;
      def emit_selector_list(node, depth)
        parts = []
        node.children.each do |child|
          if child.respond_to?(:rule_name)
            parts << emit_node(child, depth)
          elsif token_type(child) == "COMMA"
            next
          end
        end
        sep = @minified ? "," : ", "
        parts.join(sep)
      end

      # complex_selector = compound_selector { [ combinator ] compound_selector } ;
      def emit_complex_selector(node, depth)
        parts = node.children.map { |c| emit_node(c, depth) }
        parts.join(" ")
      end

      # combinator = GREATER | PLUS | TILDE ;
      def emit_combinator(node, _depth)
        node.children.empty? ? "" : node.children[0].value
      end

      # compound_selector = simple_selector { subclass_selector } ;
      # Concatenate without spaces: h1.classname#id
      def emit_compound_selector(node, depth)
        parts = node.children.map { |c| emit_node(c, depth) }
        parts.join
      end

      # simple_selector = IDENT | STAR | AMPERSAND ;
      def emit_simple_selector(node, _depth)
        node.children.empty? ? "" : node.children[0].value
      end

      # subclass_selector — dispatch to child.
      def emit_subclass_selector(node, depth)
        node.children.empty? ? "" : emit_node(node.children[0], depth)
      end

      # class_selector = DOT IDENT ;
      def emit_class_selector(node, _depth)
        parts = node.children.filter_map do |c|
          c.respond_to?(:rule_name) ? nil : c.value
        end
        parts.join
      end

      # id_selector = HASH ;
      def emit_id_selector(node, _depth)
        node.children.empty? ? "" : node.children[0].value
      end

      # attribute_selector = LBRACKET IDENT [ attr_matcher attr_value ] RBRACKET ;
      def emit_attribute_selector(node, depth)
        parts = []
        node.children.each do |child|
          parts << if child.respond_to?(:rule_name)
            emit_node(child, depth)
          else
            case token_type(child)
            when "LBRACKET" then "["
            when "RBRACKET" then "]"
            else child.value
            end
          end
        end
        parts.join
      end

      def emit_attr_matcher(node, _depth)
        node.children.empty? ? "" : node.children[0].value
      end

      def emit_attr_value(node, _depth)
        return "" if node.children.empty?

        child = node.children[0]
        (token_type(child) == "STRING") ? "\"#{child.value}\"" : child.value
      end

      # pseudo_class = COLON FUNCTION pseudo_class_args RPAREN | COLON IDENT ;
      def emit_pseudo_class(node, depth)
        parts = []
        node.children.each do |child|
          parts << if child.respond_to?(:rule_name)
            emit_node(child, depth)
          else
            case token_type(child)
            when "COLON" then ":"
            when "RPAREN" then ")"
            else child.value
            end
          end
        end
        parts.join
      end

      def emit_pseudo_class_args(node, depth)
        parts = node.children.map { |c| emit_node(c, depth) }
        parts.join
      end

      def emit_pseudo_class_arg(node, depth)
        emit_default(node, depth)
      end

      # pseudo_element = COLON_COLON IDENT ;
      def emit_pseudo_element(node, _depth)
        parts = []
        node.children.each do |child|
          parts << ((token_type(child) == "COLON_COLON") ? "::" : child.value)
        end
        parts.join
      end

      # ============================================================
      # Blocks and Declarations
      # ============================================================

      # block = LBRACE block_contents RBRACE ;
      def emit_block(node, depth)
        contents = node.children.find do |c|
          c.respond_to?(:rule_name) && c.rule_name == "block_contents"
        end

        if @minified
          return "{}" unless contents

          inner = emit_block_contents(contents, depth + 1)
          "{#{inner}}"
        else
          unless contents
            return "{\n#{@indent * depth}}"
          end

          inner = emit_block_contents(contents, depth + 1)
          return "{\n#{@indent * depth}}" if inner.strip.empty?

          "{\n#{inner}\n#{@indent * depth}}"
        end
      end

      # block_contents = { block_item } ;
      def emit_block_contents(node, depth)
        parts = []
        node.children.each do |child|
          text = emit_node(child, depth)
          parts << text if text.strip.length.positive?
        end

        return parts.join if @minified

        prefix = @indent * depth
        parts.map { |p| "#{prefix}#{p}" }.join("\n")
      end

      # block_item = lattice_block_item | at_rule | declaration_or_nested ;
      def emit_block_item(node, depth)
        node.children.empty? ? "" : emit_node(node.children[0], depth)
      end

      def emit_declaration_or_nested(node, depth)
        node.children.empty? ? "" : emit_node(node.children[0], depth)
      end

      # declaration = property COLON value_list [ priority ] SEMICOLON ;
      def emit_declaration(node, depth)
        prop = ""
        value = ""
        priority = ""

        node.children.each do |child|
          next unless child.respond_to?(:rule_name)

          case child.rule_name
          when "property" then prop = emit_property(child, depth)
          when "value_list" then value = emit_value_list(child, depth)
          when "priority" then priority = " !important"
          end
        end

        @minified ? "#{prop}:#{value}#{priority};" : "#{prop}: #{value}#{priority};"
      end

      # property = IDENT | CUSTOM_PROPERTY ;
      def emit_property(node, _depth)
        node.children.empty? ? "" : node.children[0].value
      end

      def emit_priority(_node, _depth)
        "!important"
      end

      # ============================================================
      # Values
      # ============================================================

      # value_list = value { value } ;
      # Space-separate values, but commas don't need extra spaces.
      def emit_value_list(node, depth)
        parts = node.children.map { |c| emit_node(c, depth) }
        result = parts.join(" ")
        # Clean up space around commas: "red , blue" -> "red, blue"
        result.gsub(" , ", ", ").gsub(" ,", ",")
      end

      # value = DIMENSION | PERCENTAGE | NUMBER | STRING | IDENT | HASH | ... ;
      def emit_value(node, depth)
        children = node.children
        if children.size == 1
          child = children[0]
          if child.respond_to?(:rule_name)
            return emit_node(child, depth)
          end

          return "\"#{child.value}\"" if token_type(child) == "STRING"

          return child.value
        end
        emit_default(node, depth)
      end

      # function_call = FUNCTION function_args RPAREN | URL_TOKEN ;
      def emit_function_call(node, depth)
        children = node.children

        # URL_TOKEN is a single token.
        return children[0].value if children.size == 1 && !children[0].respond_to?(:rule_name)

        parts = []
        children.each do |child|
          parts << if child.respond_to?(:rule_name)
            emit_node(child, depth)
          else
            case token_type(child)
            when "FUNCTION" then child.value  # includes "("
            when "RPAREN" then ")"
            else child.value
            end
          end
        end
        parts.join
      end

      # function_args = { function_arg } ;
      def emit_function_args(node, depth)
        parts = node.children.map { |c| emit_node(c, depth) }
        result = parts.join(" ")
        result.gsub(" , ", ", ").gsub(" ,", ",")
      end

      # function_arg = single argument.
      #
      # When function_arg has multiple children (nested function call):
      #   FUNCTION token (includes "(") + function_args node + RPAREN token
      # These must be joined with "" (no spaces) so "rgb(255, 0, 0)" doesn't
      # become "rgb( 255, 0, 0 )".
      def emit_function_arg(node, depth)
        children = node.children
        if children.size == 1
          child = children[0]
          return child.respond_to?(:rule_name) ? emit_node(child, depth) : child.value
        end

        # Multi-child: nested function call — join without spaces.
        # FUNCTION token value already includes "(" (e.g., "rgb(").
        # RPAREN token emits ")".
        # Other children (function_args ASTNode) recurse normally.
        parts = children.map do |child|
          if child.respond_to?(:rule_name)
            emit_node(child, depth)
          else
            case token_type(child)
            when "FUNCTION" then child.value
            when "RPAREN" then ")"
            else child.value
            end
          end
        end
        parts.join("")
      end

      # ============================================================
      # Default and Utilities
      # ============================================================

      # Default handler: concatenate children with spaces.
      def emit_default(node, depth)
        parts = node.children.map { |c| emit_node(c, depth) }
        parts.join(" ")
      end

      def token_type(token)
        t = token.type
        t.respond_to?(:name) ? t.name : t.to_s
      end
    end
  end
end
