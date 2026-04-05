# frozen_string_literal: true

# ================================================================
# MosaicIR -- Intermediate Representation for Mosaic Components
# ================================================================
#
# The MosaicIR is the output of the analyzer stage. It is a
# validated, platform-neutral data structure consumed by the VM
# and all backends (React, Web Components, SwiftUI, etc.).
#
# The pipeline is:
#   Source text → Lexer → Parser → Analyzer → **MosaicIR** → VM → Backend
#
# Unlike the AST (which mirrors source syntax), the IR:
#   - Strips syntax noise (keywords, semicolons, braces)
#   - Resolves types to typed structs
#   - Normalizes defaults (e.g., "number = 0" → MosaicValue number:0)
#   - Identifies primitive vs component nodes
#
# All IR types are Ruby Struct objects. Hashes represent discriminated
# unions (like TypeScript union types) using a `:kind` key.
#
# MosaicType (hash with :kind):
#   { kind: "text" }
#   { kind: "number" }
#   { kind: "bool" }
#   { kind: "image" }
#   { kind: "color" }
#   { kind: "node" }
#   { kind: "component", name: "Button" }
#   { kind: "list", element_type: <MosaicType> }
#
# MosaicValue (hash with :kind):
#   { kind: "string", value: "hello" }
#   { kind: "number", value: 42.0 }
#   { kind: "bool", value: true }
#   { kind: "dimension", value: 16.0, unit: "dp" }
#   { kind: "color_hex", value: "#2563eb" }
#   { kind: "ident", value: "center" }
#   { kind: "slot_ref", slot_name: "title" }
#   { kind: "enum", namespace: "heading", member: "large" }
#
# MosaicChild (hash with :kind):
#   { kind: "node", node: <MosaicNode> }
#   { kind: "slot_ref", slot_name: "header" }
#   { kind: "when", slot_name: "show", children: [...] }
#   { kind: "each", slot_name: "items", item_name: "item", children: [...] }
# ================================================================

module CodingAdventures
  module MosaicAnalyzer
    # The top-level IR container: one component + its imports.
    MosaicIR = Struct.new(:component, :imports, keyword_init: true)

    # A Mosaic component declaration.
    #   name:    String (e.g., "ProfileCard")
    #   slots:   Array<MosaicSlot>
    #   tree:    MosaicNode (root of the visual tree)
    MosaicComponent = Struct.new(:name, :slots, :tree, keyword_init: true)

    # An import declaration: `import Button from "./button.mosaic"`
    #   component_name: String ("Button")
    #   alias:          String or nil ("InfoCard" if `import Button as InfoCard`)
    #   path:           String ("./button.mosaic")
    MosaicImport = Struct.new(:component_name, :alias, :path, keyword_init: true)

    # A typed slot declaration.
    #   name:          String (kebab-case, e.g., "avatar-url")
    #   type:          Hash (MosaicType)
    #   default_value: Hash or nil (MosaicValue)
    #   required:      Boolean (true when no default_value)
    MosaicSlot = Struct.new(:name, :type, :default_value, :required, keyword_init: true)

    # A visual node in the component tree.
    #   tag:         String (e.g., "Row", "Button")
    #   is_primitive: Boolean (true for Row, Column, Text, etc.)
    #   properties:  Array<MosaicProperty>
    #   children:    Array<MosaicChild hash>
    MosaicNode = Struct.new(:tag, :is_primitive, :properties, :children, keyword_init: true)

    # A property assignment on a node: `padding: 16dp`
    #   name:  String (e.g., "padding", "background")
    #   value: Hash (MosaicValue)
    MosaicProperty = Struct.new(:name, :value, keyword_init: true)
  end
end
