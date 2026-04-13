# frozen_string_literal: true

require "coding_adventures_code39"
require_relative "coding_adventures/barcode_1d/version"

module CodingAdventures
  module Barcode1D
    class Error < StandardError; end
    class UnsupportedSymbologyError < Error; end
    class BackendUnavailableError < Error; end

    module_function

    DEFAULT_LAYOUT_CONFIG = CodingAdventures::Code39::DEFAULT_LAYOUT_CONFIG
    DEFAULT_RENDER_CONFIG = DEFAULT_LAYOUT_CONFIG

    def current_backend
      if RUBY_PLATFORM.include?("darwin") && /arm64|aarch64/.match?(RUBY_PLATFORM)
        :metal
      end
    end

    def build_scene(data, symbology: :code39, layout_config: DEFAULT_LAYOUT_CONFIG)
      case normalize_symbology(symbology)
      when :code39
        CodingAdventures::Code39.layout_code39(data, layout_config)
      end
    end

    def render_pixels(data, symbology: :code39, layout_config: DEFAULT_LAYOUT_CONFIG)
      raise BackendUnavailableError, "no native Paint VM is available for this host" unless current_backend == :metal

      require "coding_adventures_paint_vm_metal_native"
      CodingAdventures::PaintVmMetalNative.render(
        build_scene(data, symbology: symbology, layout_config: layout_config),
      )
    rescue LoadError => e
      raise BackendUnavailableError, "coding_adventures_paint_vm_metal_native is not installed: #{e.message}"
    end

    def render_png(data, symbology: :code39, layout_config: DEFAULT_LAYOUT_CONFIG)
      require "coding_adventures_paint_codec_png_native"
      CodingAdventures::PaintCodecPngNative.encode(
        render_pixels(data, symbology: symbology, layout_config: layout_config),
      )
    rescue LoadError => e
      raise BackendUnavailableError, "coding_adventures_paint_codec_png_native is not installed: #{e.message}"
    end

    def normalize_symbology(symbology)
      normalized = symbology.to_s.delete("_-").downcase
      return :code39 if normalized == "code39"

      raise UnsupportedSymbologyError, "unsupported symbology: #{symbology}"
    end
    private_class_method :normalize_symbology
  end
end
