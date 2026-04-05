# frozen_string_literal: true

# ================================================================
# MosaicWebComponentRenderer -- Emits TypeScript Custom Element Classes
# ================================================================
#
# This is the Web Components backend for the Mosaic compiler.
# It implements the MosaicRenderer protocol and is driven by MosaicVM.
#
# The generated TypeScript file contains a Custom Element that:
#   - Extends HTMLElement
#   - Uses Shadow DOM for style encapsulation
#   - Exposes Mosaic slots as property setters/getters
#   - Rebuilds shadow DOM via _render() on any property change
#   - Observes HTML attributes for primitive-type slots
#
# Architecture: Fragment List
# ---------------------------
# During VM traversal, the renderer accumulates RenderFragment objects.
# On emit(), these are serialized into a _render() method body.
#
# Fragment types:
#   :open_tag    → html += '<tag style="...">'; (static HTML)
#   :close_tag   → html += '</tag>';
#   :self_closing → html += '<tag ... />';
#   :slot_ref    → html += `...${escapeHtml(this._field)}...`;
#   :slot_proj   → html += '<slot name="action"></slot>';
#   :when_open   → if (this._field) {
#   :when_close  → }
#   :each_open   → this._field.forEach((item) => {
#   :each_close  → });
#
# Security:
#   Text slot values pass through _escapeHtml().
#   Image URLs validated against javascript: scheme.
#   Colors always emitted as rgba() strings.
#
# Tag Name Convention:
#   PascalCase → kebab-case with mosaic- prefix
#   ProfileCard → <mosaic-profile-card>
# ================================================================

require "coding_adventures_mosaic_vm"

module CodingAdventures
  module MosaicEmitWebcomponent
    # The Web Components backend for the Mosaic compiler.
    #
    # @example
    #   ir = CodingAdventures::MosaicAnalyzer.analyze(source)
    #   vm = CodingAdventures::MosaicVm::MosaicVM.new(ir)
    #   result = vm.run(CodingAdventures::MosaicEmitWebcomponent::WebComponentRenderer.new)
    class WebComponentRenderer
      include CodingAdventures::MosaicVm::MosaicRenderer

      def initialize
        @component_name = ""
        @slots = []
        @fragments = []
        @stack = []
      end

      # ----------------------------------------------------------------
      # MosaicRenderer protocol
      # ----------------------------------------------------------------

      def begin_component(name, slots)
        @component_name = name
        @slots = slots
        @fragments = []
        @stack = [{ kind: "component", fragments: [] }]
      end

      def end_component
        # No-op
      end

      def emit
        content = build_file
        { files: [{ filename: "#{pascal_to_kebab(@component_name)}.ts", content: content }] }
      end

      def begin_node(tag, is_primitive, properties, _ctx)
        frame = {
          kind: "node",
          tag: tag,
          is_primitive: is_primitive,
          properties: properties,
          fragments: []
        }
        @stack.push(frame)
      end

      def end_node(_tag)
        frame = @stack.pop
        frags = node_to_fragments(frame)
        append_fragments(frags)
      end

      def render_slot_child(slot_name, slot_type, _ctx)
        # Named slot projection using HTML slot element
        append_fragments([{ kind: :slot_proj, slot_name: slot_name }])
      end

      def begin_when(slot_name, _ctx)
        @stack.push({ kind: "when", slot_name: slot_name, fragments: [] })
      end

      def end_when
        frame = @stack.pop
        # Emit when_open, children, when_close
        field = "_#{slot_name_to_field(frame[:slot_name])}"
        combined = [{ kind: :when_open, field: field }] +
                   frame[:fragments] +
                   [{ kind: :when_close }]
        append_fragments(combined)
      end

      def begin_each(slot_name, item_name, _element_type, _ctx)
        @stack.push({ kind: "each", slot_name: slot_name, item_name: item_name, fragments: [] })
      end

      def end_each
        frame = @stack.pop
        field = "_#{slot_name_to_field(frame[:slot_name])}"
        combined = [{ kind: :each_open, field: field, item_name: frame[:item_name] }] +
                   frame[:fragments] +
                   [{ kind: :each_close }]
        append_fragments(combined)
      end

      # ----------------------------------------------------------------
      # Node fragment building
      # ----------------------------------------------------------------

      def node_to_fragments(frame)
        tag = frame[:tag]
        is_primitive = frame[:is_primitive]
        properties = frame[:properties]

        html_tag, self_closing = primitive_html_tag(tag, is_primitive)
        styles = {}
        attrs = []
        text_slot_expr = nil
        text_literal = nil

        properties.each do |prop|
          tc_slot, tc_lit = apply_property(prop, tag, styles, attrs)
          text_slot_expr ||= tc_slot
          text_literal ||= tc_lit
        end

        open_html = build_open_tag(html_tag, styles, attrs)

        if self_closing
          return [{ kind: :self_closing, html: open_html.sub(/>$/, " />") }] + frame[:fragments]
        end

        result = [{ kind: :open_tag, html: open_html }]

        if text_slot_expr
          result << { kind: :slot_ref, expr: text_slot_expr }
        elsif text_literal
          result << { kind: :open_tag, html: text_literal }
        end

        result += frame[:fragments]
        result << { kind: :close_tag, tag: html_tag }
        result
      end

      def primitive_html_tag(tag, is_primitive)
        return [tag.downcase, false] unless is_primitive

        case tag
        when "Box"     then ["div", false]
        when "Column"  then ["div", false]
        when "Row"     then ["div", false]
        when "Text"    then ["span", false]
        when "Image"   then ["img", true]
        when "Spacer"  then ["div", false]
        when "Scroll"  then ["div", false]
        when "Divider" then ["hr", true]
        when "Icon"    then ["span", false]
        when "Stack"   then ["div", false]
        else ["div", false]
        end
      end

      def build_open_tag(html_tag, styles, attrs)
        parts = []
        unless styles.empty?
          style_str = styles.map { |k, v| "#{k}: #{v}" }.join("; ")
          parts << "style=\"#{style_str}\""
        end
        parts += attrs
        if parts.empty?
          "<#{html_tag}>"
        else
          "<#{html_tag} #{parts.join(" ")}>"
        end
      end

      # Apply a property, returning [text_slot_expr, text_literal] (either may be nil)
      def apply_property(prop, tag, styles, attrs)
        name = prop[:name]
        val  = prop[:value]
        text_slot_expr = nil
        text_literal = nil

        case name
        when "content"
          if val[:kind] == "slot_ref"
            text_slot_expr = "${this._escapeHtml(String(this.#{slot_name_to_field(val[:slot_name])}))}"
          else
            text_literal = format_html_value(val)
          end
        when "source"
          if val[:kind] == "slot_ref"
            attrs << "src=\"${this._validateUrl(String(this.#{slot_name_to_field(val[:slot_name])}))}\""
          else
            # Reject javascript: URLs at code-generation time for literal src values
            src_val = format_html_value(val)
            if src_val.downcase.strip.start_with?("javascript:")
              src_val = "about:blank"
            end
            attrs << "src=\"#{src_val}\""
          end
        when "a11y-label"
          if val[:kind] == "slot_ref"
            attrs << "aria-label=\"${this._escapeHtml(String(this.#{slot_name_to_field(val[:slot_name])}))}\""
          else
            attrs << "aria-label=\"#{format_html_value(val)}\""
          end
        when "a11y-hidden"
          attrs << "aria-hidden=\"true\""
        when "padding"
          styles["padding"] = dim_to_css(val)
        when "background", "background-color"
          styles["background"] = color_to_css(val)
        when "color"
          styles["color"] = color_to_css(val)
        when "width"
          styles["width"] = dim_to_css(val)
        when "height"
          styles["height"] = dim_to_css(val)
        when "font-size"
          styles["font-size"] = dim_to_css(val)
        when "gap"
          styles["gap"] = dim_to_css(val)
        when "corner-radius"
          styles["border-radius"] = dim_to_css(val)
        when "opacity"
          styles["opacity"] = val[:value].to_s if val[:kind] == "number"
        end

        [text_slot_expr, text_literal]
      end

      # ----------------------------------------------------------------
      # File building
      # ----------------------------------------------------------------

      def build_file
        element_name = pascal_to_kebab(@component_name)
        lines = []
        lines << "// AUTO-GENERATED — DO NOT EDIT"
        lines << "// Generated by MosaicWebComponentRenderer"
        lines << ""
        lines << "export class #{@component_name} extends HTMLElement {"
        lines << "  private _shadow: ShadowRoot;"
        lines << ""

        # Private fields
        @slots.each do |slot|
          ts_type = ts_type_for_slot(slot.type)
          default_val = slot.default_value ? ts_default_value(slot.default_value) : ts_zero_value(slot.type)
          lines << "  private _#{slot_name_to_field(slot.name)}: #{ts_type} = #{default_val};"
        end
        lines << ""

        # Observed attributes (primitive scalar types)
        observed = @slots.select { |s| scalar_type?(s.type) }.map { |s| s.name }
        unless observed.empty?
          lines << "  static get observedAttributes() {"
          lines << "    return [#{observed.map { |n| "\"#{n}\"" }.join(", ")}];"
          lines << "  }"
          lines << ""
        end

        lines << "  constructor() {"
        lines << "    super();"
        lines << "    this._shadow = this.attachShadow({ mode: 'open' });"
        lines << "  }"

        # Setters/getters
        @slots.each do |slot|
          ts_type = ts_type_for_slot(slot.type)
          field = slot_name_to_field(slot.name)
          lines << ""
          lines << "  set #{field}(value: #{ts_type}) {"
          lines << "    this._#{field} = value;"
          lines << "    this._render();"
          lines << "  }"
          lines << ""
          lines << "  get #{field}(): #{ts_type} {"
          lines << "    return this._#{field};"
          lines << "  }"
        end

        # connectedCallback
        lines << ""
        lines << "  connectedCallback() {"
        lines << "    this._render();"
        lines << "  }"

        # _render method
        lines << ""
        lines << "  private _render() {"
        lines << "    let html = '';"
        serialize_fragments(@stack[0][:fragments]).each do |line|
          lines << "    #{line}"
        end
        lines << "    this._shadow.innerHTML = html;"
        lines << "  }"

        # _escapeHtml helper
        lines << ""
        lines << "  private _escapeHtml(str: string): string {"
        lines << '    return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;")'
        lines << '               .replace(/"/g, "&quot;").replace(/\'/g, "&#39;");'
        lines << "  }"

        # _validateUrl helper
        lines << ""
        lines << "  private _validateUrl(url: string): string {"
        lines << '    return url.startsWith("javascript:") ? "" : url;'
        lines << "  }"

        lines << "}"
        lines << ""
        lines << "customElements.define(\"#{element_name}\", #{@component_name});"
        lines << ""

        lines.join("\n")
      end

      # Serialize fragments into JavaScript _render() body lines
      def serialize_fragments(frags)
        result = []
        frags.each do |frag|
          case frag[:kind]
          when :open_tag
            result << "html += #{frag[:html].inspect};"
          when :close_tag
            result << "html += '</#{frag[:tag]}>';"
          when :self_closing
            result << "html += #{frag[:html].inspect};"
          when :slot_ref
            result << "html += `#{frag[:expr]}`;"
          when :slot_proj
            result << "html += '<slot name=\"#{frag[:slot_name]}\"></slot>';"
          when :when_open
            result << "if (this.#{frag[:field]}) {"
          when :when_close
            result << "}"
          when :each_open
            result << "this.#{frag[:field]}.forEach((#{frag[:item_name]}) => {"
          when :each_close
            result << "});"
          end
        end
        result
      end

      # ----------------------------------------------------------------
      # Value formatting helpers
      # ----------------------------------------------------------------

      def format_html_value(val)
        case val[:kind]
        when "string"    then val[:value]
        when "number"    then val[:value].to_s
        when "bool"      then val[:value].to_s
        when "color"     then rgba_string(val)
        when "dimension" then dim_to_css_str(val[:value], val[:unit])
        else ""
        end
      end

      def dim_to_css(val)
        return val[:value].to_s if val[:kind] == "number"
        return val[:value] if val[:kind] == "string"

        dim_to_css_str(val[:value], val[:unit]) if val[:kind] == "dimension"
      end

      def dim_to_css_str(value, unit)
        case unit
        when "dp", "sp" then "#{value.to_i}px"
        when "%"        then "#{value.to_i}%"
        else "#{value}#{unit}"
        end
      end

      def color_to_css(val)
        return val[:value] if val[:kind] == "string"
        return rgba_string(val) if val[:kind] == "color"

        val.to_s
      end

      def rgba_string(val)
        a = (val[:a] / 255.0).round(3)
        "rgba(#{val[:r]}, #{val[:g]}, #{val[:b]}, #{a})"
      end

      # ----------------------------------------------------------------
      # TypeScript type helpers
      # ----------------------------------------------------------------

      def ts_type_for_slot(type)
        case type[:kind]
        when "text"      then "string"
        when "number"    then "number"
        when "bool"      then "boolean"
        when "image"     then "string"
        when "color"     then "string"
        when "node"      then "HTMLElement | null"
        when "component" then "HTMLElement | null"
        when "list"      then "#{ts_type_for_slot(type[:element_type])}[]"
        else "unknown"
        end
      end

      def ts_default_value(val)
        case val[:kind]
        when "string"    then "\"#{val[:value]}\""
        when "number"    then val[:value].to_s
        when "bool"      then val[:value].to_s
        when "color_hex" then "\"#{val[:value]}\""
        when "dimension" then "\"#{val[:value]}#{val[:unit]}\""
        else "null"
        end
      end

      def ts_zero_value(type)
        case type[:kind]
        when "text", "image", "color" then '""'
        when "number"                 then "0"
        when "bool"                   then "false"
        when "node", "component"      then "null"
        when "list"                   then "[]"
        else "null"
        end
      end

      def scalar_type?(type)
        %w[text number bool image color].include?(type[:kind])
      end

      # ----------------------------------------------------------------
      # Naming helpers
      # ----------------------------------------------------------------

      # Convert PascalCase to kebab-case with mosaic- prefix
      # ProfileCard → mosaic-profile-card
      def pascal_to_kebab(name)
        kebab = name.gsub(/([A-Z])/) { "-#{$1.downcase}" }.sub(/^-/, "")
        "mosaic-#{kebab}"
      end

      # Convert slot name (kebab-case) to camelCase JS field name
      # avatar-url → avatarUrl
      def slot_name_to_field(name)
        parts = name.split("-")
        parts[0] + parts[1..].map(&:capitalize).join
      end

      # ----------------------------------------------------------------
      # Stack helpers
      # ----------------------------------------------------------------

      def append_fragments(frags)
        top = @stack.last
        top[:fragments] += frags
      end
    end
  end
end
