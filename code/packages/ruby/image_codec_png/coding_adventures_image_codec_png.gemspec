# frozen_string_literal: true

require_relative "lib/coding_adventures/image_codec_png/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_image_codec_png"
  spec.version = CodingAdventures::ImageCodecPng::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Bounded IC18 PNG encoder and decoder reusing repository ZIP DEFLATE/CRC-32 and pixel container"
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_zip", "~> 0.2"
  spec.add_dependency "coding-adventures-pixel-container", "~> 0.1"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
end
