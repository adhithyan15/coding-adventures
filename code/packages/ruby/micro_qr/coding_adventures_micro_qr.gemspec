# frozen_string_literal: true

require_relative "lib/coding_adventures/micro_qr/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_micro_qr"
  spec.version       = CodingAdventures::MicroQR::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Micro QR Code encoder — ISO/IEC 18004:2015 Annex E"
  spec.description   = "Encodes strings to Micro QR Code (M1–M4) module grids " \
                       "with Reed-Solomon ECC, masking, and PaintScene output."
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
  spec.add_dependency "coding_adventures_barcode_2d",        "~> 0.1"

  spec.add_development_dependency "minitest",   "~> 5.0"
  spec.add_development_dependency "simplecov",  "~> 0.22"
  spec.add_development_dependency "rake",       "~> 13.0"
end
