# frozen_string_literal: true

require "coding_adventures/pixel_container"
require_relative "coding_adventures/paint_vm_metal_native/version"

begin
  require "paint_vm_metal_native"
rescue LoadError
  nil
end

module CodingAdventures
  module PaintVmMetalNative
    class Error < StandardError; end

    module_function

    def supported_runtime?
      RUBY_PLATFORM.include?("darwin") && /arm64|aarch64/.match?(RUBY_PLATFORM)
    end

    def available?
      supported_runtime? && respond_to?(:render_rect_scene_native)
    end

    def render(scene)
      raise Error, "Metal is only available on macOS arm64" unless supported_runtime?
      raise Error, "paint_vm_metal_native extension is not available" unless respond_to?(:render_rect_scene_native)

      width, height, background, rects = encode_scene(scene)
      rendered_width, rendered_height, data = render_rect_scene_native(width, height, background, rects)
      CodingAdventures::PixelContainer::Container.new(rendered_width, rendered_height, data.b)
    end

    def encode_scene(scene)
      instructions = fetch_value(scene, :instructions)
      [
        fetch_value(scene, :width).to_f,
        fetch_value(scene, :height).to_f,
        fetch_value(scene, :background, "#ffffff").to_s,
        instructions.map { |instruction| encode_instruction(instruction) },
      ]
    end
    private_class_method :encode_scene

    def encode_instruction(instruction)
      kind = fetch_value(instruction, :kind)
      raise Error, "only rect paint instructions are supported right now" unless kind.to_s == "rect"

      [
        fetch_value(instruction, :x).to_f,
        fetch_value(instruction, :y).to_f,
        fetch_value(instruction, :width).to_f,
        fetch_value(instruction, :height).to_f,
        fetch_value(instruction, :fill, "#000000").to_s,
      ]
    end
    private_class_method :encode_instruction

    def fetch_value(object, key, default = :__missing__)
      if object.is_a?(Hash)
        return object[key] if object.key?(key)
        return object[key.to_s] if object.key?(key.to_s)
      elsif object.respond_to?(key)
        return object.public_send(key)
      end

      return default unless default == :__missing__

      raise Error, "scene is missing #{key}"
    end
    private_class_method :fetch_value
  end
end
