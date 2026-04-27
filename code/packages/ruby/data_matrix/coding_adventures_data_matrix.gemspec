# frozen_string_literal: true

require_relative "lib/coding_adventures/data_matrix/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_data_matrix"
  spec.version       = CodingAdventures::DataMatrix::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Data Matrix ECC200 encoder — ISO/IEC 16022:2006"
  spec.description   = "Pure-Ruby Data Matrix ECC200 encoder. Encodes strings to a " \
                       "ModuleGrid via ASCII encoding (with digit-pair compaction), " \
                       "scrambled-pad codewords, GF(256)/0x12D Reed-Solomon error " \
                       "correction (b=1 convention, per-block interleaving), and the " \
                       "Utah diagonal placement algorithm. Supports all 30 ECC200 " \
                       "symbol sizes (24 square + 6 rectangular). Zero runtime " \
                       "dependencies."
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
  # GF(256)/0x12D tables, RS encoding, Utah placement, and ModuleGrid
  # all live in this gem without external dependencies.

  spec.add_development_dependency "rspec",     "~> 3.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard",  "~> 1.0"
  spec.add_development_dependency "rake",      "~> 13.0"
end
