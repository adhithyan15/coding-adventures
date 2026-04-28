# frozen_string_literal: true

require_relative "lib/coding_adventures/aztec_code/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_aztec_code"
  spec.version       = CodingAdventures::AztecCode::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Aztec Code encoder — ISO/IEC 24778:2008"
  spec.description   = "Pure-Ruby Aztec Code encoder. Encodes strings or byte " \
                       "arrays to ModuleGrid (Compact or Full symbols, 1–32 " \
                       "layers), with GF(256)/0x12D Reed-Solomon ECC, GF(16) " \
                       "mode-message protection, bit stuffing, and PaintScene " \
                       "output via barcode_2d."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"      => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # Runtime dependencies — loaded before own modules per Ruby require ordering rules.
  spec.add_dependency "coding_adventures_paint_instructions", "~> 0.1"
  spec.add_dependency "coding_adventures_barcode_2d", "~> 0.1"

  spec.add_development_dependency "rspec",     "~> 3.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard",  "~> 1.0"
  spec.add_development_dependency "rake",      "~> 13.0"
end
