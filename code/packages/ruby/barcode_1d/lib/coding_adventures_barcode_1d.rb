# frozen_string_literal: true

require "coding_adventures_codabar"
require "coding_adventures_code128"
require "coding_adventures_code39"
require "coding_adventures_ean_13"
require "coding_adventures_itf"
require "coding_adventures_upc_a"
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
      when :codabar
        CodingAdventures::Codabar.layout_codabar(data, layout_config)
      when :code128
        CodingAdventures::Code128.layout_code128(data, layout_config)
      when :code39
        CodingAdventures::Code39.layout_code39(data, layout_config)
      when :ean13
        CodingAdventures::Ean13.layout_ean_13(data, layout_config)
      when :itf
        CodingAdventures::Itf.layout_itf(data, layout_config)
      when :upca
        CodingAdventures::UpcA.layout_upc_a(data, layout_config)
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
      return :codabar if normalized == "codabar"
      return :code128 if normalized == "code128"
      return :code39 if normalized == "code39"
      return :ean13 if normalized == "ean13"
      return :itf if normalized == "itf"
      return :upca if normalized == "upca"

      raise UnsupportedSymbologyError, "unsupported symbology: #{symbology}"
    end
    private_class_method :normalize_symbology
  end
end
