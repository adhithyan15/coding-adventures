# frozen_string_literal: true

require "coding_adventures_paint_instructions"
require_relative "coding_adventures/paint_vm_ascii/version"

module CodingAdventures
  module PaintVmAscii
    module_function

    def render(scene, scale_x: 8, scale_y: 16)
      cols = (scene.width.to_f / scale_x).ceil
      rows = (scene.height.to_f / scale_y).ceil
      chars = Array.new(rows) { Array.new(cols, " ") }

      scene.instructions.each do |inst|
        case inst.kind.to_s
        when "rect"
          render_rect(inst, chars, scale_x, scale_y)
        else
          raise ArgumentError, "paint_vm_ascii: unsupported paint instruction kind: #{inst.kind}"
        end
      end

      chars.map { |row| row.join.rstrip }.join("\n").rstrip
    end

    def render_rect(inst, chars, scale_x, scale_y)
      fill = inst.fill
      return if fill.nil? || fill.empty? || fill == "transparent" || fill == "none"

      c1 = (inst.x.to_f / scale_x).round
      r1 = (inst.y.to_f / scale_y).round
      c2 = ((inst.x + inst.width).to_f / scale_x).round
      r2 = ((inst.y + inst.height).to_f / scale_y).round

      (r1..r2).each do |row|
        next if row.negative? || row >= chars.length

        (c1..c2).each do |col|
          next if col.negative? || col >= chars[row].length

          chars[row][col] = "\u2588"
        end
      end
    end
    private_class_method :render_rect
  end
end
