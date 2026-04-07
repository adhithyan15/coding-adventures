# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "coding-adventures-image-codec-bmp"
  spec.version       = "0.1.0"
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "BMP image encoder and decoder for the coding-adventures suite"
  spec.description   = "Encodes and decodes 32-bit RGBA BMP (Windows Bitmap) files. " \
                       "Produces top-down, uncompressed 32-bit BMPs with BGRA pixel layout. " \
                       "Handles both top-down (negative biHeight) and bottom-up (positive biHeight) files on decode."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri"       => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # No runtime dependencies — pixel_container is loaded via LOAD_PATH at runtime.

  spec.add_development_dependency "minitest",  "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake",      "~> 13.0"
end
