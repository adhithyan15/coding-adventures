# frozen_string_literal: true

require_relative "lib/coding_adventures/barcode_1d/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_barcode_1d"
  spec.version       = CodingAdventures::Barcode1D::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "High-level 1D barcode pipeline for Ruby"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true",
  }

  spec.add_dependency "coding_adventures_codabar", "~> 0.1"
  spec.add_dependency "coding_adventures_code128", "~> 0.1"
  spec.add_dependency "coding_adventures_code39", "~> 0.1"
  spec.add_dependency "coding_adventures_ean_13", "~> 0.1"
  spec.add_dependency "coding_adventures_itf", "~> 0.1"
  spec.add_dependency "coding_adventures_paint_vm_metal_native", "~> 0.1"
  spec.add_dependency "coding_adventures_paint_codec_png_native", "~> 0.1"
  spec.add_dependency "coding_adventures_upc_a", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
