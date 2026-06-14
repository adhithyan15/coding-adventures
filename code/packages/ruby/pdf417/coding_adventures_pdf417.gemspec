# frozen_string_literal: true

require_relative "lib/coding_adventures/pdf417/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_pdf417"
  spec.version       = CodingAdventures::PDF417::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "PDF417 stacked-linear barcode encoder — ISO/IEC 15438:2015"
  spec.description   = "Pure-Ruby PDF417 encoder. Encodes strings or byte arrays " \
                       "to ModuleGrid via byte-compaction mode (codeword 924 latch, " \
                       "6-bytes-to-5-codewords base-900 packing), auto ECC level " \
                       "selection, GF(929) Reed-Solomon error correction (b=3 " \
                       "convention, α=3), and dimension auto-selection for a roughly " \
                       "square symbol. Zero runtime dependencies."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"       => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # No runtime dependencies — the encoder is fully self-contained.
  # GF(929) tables, RS encoding, cluster tables, and the ModuleGrid struct
  # all live in this gem.

  spec.add_development_dependency "rspec",     "~> 3.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard",  "~> 1.0"
  spec.add_development_dependency "rake",      "~> 13.0"
end
