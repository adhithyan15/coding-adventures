# frozen_string_literal: true

# ================================================================
# Mosaic Analyzer -- Walks the AST and Produces a Typed MosaicIR
# ================================================================
#
# The analyzer is the third stage of the Mosaic compiler pipeline:
#
#   Source text → Lexer → Parser → **Analyzer** → MosaicIR → VM → Backend
#
# The analyzer:
#   1. Strips syntax noise (keywords, semicolons, braces)
#   2. Resolves type keywords → MosaicType hashes
#   3. Normalizes values (dimensions, colors, booleans)
#   4. Determines required/optional slots (no default → required)
#   5. Identifies primitive vs component nodes
#
# Primitive node set (built-in layout/display elements):
#   Row, Column, Box, Stack, Text, Image, Icon, Spacer, Divider, Scroll
#
# This analyzer is permissive by design — it does not enforce that
# property names are known. Stricter validation can be layered on top.
# ================================================================

require "coding_adventures_mosaic_parser"

module CodingAdventures
  module MosaicAnalyzer
    # ----------------------------------------------------------------
    # Error class for analysis failures
    # ----------------------------------------------------------------

    # Raised when the analyzer encounters a structural problem in the AST.
    # These indicate a bug in the parser or a malformed AST.
    class AnalysisError < StandardError
      def initialize(message)
        super("AnalysisError: #{message}")
      end
    end

    # ----------------------------------------------------------------
    # Primitive node registry
    # ----------------------------------------------------------------

    # Built-in layout and display elements. Nodes with these tag names
    # have `is_primitive: true`; all other names are component types.
    PRIMITIVE_NODES = %w[Row Column Box Stack Text Image Icon Spacer Divider Scroll].to_set

    # ----------------------------------------------------------------
    # Public API
    # ----------------------------------------------------------------

    # Analyze Mosaic source text and return a typed MosaicIR.
    #
    # @param source [String] the .mosaic source text
    # @return [MosaicIR] the typed intermediate representation
    # @raise [AnalysisError] if the AST is structurally invalid
    def self.analyze(source)
      ast = CodingAdventures::MosaicParser.parse(source)
      analyze_file(ast)
    end

    # Analyze a pre-parsed ASTNode and return a typed MosaicIR.
    # Use this when you already have an AST and want to skip re-parsing.
    #
    # @param ast [ASTNode] the root ASTNode (rule_name must be "file")
    # @return [MosaicIR] the typed intermediate representation
    def self.analyze_ast(ast)
      analyze_file(ast)
    end

    # ----------------------------------------------------------------
    # File-level analysis
    # ----------------------------------------------------------------

    def self.analyze_file(ast)
      unless ast.rule_name == "file"
        raise AnalysisError, "Expected root rule 'file', got '#{ast.rule_name}'"
      end

      imports = []
      component_decl = nil

      ast.children.each do |child|
        next unless child.respond_to?(:rule_name)
        case child.rule_name
        when "import_decl"
          imports << analyze_import(child)
        when "component_decl"
          component_decl = child
        end
      end

      raise AnalysisError, "No component declaration found in file" unless component_decl

      component = analyze_component(component_decl)
      MosaicIR.new(component: component, imports: imports)
    end
    private_class_method :analyze_file

    # ----------------------------------------------------------------
    # Import analysis
    # ----------------------------------------------------------------

    # import_decl = KEYWORD NAME [ KEYWORD NAME ] KEYWORD STRING SEMICOLON
    # "import" NAME ["as" NAME] "from" STRING ";"
    def self.analyze_import(node)
      names  = token_values(node, "NAME")
      strings = token_values(node, "STRING")

      raise AnalysisError, "import_decl missing component name" if names.empty?
      raise AnalysisError, "import_decl missing path string" if strings.empty?

      component_name = names[0]
      # If there's a second NAME, it's the alias (import X as Y from ...)
      import_alias = names.length >= 2 ? names[1] : nil
      path = strings[0]

      MosaicImport.new(component_name: component_name, alias: import_alias, path: path)
    end
    private_class_method :analyze_import

    # ----------------------------------------------------------------
    # Component analysis
    # ----------------------------------------------------------------

    # component_decl = KEYWORD NAME LBRACE { slot_decl } node_tree RBRACE
    def self.analyze_component(node)
      names = token_values(node, "NAME")
      raise AnalysisError, "component_decl missing name" if names.empty?

      name = names[0]
      slots = []
      tree_node = nil

      node.children.each do |child|
        next unless child.respond_to?(:rule_name)
        case child.rule_name
        when "slot_decl"
          slots << analyze_slot(child)
        when "node_tree"
          tree_node = child
        end
      end

      raise AnalysisError, "component '#{name}' has no node tree" unless tree_node

      tree = analyze_node_tree(tree_node)
      MosaicComponent.new(name: name, slots: slots, tree: tree)
    end
    private_class_method :analyze_component

    # ----------------------------------------------------------------
    # Slot analysis
    # ----------------------------------------------------------------

    # slot_decl = KEYWORD NAME COLON slot_type [ EQUALS default_value ] SEMICOLON
    def self.analyze_slot(node)
      names = token_values(node, "NAME")
      raise AnalysisError, "slot_decl missing name" if names.empty?

      name = names[0]
      slot_type_node = find_child(node, "slot_type")
      raise AnalysisError, "slot '#{name}' missing type" unless slot_type_node

      type = analyze_slot_type(slot_type_node)
      default_val_node = find_child(node, "default_value")
      default_value = default_val_node ? analyze_default_value(default_val_node) : nil
      required = default_value.nil?

      MosaicSlot.new(name: name, type: type, default_value: default_value, required: required)
    end
    private_class_method :analyze_slot

    # slot_type = KEYWORD | NAME | list_type
    def self.analyze_slot_type(node)
      list_node = find_child(node, "list_type")
      return analyze_list_type(list_node) if list_node

      keyword = first_token_value(node, "KEYWORD")
      return parse_primitive_type(keyword) if keyword

      name = first_token_value(node, "NAME")
      return { kind: "component", name: name } if name

      raise AnalysisError, "slot_type has no recognizable content"
    end
    private_class_method :analyze_slot_type

    # list_type = KEYWORD LANGLE slot_type RANGLE
    def self.analyze_list_type(node)
      element_type_node = find_child(node, "slot_type")
      raise AnalysisError, "list_type missing element type" unless element_type_node

      element_type = analyze_slot_type(element_type_node)
      { kind: "list", element_type: element_type }
    end
    private_class_method :analyze_list_type

    # Map keyword strings to MosaicType hashes
    def self.parse_primitive_type(keyword)
      case keyword
      when "text"   then { kind: "text" }
      when "number" then { kind: "number" }
      when "bool"   then { kind: "bool" }
      when "image"  then { kind: "image" }
      when "color"  then { kind: "color" }
      when "node"   then { kind: "node" }
      else
        raise AnalysisError, "Unknown primitive type keyword: '#{keyword}'"
      end
    end
    private_class_method :parse_primitive_type

    # default_value = STRING | NUMBER | DIMENSION | COLOR_HEX | KEYWORD
    def self.analyze_default_value(node)
      str = first_token_value(node, "STRING")
      return { kind: "string", value: str } if str

      dim = first_token_value(node, "DIMENSION")
      return parse_dimension(dim) if dim

      num = first_token_value(node, "NUMBER")
      return { kind: "number", value: num.to_f } if num

      color = first_token_value(node, "COLOR_HEX")
      return { kind: "color_hex", value: color } if color

      kw = first_token_value(node, "KEYWORD")
      if kw == "true"  then return { kind: "bool", value: true }
      elsif kw == "false" then return { kind: "bool", value: false }
      end

      raise AnalysisError, "default_value has no recognizable content"
    end
    private_class_method :analyze_default_value

    # ----------------------------------------------------------------
    # Node tree analysis
    # ----------------------------------------------------------------

    # node_tree = node_element
    def self.analyze_node_tree(node)
      element = find_child(node, "node_element")
      raise AnalysisError, "node_tree missing node_element" unless element

      analyze_node_element(element)
    end
    private_class_method :analyze_node_tree

    # node_element = NAME LBRACE { node_content } RBRACE
    def self.analyze_node_element(node)
      tag = first_token_value(node, "NAME")
      raise AnalysisError, "node_element missing tag name" unless tag

      is_primitive = PRIMITIVE_NODES.include?(tag)
      properties = []
      children = []

      node.children.each do |child|
        next unless child.respond_to?(:rule_name)
        next unless child.rule_name == "node_content"

        prop, child_item = analyze_node_content(child)
        properties << prop if prop
        children << child_item if child_item
      end

      MosaicNode.new(tag: tag, is_primitive: is_primitive, properties: properties, children: children)
    end
    private_class_method :analyze_node_element

    # node_content = property_assignment | child_node | slot_reference | when_block | each_block
    def self.analyze_node_content(node)
      node.children.each do |child|
        next unless child.respond_to?(:rule_name)

        case child.rule_name
        when "property_assignment"
          return [analyze_property_assignment(child), nil]
        when "child_node"
          element = find_child(child, "node_element")
          return [nil, { kind: "node", node: analyze_node_element(element) }] if element
        when "slot_reference"
          name = first_token_value(child, "NAME")
          return [nil, { kind: "slot_ref", slot_name: name }] if name
        when "when_block"
          return [nil, analyze_when_block(child)]
        when "each_block"
          return [nil, analyze_each_block(child)]
        end
      end
      [nil, nil]
    end
    private_class_method :analyze_node_content

    # ----------------------------------------------------------------
    # Property analysis
    # ----------------------------------------------------------------

    # property_assignment = (NAME | KEYWORD) COLON property_value SEMICOLON
    def self.analyze_property_assignment(node)
      # Property name can be NAME or KEYWORD (e.g., "color", "node")
      name = first_token_value(node, "NAME") || first_token_value(node, "KEYWORD")
      raise AnalysisError, "property_assignment missing name" unless name

      value_node = find_child(node, "property_value")
      raise AnalysisError, "property '#{name}' missing value" unless value_node

      value = analyze_property_value(value_node)
      MosaicProperty.new(name: name, value: value)
    end
    private_class_method :analyze_property_assignment

    # property_value = slot_ref | STRING | DIMENSION | NUMBER | COLOR_HEX | KEYWORD | enum_value | NAME
    def self.analyze_property_value(node)
      # Check child rule nodes first
      node.children.each do |child|
        next unless child.respond_to?(:rule_name)

        case child.rule_name
        when "slot_ref"
          name = first_token_value(child, "NAME")
          return { kind: "slot_ref", slot_name: name } if name
        when "enum_value"
          names = token_values(child, "NAME")
          return { kind: "enum", namespace: names[0], member: names[1] } if names.length >= 2
        end
      end

      # Leaf tokens
      str = first_token_value(node, "STRING")
      return { kind: "string", value: str } if str

      dim = first_token_value(node, "DIMENSION")
      return parse_dimension(dim) if dim

      num = first_token_value(node, "NUMBER")
      return { kind: "number", value: num.to_f } if num

      color = first_token_value(node, "COLOR_HEX")
      return { kind: "color_hex", value: color } if color

      kw = first_token_value(node, "KEYWORD")
      if kw == "true"  then return { kind: "bool", value: true }
      elsif kw == "false" then return { kind: "bool", value: false }
      elsif kw then return { kind: "ident", value: kw }
      end

      ident = first_token_value(node, "NAME")
      return { kind: "ident", value: ident } if ident

      raise AnalysisError, "property_value has no recognizable content"
    end
    private_class_method :analyze_property_value

    # ----------------------------------------------------------------
    # When / Each block analysis
    # ----------------------------------------------------------------

    # when_block = KEYWORD slot_ref LBRACE { node_content } RBRACE
    def self.analyze_when_block(node)
      slot_ref_node = find_child(node, "slot_ref")
      raise AnalysisError, "when_block missing slot_ref" unless slot_ref_node

      slot_name = first_token_value(slot_ref_node, "NAME")
      raise AnalysisError, "when_block slot_ref missing name" unless slot_name

      children = collect_node_contents(node)
      { kind: "when", slot_name: slot_name, children: children }
    end
    private_class_method :analyze_when_block

    # each_block = KEYWORD slot_ref KEYWORD NAME LBRACE { node_content } RBRACE
    def self.analyze_each_block(node)
      slot_ref_node = find_child(node, "slot_ref")
      raise AnalysisError, "each_block missing slot_ref" unless slot_ref_node

      slot_name = first_token_value(slot_ref_node, "NAME")
      raise AnalysisError, "each_block slot_ref missing name" unless slot_name

      # The loop variable is the NAME token that is a direct child of each_block
      # (not inside the slot_ref). It comes after the "as" KEYWORD.
      item_name = find_loop_variable(node, slot_ref_node)
      raise AnalysisError, "each_block missing loop variable name" unless item_name

      children = collect_node_contents(node)
      { kind: "each", slot_name: slot_name, item_name: item_name, children: children }
    end
    private_class_method :analyze_each_block

    # Find the loop variable NAME in an each_block.
    # Structure: KEYWORD(each) slot_ref KEYWORD(as) NAME(item) LBRACE ... RBRACE
    # We skip the slot_ref sub-tree and look for NAME after "as".
    def self.find_loop_variable(each_block, slot_ref_node)
      after_as = false
      each_block.children.each do |child|
        # Skip the slot_ref subtree
        next if child.respond_to?(:rule_name) && child == slot_ref_node
        next if child.respond_to?(:rule_name) && child.rule_name == "slot_ref"

        unless child.respond_to?(:rule_name)
          if child.type == "KEYWORD" && child.value == "as"
            after_as = true
            next
          end
          return child.value if after_as && child.type == "NAME"
        end
      end
      nil
    end
    private_class_method :find_loop_variable

    # Collect all node_content children from a block
    def self.collect_node_contents(node)
      children = []
      node.children.each do |child|
        next unless child.respond_to?(:rule_name)
        next unless child.rule_name == "node_content"

        _prop, child_item = analyze_node_content(child)
        children << child_item if child_item
      end
      children
    end
    private_class_method :collect_node_contents

    # ----------------------------------------------------------------
    # Value parsing helpers
    # ----------------------------------------------------------------

    # Parse a DIMENSION token like "16dp" or "100%" into a structured hash.
    # The lexer guarantees DIMENSION = number + unit suffix.
    def self.parse_dimension(raw)
      match = raw.match(/^(-?[0-9]*\.?[0-9]+)([a-zA-Z%]+)$/)
      raise AnalysisError, "Invalid DIMENSION token: '#{raw}'" unless match

      { kind: "dimension", value: match[1].to_f, unit: match[2] }
    end
    private_class_method :parse_dimension

    # ----------------------------------------------------------------
    # AST traversal helpers
    # ----------------------------------------------------------------

    # Find the first direct child ASTNode with a given rule_name.
    def self.find_child(node, rule_name)
      node.children.find { |c| c.respond_to?(:rule_name) && c.rule_name == rule_name }
    end
    private_class_method :find_child

    # Collect all direct-child token values of a given type.
    def self.token_values(node, token_type)
      node.children
        .reject { |c| c.respond_to?(:rule_name) }
        .select { |c| c.type == token_type }
        .map(&:value)
    end
    private_class_method :token_values

    # Get the first direct-child token value of a given type, or nil.
    def self.first_token_value(node, token_type)
      node.children.each do |child|
        next if child.respond_to?(:rule_name)
        return child.value if child.type == token_type
      end
      nil
    end
    private_class_method :first_token_value
  end
end
