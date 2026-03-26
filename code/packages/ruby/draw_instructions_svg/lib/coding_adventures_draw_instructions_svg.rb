# frozen_string_literal: true

require "cgi"
require "ostruct"
require "coding_adventures_draw_instructions"
require_relative "coding_adventures/draw_instructions_svg/version"

module CodingAdventures
  # SVG serializer for the generic draw scene model.
  module DrawInstructionsSvg
    module_function

    def render_svg(scene)
      label = scene.metadata[:label] || "draw instructions scene"
      lines = []
      lines << %(<svg xmlns="http://www.w3.org/2000/svg" width="#{scene.width}" height="#{scene.height}" viewBox="0 0 #{scene.width} #{scene.height}" role="img" aria-label="#{CGI.escapeHTML(label.to_s)}">)
      lines << %(<rect x="0" y="0" width="#{scene.width}" height="#{scene.height}" fill="#{CGI.escapeHTML(scene.background)}" />)
      scene.instructions.each { |instruction| lines << render_instruction(instruction) }
      lines << "</svg>"
      lines.join("\n")
    end

    def svg_renderer
      OpenStruct.new(render: method(:render_svg))
    end

    def render_instruction(instruction)
      case instruction.kind
      when "rect"
        %(<rect x="#{instruction.x}" y="#{instruction.y}" width="#{instruction.width}" height="#{instruction.height}" fill="#{CGI.escapeHTML(instruction.fill)}"#{metadata_to_attributes(instruction.metadata)} />)
      when "text"
        %(<text x="#{instruction.x}" y="#{instruction.y}" text-anchor="#{instruction.align}" font-family="#{CGI.escapeHTML(instruction.font_family)}" font-size="#{instruction.font_size}" fill="#{CGI.escapeHTML(instruction.fill)}"#{metadata_to_attributes(instruction.metadata)}>#{CGI.escapeHTML(instruction.value)}</text>)
      else
        children = instruction.children.map { |child| render_instruction(child) }.join("\n")
        %(<g#{metadata_to_attributes(instruction.metadata)}>\n#{children}\n</g>)
      end
    end

    def metadata_to_attributes(metadata)
      metadata.map { |key, value| %( data-#{key}="#{CGI.escapeHTML(value.to_s)}") }.join
    end
  end
end
