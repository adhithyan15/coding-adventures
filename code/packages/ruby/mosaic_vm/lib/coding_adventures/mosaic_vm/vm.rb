# frozen_string_literal: true

# ================================================================
# MosaicVM -- Generic Tree-Walking Driver for Mosaic Backends
# ================================================================
#
# The VM is the fourth stage of the Mosaic compiler pipeline:
#
#   Source text → Lexer → Parser → Analyzer → MosaicIR → **VM** → Backend
#
# The VM's responsibilities:
#   1. Traverse the MosaicIR tree depth-first.
#   2. Normalize every MosaicValue into a ResolvedValue (hex → RGBA,
#      dimensions parsed, idents → strings, slot_refs enriched).
#   3. Track the SlotContext (component slots + active each-loop scopes).
#   4. Call MosaicRenderer methods in strict open-before-close order.
#
# The VM is agnostic about output format. It has no knowledge of React,
# Web Components, SwiftUI, or any platform. Backends own the output —
# the VM only drives the traversal and normalizes values.
#
# Traversal order (depth-first, open/close):
#
#   begin_component(name, slots)
#     begin_node(tag, is_primitive, resolved_props, ctx)
#       begin_node(child_tag, ...) ... end_node(child_tag)
#       render_slot_child(slot_name, slot_type, ctx)
#       begin_when(slot_name, ctx)
#         [when children]
#       end_when
#       begin_each(slot_name, item_name, element_type, ctx)
#         [each children with loop scope pushed]
#       end_each
#     end_node(tag)
#   end_component
#   emit → { files: [...] }
#
# Color Expansion:
#   #rgb      → r=(r*17), g=(g*17), b=(b*17), a=255
#   #rrggbb   → r, g, b parsed, a=255
#   #rrggbbaa → r, g, b, a all parsed
#
# ================================================================

require "coding_adventures_mosaic_analyzer"

module CodingAdventures
  module MosaicVm
    # ----------------------------------------------------------------
    # Error
    # ----------------------------------------------------------------

    # Raised when the VM encounters a runtime invariant violation.
    # These are "should not happen" errors — the analyzer should have
    # caught undefined slot references and type mismatches earlier.
    class MosaicVMError < StandardError
      def initialize(msg)
        super("MosaicVMError: #{msg}")
      end
    end

    # ----------------------------------------------------------------
    # MosaicVM
    # ----------------------------------------------------------------

    # The generic tree-walking driver for Mosaic compiler backends.
    #
    # Construct with a MosaicIR, then call run(renderer) with any
    # backend that responds to the MosaicRenderer protocol. Returns
    # the emit result (a hash with `:files`).
    #
    # A single MosaicVM instance can be run against multiple renderers.
    # The VM is stateless between run() calls.
    #
    # @example
    #   ir = CodingAdventures::MosaicAnalyzer.analyze(source)
    #   vm = CodingAdventures::MosaicVm::MosaicVM.new(ir)
    #   result = vm.run(CodingAdventures::MosaicEmitReact::ReactRenderer.new)
    #   # => { files: [{ filename: "MyComponent.tsx", content: "..." }] }
    class MosaicVM
      def initialize(ir)
        @ir = ir
      end

      # Traverse the IR tree, calling renderer methods in depth-first order.
      #
      # @param renderer [Object] a backend implementing the MosaicRenderer protocol
      # @return [Hash] the emit result from renderer.emit
      def run(renderer)
        context = {
          component_slots: @ir.component.slots.each_with_object({}) { |s, h| h[s.name] = s },
          loop_scopes: []
        }

        renderer.begin_component(@ir.component.name, @ir.component.slots)
        walk_node(@ir.component.tree, context, renderer)
        renderer.end_component
        renderer.emit
      end

      private

      # Traverse a single node: resolve properties, call begin_node,
      # walk children, call end_node.
      def walk_node(node, ctx, renderer)
        resolved = node.properties.map do |prop|
          { name: prop.name, value: resolve_value(prop.value, ctx) }
        end

        renderer.begin_node(node.tag, node.is_primitive, resolved, ctx)

        node.children.each do |child|
          walk_child(child, ctx, renderer)
        end

        renderer.end_node(node.tag)
      end

      # Dispatch a single child to the appropriate renderer method.
      def walk_child(child, ctx, renderer)
        case child[:kind]
        when "node"
          walk_node(child[:node], ctx, renderer)

        when "slot_ref"
          slot = resolve_slot(child[:slot_name], ctx)
          renderer.render_slot_child(child[:slot_name], slot.type, ctx)

        when "when"
          renderer.begin_when(child[:slot_name], ctx)
          child[:children].each { |c| walk_child(c, ctx, renderer) }
          renderer.end_when

        when "each"
          list_slot = ctx[:component_slots][child[:slot_name]]
          raise MosaicVMError, "Unknown list slot: @#{child[:slot_name]}" unless list_slot
          raise MosaicVMError, "each block @#{child[:slot_name]} is not a list type" unless list_slot.type[:kind] == "list"

          element_type = list_slot.type[:element_type]
          renderer.begin_each(child[:slot_name], child[:item_name], element_type, ctx)

          inner_ctx = {
            component_slots: ctx[:component_slots],
            loop_scopes: ctx[:loop_scopes] + [{ item_name: child[:item_name], element_type: element_type }]
          }

          child[:children].each { |c| walk_child(c, inner_ctx, renderer) }
          renderer.end_each
        end
      end

      # Normalize a MosaicValue into a ResolvedValue hash.
      #
      # Transformations:
      #   color_hex  → { kind: "color", r:, g:, b:, a: } (parsed RGBA)
      #   dimension  → { kind: "dimension", value:, unit: } (passthrough, already structured)
      #   ident      → { kind: "string", value: } (folded)
      #   slot_ref   → enriched with slot_type and is_loop_var
      #   all others → unchanged
      def resolve_value(val, ctx)
        case val[:kind]
        when "string"    then { kind: "string",    value: val[:value] }
        when "number"    then { kind: "number",    value: val[:value] }
        when "bool"      then { kind: "bool",      value: val[:value] }
        when "ident"     then { kind: "string",    value: val[:value] }
        when "dimension" then { kind: "dimension", value: val[:value], unit: val[:unit] }
        when "color_hex" then parse_color(val[:value])
        when "enum"      then { kind: "enum", namespace: val[:namespace], member: val[:member] }
        when "slot_ref"  then resolve_slot_ref(val[:slot_name], ctx)
        else
          raise MosaicVMError, "Unknown MosaicValue kind: #{val[:kind]}"
        end
      end

      # Parse a hex color string into RGBA integers.
      #
      # Expansion rules:
      #   #rgb      → r=(r*17), g=(g*17), b=(b*17), a=255
      #   #rrggbb   → r, g, b parsed, a=255
      #   #rrggbbaa → r, g, b, a all parsed
      def parse_color(hex)
        h = hex[1..] # strip leading '#'
        r, g, b, a = case h.length
                     when 3
                       [h[0].hex * 17, h[1].hex * 17, h[2].hex * 17, 255]
                     when 6
                       [h[0, 2].to_i(16), h[2, 2].to_i(16), h[4, 2].to_i(16), 255]
                     when 8
                       [h[0, 2].to_i(16), h[2, 2].to_i(16), h[4, 2].to_i(16), h[6, 2].to_i(16)]
                     else
                       raise MosaicVMError, "Invalid color hex: #{hex}"
                     end
        { kind: "color", r: r, g: g, b: b, a: a }
      end

      # Resolve a slot reference value, enriching it with type and loop info.
      # Checks loop scopes innermost-first, then component slots.
      def resolve_slot_ref(slot_name, ctx)
        # 1. Check active loop scopes innermost-first
        ctx[:loop_scopes].reverse_each do |scope|
          if scope[:item_name] == slot_name
            return {
              kind: "slot_ref",
              slot_name: slot_name,
              slot_type: scope[:element_type],
              is_loop_var: true
            }
          end
        end

        # 2. Fall back to component slots
        slot = ctx[:component_slots][slot_name]
        raise MosaicVMError, "Unresolved slot reference: @#{slot_name}" unless slot

        { kind: "slot_ref", slot_name: slot_name, slot_type: slot.type, is_loop_var: false }
      end

      # Look up a named slot for render_slot_child (slot ref used as child, not value).
      def resolve_slot(slot_name, ctx)
        slot = ctx[:component_slots][slot_name]
        raise MosaicVMError, "Unknown slot: @#{slot_name}" unless slot

        slot
      end
    end

    # ----------------------------------------------------------------
    # MosaicRenderer mixin (protocol definition)
    # ----------------------------------------------------------------

    # Mixin that defines the MosaicRenderer protocol.
    # Include in a backend class and implement all methods.
    #
    # All methods raise NotImplementedError by default, reminding
    # implementors what they need to implement.
    module MosaicRenderer
      # Called once before tree traversal.
      # @param name [String] the component name
      # @param slots [Array<MosaicSlot>] the component's slot declarations
      def begin_component(name, slots)
        raise NotImplementedError, "#{self.class}#begin_component not implemented"
      end

      # Called once after tree traversal.
      def end_component
        raise NotImplementedError, "#{self.class}#end_component not implemented"
      end

      # Called on entering each node.
      # @param tag [String] the node tag name
      # @param is_primitive [Boolean] whether it's a built-in element
      # @param properties [Array<Hash>] resolved properties: [{name:, value:}]
      # @param ctx [Hash] the slot context
      def begin_node(tag, is_primitive, properties, ctx)
        raise NotImplementedError, "#{self.class}#begin_node not implemented"
      end

      # Called on leaving each node.
      # @param tag [String] the node tag name
      def end_node(tag)
        raise NotImplementedError, "#{self.class}#end_node not implemented"
      end

      # Called for @slot; children (slot ref used as a child, not a value).
      # @param slot_name [String] the slot name
      # @param slot_type [Hash] the slot's MosaicType
      # @param ctx [Hash] the slot context
      def render_slot_child(slot_name, slot_type, ctx)
        raise NotImplementedError, "#{self.class}#render_slot_child not implemented"
      end

      # Called on entering a when @flag { ... } block.
      # @param slot_name [String] the boolean slot name
      # @param ctx [Hash] the slot context
      def begin_when(slot_name, ctx)
        raise NotImplementedError, "#{self.class}#begin_when not implemented"
      end

      # Called on leaving a when block.
      def end_when
        raise NotImplementedError, "#{self.class}#end_when not implemented"
      end

      # Called on entering an each @list as item { ... } block.
      # @param slot_name [String] the list slot name
      # @param item_name [String] the loop variable name
      # @param element_type [Hash] the element's MosaicType
      # @param ctx [Hash] the slot context
      def begin_each(slot_name, item_name, element_type, ctx)
        raise NotImplementedError, "#{self.class}#begin_each not implemented"
      end

      # Called on leaving an each block.
      def end_each
        raise NotImplementedError, "#{self.class}#end_each not implemented"
      end

      # Called at the end of run(). Returns the emit result hash.
      # @return [Hash] { files: [{ filename: String, content: String }] }
      def emit
        raise NotImplementedError, "#{self.class}#emit not implemented"
      end
    end
  end
end
