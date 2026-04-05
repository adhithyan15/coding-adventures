# frozen_string_literal: true

# ================================================================
# MosaicReactRenderer -- Emits TypeScript React Functional Components
# ================================================================
#
# This is the React backend for the Mosaic compiler. It implements
# the MosaicRenderer protocol and is driven by MosaicVM.
#
# Architecture: String Stack
# --------------------------
# The renderer maintains a stack of string buffers, one per open node.
# When begin_node is called, a new buffer is pushed. When end_node is
# called, the buffer is popped, converted to JSX, and appended to the
# parent buffer.
#
# Output File: ComponentName.tsx
# --------------------------------
# The generated file contains:
#   1. Auto-generated file header
#   2. import React from "react";
#   3. Optional type-scale CSS import
#   4. Optional component type imports
#   5. Props interface
#   6. Exported functional component
#
# Primitive Node → JSX Element Mapping:
#   Box     → <div style={{ position: 'relative' }}>
#   Column  → <div style={{ display: 'flex', flexDirection: 'column' }}>
#   Row     → <div style={{ display: 'flex', flexDirection: 'row' }}>
#   Text    → <span> (or <h2> if a11y-role: heading)
#   Image   → <img ... /> (self-closing)
#   Spacer  → <div style={{ flex: 1 }}>
#   Scroll  → <div style={{ overflow: 'auto' }}>
#   Divider → <hr style={{ ... }} /> (self-closing)
#   Icon    → <span class="icon">
#   Stack   → <div style={{ position: 'relative' }}>
#
# Property → Inline Style Mapping:
#   Most properties → camelCase inline React styles
#   style: heading.large → className="mosaic-heading-large"
#   a11y-label, a11y-role, a11y-hidden → ARIA attributes
#   content (Text) → JSX children text
#   source (Image) → src attribute
#   Colors → rgba(r, g, b, alpha/255.0)
#   Dimensions with dp/sp → px; % passes through
# ================================================================

require "coding_adventures_mosaic_vm"

module CodingAdventures
  module MosaicEmitReact
    # The React backend for the Mosaic compiler.
    #
    # Include with MosaicVM:
    #   ir = CodingAdventures::MosaicAnalyzer.analyze(source)
    #   vm = CodingAdventures::MosaicVm::MosaicVM.new(ir)
    #   renderer = CodingAdventures::MosaicEmitReact::ReactRenderer.new
    #   result = vm.run(renderer)
    #   # result[:files][0][:filename] == "MyComponent.tsx"
    class ReactRenderer
      include CodingAdventures::MosaicVm::MosaicRenderer

      def initialize
        @component_name   = ""
        @slots            = []
        @stack            = []
        @slot_imports     = Set.new  # component-type slot imports
        @node_imports     = Set.new  # non-primitive node tag imports
        @needs_type_scale = false
      end

      # ----------------------------------------------------------------
      # MosaicRenderer protocol implementation
      # ----------------------------------------------------------------

      def begin_component(name, slots)
        @component_name   = name
        @slots            = slots
        @stack            = [{ kind: "component", lines: [] }]
        @slot_imports     = Set.new
        @node_imports     = Set.new
        @needs_type_scale = false

        # Pre-scan slots for component-type imports
        slots.each do |slot|
          case slot.type[:kind]
          when "component"
            @slot_imports.add(slot.type[:name])
          when "list"
            @slot_imports.add(slot.type[:element_type][:name]) if slot.type[:element_type][:kind] == "component"
          end
        end
      end

      def end_component
        # No-op — root node JSX is already in stack[0].lines
      end

      def emit
        content = build_file
        { files: [{ filename: "#{@component_name}.tsx", content: content }] }
      end

      def begin_node(tag, is_primitive, properties, _ctx)
        frame = build_node_frame(tag, is_primitive, properties)
        @stack.push(frame)
      end

      def end_node(_tag)
        frame = @stack.pop
        jsx = build_jsx_element(frame)
        append_to_parent(jsx)
      end

      def render_slot_child(slot_name, _slot_type, _ctx)
        # Slot refs used as children render as the destructured prop variable
        append_to_parent("{#{slot_name}}")
      end

      def begin_when(slot_name, _ctx)
        @stack.push({ kind: "when", slot_name: slot_name, lines: [] })
      end

      def end_when
        frame = @stack.pop
        children = frame[:lines]
        body = if children.length == 1
                 children[0]
               else
                 inner = children.map { |l| "  #{l}" }.join("\n")
                 "<>\n#{inner}\n</>"
               end
        indented = body.split("\n").map { |l| "  #{l}" }.join("\n")
        jsx = "{#{frame[:slot_name]} && (\n#{indented}\n)}"
        append_to_parent(jsx)
      end

      def begin_each(slot_name, item_name, _element_type, _ctx)
        @stack.push({ kind: "each", slot_name: slot_name, item_name: item_name, lines: [] })
      end

      def end_each
        frame = @stack.pop
        indented = frame[:lines].map { |l| "    #{l}" }.join("\n")
        jsx = "{#{frame[:slot_name]}.map((#{frame[:item_name]}, _index) => (\n" \
              "  <React.Fragment key={_index}>\n" \
              "#{indented}\n" \
              "  </React.Fragment>\n" \
              "))}"
        append_to_parent(jsx)
      end

      # ----------------------------------------------------------------
      # Node Frame Building
      # ----------------------------------------------------------------

      def build_node_frame(tag, is_primitive, properties)
        styles = {}
        attrs = []
        class_names = []
        text_content = nil
        self_closing = false

        # Step 1: Base styles and JSX tag from the primitive element type
        jsx_tag = if is_primitive
                    map_primitive_tag(tag, styles)
                  else
                    @node_imports.add(tag)
                    tag
                  end

        # self-closing for specific elements
        self_closing = %w[Image Divider].include?(tag)

        # Step 2: Apply each property to the frame
        properties.each do |prop|
          tc = apply_property(prop, tag, styles, attrs, class_names)
          text_content = tc if tc
        end

        # Step 3: Post-process — a11y-role: heading changes Text → h2
        if tag == "Text"
          idx = attrs.index('role="heading"')
          if idx
            jsx_tag = "h2"
            attrs.delete_at(idx)
          end
        end

        {
          kind: "node", tag: tag, jsx_tag: jsx_tag,
          styles: styles, attrs: attrs, class_names: class_names,
          text_content: text_content, self_closing: self_closing,
          lines: []
        }
      end

      def map_primitive_tag(tag, styles)
        case tag
        when "Box"
          styles["position"] = '"relative"'
          "div"
        when "Column"
          styles["display"] = '"flex"'
          styles["flexDirection"] = '"column"'
          "div"
        when "Row"
          styles["display"] = '"flex"'
          styles["flexDirection"] = '"row"'
          "div"
        when "Text"   then "span"
        when "Image"  then "img"
        when "Spacer"
          styles["flex"] = "1"
          "div"
        when "Scroll"
          styles["overflow"] = '"auto"'
          "div"
        when "Divider"
          styles["border"] = '"none"'
          styles["borderTop"] = '"1px solid currentColor"'
          "hr"
        when "Icon"   then "span"
        when "Stack"
          styles["position"] = '"relative"'
          "div"
        else
          "div"
        end
      end

      # Apply a single property to the frame, returning text content if applicable.
      def apply_property(prop, tag, styles, attrs, class_names)
        name = prop[:name]
        val  = prop[:value]
        text_content = nil

        case name
        when "content"
          # Text content → JSX children
          text_content = format_value_jsx(val)
        when "source"
          attrs << "src={#{format_value_jsx(val)}}"
        when "style"
          # Typography scale: heading.large → className
          if val[:kind] == "enum"
            class_names << "mosaic-#{val[:namespace]}-#{val[:member]}"
            @needs_type_scale = true
          end
        when "a11y-label"
          attrs << "aria-label={#{format_value_jsx(val)}}"
        when "a11y-role"
          attrs << "role=#{format_attr_value(val)}"
        when "a11y-hidden"
          attrs << "aria-hidden={#{format_value_jsx(val)}}"
        when "padding"
          styles["padding"] = "#{format_css_value(val)}"
        when "padding-top"
          styles["paddingTop"] = "#{format_css_value(val)}"
        when "padding-bottom"
          styles["paddingBottom"] = "#{format_css_value(val)}"
        when "padding-left"
          styles["paddingLeft"] = "#{format_css_value(val)}"
        when "padding-right"
          styles["paddingRight"] = "#{format_css_value(val)}"
        when "margin"
          styles["margin"] = "#{format_css_value(val)}"
        when "background", "background-color"
          styles["backgroundColor"] = "#{format_css_value(val)}"
        when "color"
          styles["color"] = "#{format_css_value(val)}"
        when "width"
          styles["width"] = "#{format_css_value(val)}"
        when "height"
          styles["height"] = "#{format_css_value(val)}"
        when "font-size"
          styles["fontSize"] = "#{format_css_value(val)}"
        when "align"
          if val[:kind] == "string" || val[:kind] == "ident"
            styles["textAlign"] = "\"#{val[:value]}\""
          end
        when "gap"
          styles["gap"] = "#{format_css_value(val)}"
        when "corner-radius"
          styles["borderRadius"] = "#{format_css_value(val)}"
        when "opacity"
          if val[:kind] == "number"
            styles["opacity"] = val[:value].to_s
          end
        else
          # Generic property → camelCase style
          css_key = kebab_to_camel(name)
          styles[css_key] = "#{format_css_value(val)}"
        end

        text_content
      end

      # ----------------------------------------------------------------
      # JSX Building
      # ----------------------------------------------------------------

      def build_jsx_element(frame)
        jsx_tag = frame[:jsx_tag]
        styles = frame[:styles]
        attrs = frame[:attrs]
        class_names = frame[:class_names]

        # Build style attribute
        style_parts = styles.map { |k, v| "#{k}: #{v}" }
        style_attr = style_parts.empty? ? nil : "style={{ #{style_parts.join(", ")} }}"

        # Build className attribute
        class_attr = class_names.empty? ? nil : "className=\"#{class_names.join(" ")}\""

        # Assemble all attributes
        all_attrs = [style_attr, class_attr, *attrs].compact
        attr_str = all_attrs.empty? ? "" : " #{all_attrs.join(" ")}"

        if frame[:self_closing]
          "<#{jsx_tag}#{attr_str} />"
        elsif frame[:text_content]
          # Text node: inline content
          "<#{jsx_tag}#{attr_str}>#{frame[:text_content]}</#{jsx_tag}>"
        elsif frame[:lines].empty?
          "<#{jsx_tag}#{attr_str} />"
        else
          children = frame[:lines].map { |l| "  #{l}" }.join("\n")
          "<#{jsx_tag}#{attr_str}>\n#{children}\n</#{jsx_tag}>"
        end
      end

      # ----------------------------------------------------------------
      # File Building
      # ----------------------------------------------------------------

      def build_file
        lines = []
        lines << "// AUTO-GENERATED — DO NOT EDIT"
        lines << "// Generated by MosaicReactRenderer"
        lines << ""
        lines << 'import React from "react";'
        lines << 'import "./mosaic-type-scale.css";' if @needs_type_scale
        @slot_imports.sort.each do |name|
          lines << "import type { #{name}Props } from \"./#{name}.js\";"
        end
        @node_imports.sort.each do |name|
          lines << "import { #{name} } from \"./#{name}.js\";"
        end
        lines << ""

        # Props interface
        lines << "interface #{@component_name}Props {"
        @slots.each do |slot|
          type_str = typescript_type(slot.type)
          optional = slot.required ? "" : "?"
          lines << "  #{slot.name}#{optional}: #{type_str};"
        end
        lines << "}"
        lines << ""

        # Destructured props
        prop_names = @slots.map { |s|
          s.default_value ? "#{s.name} = #{default_value_ts(s.default_value)}" : s.name
        }.join(", ")

        lines << "export function #{@component_name}({ #{prop_names} }: #{@component_name}Props) {"
        lines << "  return ("

        # Root JSX
        root_lines = @stack[0][:lines]
        root_lines.each do |line|
          lines << "    #{line}"
        end

        lines << "  );"
        lines << "}"
        lines << ""

        lines.join("\n")
      end

      # ----------------------------------------------------------------
      # Value Formatting Helpers
      # ----------------------------------------------------------------

      # Format a ResolvedValue for use as a JSX expression
      def format_value_jsx(val)
        case val[:kind]
        when "string"    then "\"#{val[:value]}\""
        when "number"    then val[:value].to_s
        when "bool"      then val[:value].to_s
        when "color"     then "\"#{rgba_string(val)}\""
        when "dimension" then "\"#{dim_to_css(val[:value], val[:unit])}\""
        when "enum"      then "\"#{val[:namespace]}.#{val[:member]}\""
        when "slot_ref"  then val[:slot_name]
        else val.inspect
        end
      end

      # Format a ResolvedValue as a CSS value string (for inline styles)
      def format_css_value(val)
        case val[:kind]
        when "string"    then "\"#{val[:value]}\""
        when "number"    then val[:value].to_s
        when "bool"      then "\"#{val[:value]}\""
        when "color"     then "\"#{rgba_string(val)}\""
        when "dimension" then "\"#{dim_to_css(val[:value], val[:unit])}\""
        when "enum"      then "\"#{val[:namespace]}.#{val[:member]}\""
        when "slot_ref"  then val[:slot_name]
        else "\"#{val}\""
        end
      end

      # Format a ResolvedValue as a JSX attribute value string
      def format_attr_value(val)
        case val[:kind]
        when "string" then "\"#{val[:value]}\""
        when "ident"  then "\"#{val[:value]}\""
        else "\"#{val[:value]}\""
        end
      end

      # Convert dimension value + unit to CSS string
      def dim_to_css(value, unit)
        case unit
        when "dp", "sp" then "#{value.to_i}px"
        when "%"        then "#{value.to_i}%"
        else "#{value}#{unit}"
        end
      end

      # Format RGBA color as CSS rgba() string
      def rgba_string(val)
        a = (val[:a] / 255.0).round(3)
        "rgba(#{val[:r]}, #{val[:g]}, #{val[:b]}, #{a})"
      end

      # Convert kebab-case to camelCase
      def kebab_to_camel(str)
        parts = str.split("-")
        parts[0] + parts[1..].map(&:capitalize).join
      end

      # Format a MosaicType as a TypeScript type string
      def typescript_type(type)
        case type[:kind]
        when "text"      then "string"
        when "number"    then "number"
        when "bool"      then "boolean"
        when "image"     then "string"
        when "color"     then "string"
        when "node"      then "React.ReactNode"
        when "component" then "React.ReactNode"
        when "list"      then "#{typescript_type(type[:element_type])}[]"
        else "unknown"
        end
      end

      # Format a MosaicValue as a TypeScript default value literal
      def default_value_ts(val)
        case val[:kind]
        when "string"    then "\"#{val[:value]}\""
        when "number"    then val[:value].to_s
        when "bool"      then val[:value].to_s
        when "color_hex" then "\"#{val[:value]}\""
        when "dimension" then "\"#{val[:value]}#{val[:unit]}\""
        else "undefined"
        end
      end

      # ----------------------------------------------------------------
      # Stack helpers
      # ----------------------------------------------------------------

      def append_to_parent(jsx)
        parent = @stack.last
        parent[:lines] << jsx
      end
    end
  end
end
