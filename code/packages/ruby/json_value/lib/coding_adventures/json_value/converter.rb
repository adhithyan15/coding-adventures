# frozen_string_literal: true

# ================================================================
# JsonValue Converter -- AST <-> JsonValue <-> Native Types
# ================================================================
#
# This module provides three conversion functions that form the
# bridge between the parser's generic AST and the typed JsonValue
# representation, and between JsonValue and Ruby's native types.
#
# The conversion pipeline looks like this:
#
#   JSON text
#     |  (json_parser)
#     v
#   ASTNode tree         -- generic, rule-name-based
#     |  (from_ast)
#     v
#   JsonValue tree       -- typed, JSON-semantic
#     |  (to_native)
#     v
#   Ruby Hash/Array/etc. -- native types
#
# And the reverse:
#
#   Ruby Hash/Array/etc.
#     |  (from_native)
#     v
#   JsonValue tree
#
# ================================================================
# Algorithm: from_ast
# ================================================================
#
# The AST produced by json-parser has this structure:
#
#   ASTNode(rule_name: "value", children: [
#     ASTNode(rule_name: "object", children: [
#       Token(LBRACE), ASTNode(rule_name: "pair", ...), Token(RBRACE)
#     ])
#   ])
#
# Each ASTNode records which grammar rule produced it. Leaf nodes
# are Token objects from the lexer. The conversion is a recursive
# tree walk that dispatches on rule_name:
#
#   "value"  -> unwrap: find the meaningful child and recurse
#   "object" -> collect pairs into JsonValue::Object
#   "pair"   -> extract key (STRING token) and value (recurse)
#   "array"  -> collect elements into JsonValue::Array
#
# For leaf Token nodes, we dispatch on token type:
#
#   STRING -> JsonValue::String (value already unescaped by lexer)
#   NUMBER -> JsonValue::Number (int if no decimal/exponent, else float)
#   TRUE   -> JsonValue::Boolean(true)
#   FALSE  -> JsonValue::Boolean(false)
#   NULL   -> JsonValue::Null
#
# ================================================================

module CodingAdventures
  module JsonValue
    # Token types that carry JSON values. Structural tokens like
    # LBRACE, RBRACE, COMMA, COLON are skipped during conversion.
    VALUE_TOKEN_TYPES = %w[STRING NUMBER TRUE FALSE NULL].freeze

    # ----------------------------------------------------------------
    # from_ast: ASTNode -> JsonValue
    # ----------------------------------------------------------------
    #
    # Converts a json-parser AST node into a typed JsonValue tree.
    #
    # @param node [CodingAdventures::Parser::ASTNode, CodingAdventures::Lexer::Token]
    # @return [JsonValue::Object, JsonValue::Array, JsonValue::String,
    #          JsonValue::Number, JsonValue::Boolean, JsonValue::Null]
    # @raise [Error] if the AST has an unexpected structure
    # ----------------------------------------------------------------
    def self.from_ast(node)
      # Case 1: The node is a Token (leaf node from the lexer).
      # Tokens carry the actual values: strings, numbers, booleans, null.
      if node.is_a?(CodingAdventures::Lexer::Token)
        return convert_token(node)
      end

      # Case 2: The node is an ASTNode (internal node from the parser).
      # Dispatch based on which grammar rule produced this node.
      case node.rule_name
      when "value"
        convert_value_node(node)
      when "object"
        convert_object_node(node)
      when "pair"
        convert_pair_node(node)
      when "array"
        convert_array_node(node)
      else
        raise Error, "Unexpected AST rule: #{node.rule_name}"
      end
    end

    # ----------------------------------------------------------------
    # to_native: JsonValue -> Ruby native types
    # ----------------------------------------------------------------
    #
    # Recursively converts a JsonValue tree into plain Ruby types:
    #
    #   JsonValue::Object  -> Hash (preserving insertion order)
    #   JsonValue::Array   -> Array
    #   JsonValue::String  -> String
    #   JsonValue::Number  -> Integer or Float
    #   JsonValue::Boolean -> true or false
    #   JsonValue::Null    -> nil
    #
    # @param json_val [JsonValue type] a JsonValue instance
    # @return [Hash, ::Array, ::String, Integer, Float, true, false, nil]
    # ----------------------------------------------------------------
    def self.to_native(json_val)
      case json_val
      when Object
        result = {}
        json_val.pairs.each { |key, val| result[key] = to_native(val) }
        result
      when Array
        json_val.elements.map { |elem| to_native(elem) }
      when String
        json_val.value
      when Number
        json_val.value
      when Boolean
        json_val.value
      when Null
        nil
      else
        raise Error, "Unknown JsonValue type: #{json_val.class}"
      end
    end

    # ----------------------------------------------------------------
    # from_native: Ruby native types -> JsonValue
    # ----------------------------------------------------------------
    #
    # Converts plain Ruby types into a JsonValue tree. This is the
    # inverse of to_native.
    #
    # Supported types:
    #   Hash       -> JsonValue::Object (keys must be strings)
    #   ::Array    -> JsonValue::Array
    #   ::String   -> JsonValue::String
    #   Integer    -> JsonValue::Number
    #   Float      -> JsonValue::Number
    #   true/false -> JsonValue::Boolean
    #   nil        -> JsonValue::Null
    #
    # @param value [Hash, ::Array, ::String, Integer, Float, true, false, nil]
    # @return [JsonValue type]
    # @raise [Error] if value contains non-JSON-compatible types
    # ----------------------------------------------------------------
    def self.from_native(value)
      case value
      when ::Hash
        pairs = {}
        value.each do |k, v|
          unless k.is_a?(::String)
            raise Error, "JSON object keys must be strings, got #{k.class}: #{k.inspect}"
          end
          pairs[k] = from_native(v)
        end
        Object.new(pairs: pairs)
      when ::Array
        Array.new(elements: value.map { |elem| from_native(elem) })
      when ::String
        String.new(value: value)
      when ::Integer
        Number.new(value: value)
      when ::Float
        Number.new(value: value)
      when true, false
        Boolean.new(value: value)
      when nil
        Null.new
      else
        raise Error, "Cannot convert #{value.class} to JsonValue: #{value.inspect}"
      end
    end

    # ----------------------------------------------------------------
    # parse: JSON text -> JsonValue
    # ----------------------------------------------------------------
    #
    # Convenience method that combines lexing, parsing, and AST
    # conversion into a single call.
    #
    # @param text [::String] JSON text
    # @return [JsonValue type]
    # @raise [Error] if the text is not valid JSON
    # ----------------------------------------------------------------
    def self.parse(text)
      ast = CodingAdventures::JsonParser.parse(text)
      from_ast(ast)
    rescue CodingAdventures::Parser::GrammarParseError => e
      raise Error, "Failed to parse JSON: #{e.message}"
    rescue CodingAdventures::Lexer::LexerError => e
      raise Error, "Failed to parse JSON: #{e.message}"
    end

    # ----------------------------------------------------------------
    # parse_native: JSON text -> Ruby native types
    # ----------------------------------------------------------------
    #
    # Convenience method that parses JSON text directly into native
    # Ruby types. Equivalent to: to_native(parse(text))
    #
    # This is the most common use case -- "give me a Hash from this
    # JSON string."
    #
    # @param text [::String] JSON text
    # @return [Hash, ::Array, ::String, Integer, Float, true, false, nil]
    # @raise [Error] if the text is not valid JSON
    # ----------------------------------------------------------------
    def self.parse_native(text)
      to_native(parse(text))
    end

    # ==============================================================
    # Private Helpers
    # ==============================================================

    # Convert a Token (leaf node) to a JsonValue.
    #
    # The token's type tells us what kind of JSON value it is:
    #
    #   STRING -> JsonValue::String
    #     The value is already unescaped by the lexer, so "\n" in
    #     the JSON source is stored as an actual newline character.
    #
    #   NUMBER -> JsonValue::Number
    #     We inspect the string representation to decide integer vs
    #     float: if it contains a decimal point or exponent, it's a
    #     float. Otherwise it's an integer.
    #
    #   TRUE/FALSE -> JsonValue::Boolean
    #   NULL -> JsonValue::Null
    #
    # Structural tokens (LBRACE, RBRACE, COMMA, COLON, etc.) return
    # nil -- they have no semantic value.
    def self.convert_token(token)
      type_name = token.type.to_s

      case type_name
      when "STRING"
        String.new(value: token.value)
      when "NUMBER"
        # Determine if the number is an integer or float by examining
        # the string representation. JSON spec says:
        #   - No decimal point and no exponent -> integer
        #   - Has decimal point or exponent -> float
        str = token.value
        if str.include?(".") || str.include?("e") || str.include?("E")
          Number.new(value: Float(str))
        else
          Number.new(value: Integer(str))
        end
      when "TRUE"
        Boolean.new(value: true)
      when "FALSE"
        Boolean.new(value: false)
      when "NULL"
        Null.new
      else
        # Structural tokens (LBRACE, RBRACE, etc.) -- skip
        nil
      end
    end

    # Convert a "value" rule node.
    #
    # The "value" rule wraps exactly one meaningful child:
    #   value = object | array | STRING | NUMBER | TRUE | FALSE | NULL
    #
    # We scan the children to find the first one that is either:
    # - An ASTNode (with rule_name "object" or "array")
    # - A Token with a value type (STRING, NUMBER, TRUE, FALSE, NULL)
    def self.convert_value_node(node)
      node.children.each do |child|
        if child.is_a?(CodingAdventures::Parser::ASTNode)
          return from_ast(child)
        end

        if child.is_a?(CodingAdventures::Lexer::Token)
          type_name = child.type.to_s
          if VALUE_TOKEN_TYPES.include?(type_name)
            return convert_token(child)
          end
        end
      end

      raise Error, "No meaningful child found in 'value' node"
    end

    # Convert an "object" rule node to JsonValue::Object.
    #
    # The object grammar rule is:
    #   object = LBRACE [ pair { COMMA pair } ] RBRACE
    #
    # We skip structural tokens (LBRACE, RBRACE, COMMA) and process
    # only the "pair" sub-nodes. Each pair gives us a key-value pair.
    def self.convert_object_node(node)
      pairs = {}

      node.children.each do |child|
        next unless child.is_a?(CodingAdventures::Parser::ASTNode)
        next unless child.rule_name == "pair"

        key, val = convert_pair_node(child)
        pairs[key] = val
      end

      Object.new(pairs: pairs)
    end

    # Convert a "pair" rule node to a [key, value] tuple.
    #
    # The pair grammar rule is:
    #   pair = STRING COLON value
    #
    # We find the STRING token (the key) and the "value" ASTNode
    # (the value), then recurse on the value.
    #
    # @return [::Array(::String, JsonValue)] a two-element array [key, value]
    def self.convert_pair_node(node)
      key = nil
      val = nil

      node.children.each do |child|
        if child.is_a?(CodingAdventures::Lexer::Token) && child.type.to_s == "STRING"
          key = child.value
        elsif child.is_a?(CodingAdventures::Parser::ASTNode) && child.rule_name == "value"
          val = from_ast(child)
        end
      end

      raise Error, "Malformed pair node: missing key or value" if key.nil? || val.nil?

      [key, val]
    end

    # Convert an "array" rule node to JsonValue::Array.
    #
    # The array grammar rule is:
    #   array = LBRACKET [ value { COMMA value } ] RBRACKET
    #
    # We skip structural tokens and process only the "value" sub-nodes.
    # Each one becomes an element of the JsonValue::Array.
    def self.convert_array_node(node)
      elements = []

      node.children.each do |child|
        if child.is_a?(CodingAdventures::Parser::ASTNode) && child.rule_name == "value"
          elements << from_ast(child)
        elsif child.is_a?(CodingAdventures::Lexer::Token)
          type_name = child.type.to_s
          if VALUE_TOKEN_TYPES.include?(type_name)
            result = convert_token(child)
            elements << result unless result.nil?
          end
        end
      end

      Array.new(elements: elements)
    end

    private_class_method :convert_token, :convert_value_node,
                         :convert_object_node, :convert_pair_node,
                         :convert_array_node
  end
end
