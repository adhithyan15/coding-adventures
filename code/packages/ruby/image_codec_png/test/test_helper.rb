# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  enable_coverage :branch
  add_filter "/test/"
  add_filter { |src| !src.filename.start_with?(File.expand_path("../lib", __dir__)) }
  minimum_coverage line: 95
end

require "minitest/autorun"
require "coding_adventures_image_codec_png"

ImageCodecPng = CodingAdventures::ImageCodecPng
