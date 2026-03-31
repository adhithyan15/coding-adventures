# frozen_string_literal: true

require "cgi"
require "coding_adventures_draw_instructions"
require_relative "coding_adventures/draw_instructions_svg/version"

module CodingAdventures
  # SVG renderer for the generic draw scene model.
  #
  # == Design Philosophy
  #
  # This package is intentionally boring in the best possible way. It knows
  # how to serialize a generic draw scene to SVG, and nothing more. It does
  # not contain barcode rules, chart rules, or any other producer domain
  # logic. That separation is the whole reason this package exists.
  #
  # == How It Works
  #
  # Each draw instruction type maps to one SVG element:
  #
  #   DrawRectInstruction -> <rect>
  #   DrawTextInstruction -> <text>
  #   DrawLineInstruction -> <line>
  #   DrawGroupInstruction -> <g>
  #   DrawClipInstruction -> <defs><clipPath>...</clipPath></defs><g clip-path="...">
  #
  # Metadata hashes are serialized as +data-*+ attributes so that semantic
  # information survives into the output. Browser tooling and visualizers
  # can inspect the metadata later.
  #
  # == XML Escaping
  #
  # All user-provided text and attribute values are escaped via CGI.escapeHTML
  # to prevent injection of malicious SVG/XML content.
  module DrawInstructionsSvg
    module_function

    # ------------------------------------------------------------------
    # SvgRenderer
    # ------------------------------------------------------------------
    # A renderer is any object with a +render(scene)+ method. SvgRenderer
    # wraps the module-level render_svg method in an object so it can be
    # passed to DrawInstructions.render_with.
    # ------------------------------------------------------------------

    # An object that responds to +render(scene)+ and produces an SVG string.
    #
    # Usage:
    #   renderer = CodingAdventures::DrawInstructionsSvg::SvgRenderer.new
    #   svg_string = renderer.render(scene)
    #
    # Or via the convenience method:
    #   svg_string = CodingAdventures::DrawInstructionsSvg.render_svg(scene)
    #
    class SvgRenderer
      def render(scene)
        DrawInstructionsSvg.render_svg(scene)
      end
    end

    # Render a DrawScene to a complete SVG document string.
    #
    # The output includes:
    # - An <svg> root with xmlns, width, height, viewBox, role, and aria-label
    # - A background <rect> filling the entire scene
    # - All instructions serialized recursively
    #
    # Clip IDs use a counter that resets each render call for deterministic output.
    def render_svg(scene)
      # Reset clip counter for deterministic output across renders.
      @clip_id_counter = 0

      label = if scene.metadata && scene.metadata[:label]
                CGI.escapeHTML(scene.metadata[:label].to_s)
              else
                "draw instructions scene"
              end

      lines = []
      lines << %(<svg xmlns="http://www.w3.org/2000/svg" width="#{scene.width}" height="#{scene.height}" viewBox="0 0 #{scene.width} #{scene.height}" role="img" aria-label="#{label}">)
      lines << %(  <rect x="0" y="0" width="#{scene.width}" height="#{scene.height}" fill="#{CGI.escapeHTML(scene.background)}" />)
      scene.instructions.each { |instruction| lines << render_instruction(instruction) }
      lines << "</svg>"
      lines.join("\n")
    end

    # ------------------------------------------------------------------
    # Instruction rendering
    # ------------------------------------------------------------------
    # Each instruction type dispatches to a dedicated method. The dispatch
    # uses a simple case statement on the +kind+ field.
    # ------------------------------------------------------------------

    # Dispatch one generic instruction to the matching SVG serializer.
    def render_instruction(instruction)
      case instruction.kind
      when "rect" then render_rect(instruction)
      when "text" then render_text(instruction)
      when "group" then render_group(instruction)
      when "line" then render_line(instruction)
      when "clip" then render_clip(instruction)
      end
    end

    # Serialize a rectangle instruction to an SVG <rect>.
    #
    # When +stroke+ is present, stroke and stroke-width attributes are added.
    # Otherwise only fill is rendered.
    def render_rect(instruction)
      stroke_attrs = if instruction.stroke
                       %( stroke="#{CGI.escapeHTML(instruction.stroke)}" stroke-width="#{instruction.stroke_width || 1}")
                     else
                       ""
                     end
      %(  <rect x="#{instruction.x}" y="#{instruction.y}" width="#{instruction.width}" height="#{instruction.height}" fill="#{CGI.escapeHTML(instruction.fill)}"#{stroke_attrs}#{metadata_to_attributes(instruction.metadata)} />)
    end

    # Serialize a text instruction to an SVG <text>.
    #
    # The +font_weight+ attribute is only emitted when it is non-nil and not
    # "normal", keeping the output clean for the common case.
    def render_text(instruction)
      weight_attr = if instruction.font_weight && instruction.font_weight != "normal"
                      %( font-weight="#{instruction.font_weight}")
                    else
                      ""
                    end
      %(  <text x="#{instruction.x}" y="#{instruction.y}" text-anchor="#{instruction.align}" font-family="#{CGI.escapeHTML(instruction.font_family)}" font-size="#{instruction.font_size}" fill="#{CGI.escapeHTML(instruction.fill)}"#{weight_attr}#{metadata_to_attributes(instruction.metadata)}>#{CGI.escapeHTML(instruction.value)}</text>)
    end

    # Serialize a group instruction to an SVG <g>.
    #
    # Children are rendered recursively and indented inside the group.
    def render_group(instruction)
      children = instruction.children.map { |child| render_instruction(child) }.join("\n")
      [
        %(  <g#{metadata_to_attributes(instruction.metadata)}>),
        children,
        "  </g>",
      ].join("\n")
    end

    # Serialize a line instruction to an SVG <line>.
    #
    # SVG <line> uses x1/y1/x2/y2 attributes -- a direct 1:1 mapping from
    # our DrawLineInstruction fields.
    def render_line(instruction)
      %(  <line x1="#{instruction.x1}" y1="#{instruction.y1}" x2="#{instruction.x2}" y2="#{instruction.y2}" stroke="#{CGI.escapeHTML(instruction.stroke)}" stroke-width="#{instruction.stroke_width}"#{metadata_to_attributes(instruction.metadata)} />)
    end

    # Serialize a clip instruction to an SVG <g> with a <clipPath>.
    #
    # SVG clipping uses a <clipPath> element containing a <rect> that defines
    # the clip region, referenced by clip-path="url(#id)" on a <g> wrapping
    # the clipped children. We generate unique IDs using a counter to avoid
    # collisions within a single render pass.
    def render_clip(instruction)
      @clip_id_counter += 1
      clip_id = "clip-#{@clip_id_counter}"
      children = instruction.children.map { |child| render_instruction(child) }.join("\n")
      [
        "  <defs>",
        %(    <clipPath id="#{clip_id}">),
        %(      <rect x="#{instruction.x}" y="#{instruction.y}" width="#{instruction.width}" height="#{instruction.height}" />),
        "    </clipPath>",
        "  </defs>",
        %(  <g clip-path="url(##{clip_id})"#{metadata_to_attributes(instruction.metadata)}>),
        children,
        "  </g>",
      ].join("\n")
    end

    # ------------------------------------------------------------------
    # Metadata serialization
    # ------------------------------------------------------------------
    # Metadata is serialized as data-* attributes on SVG elements. This is
    # a nice compromise: SVG stays valid, semantic information survives into
    # the output, and browser tooling can inspect the metadata later.
    # ------------------------------------------------------------------

    # Convert a metadata hash to a string of data-* attributes.
    #
    #   metadata_to_attributes({ char: "A", index: 0 })
    #   # => ' data-char="A" data-index="0"'
    #
    def metadata_to_attributes(metadata)
      return "" if metadata.nil? || metadata.empty?
      metadata.map { |key, value| %( data-#{key}="#{CGI.escapeHTML(value.to_s)}") }.join
    end
  end
end
