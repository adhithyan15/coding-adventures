# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "coding-adventures-image-codec-ppm"
  spec.version       = "0.1.0"
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "PPM (Portable Pixmap P6) image encoder and decoder"
  spec.description   = "Encodes RGBA8 PixelContainers to binary P6 PPM files and decodes them back. " \
                       "Alpha is dropped on encode; decoded pixels receive A=255. " \
                       "Handles '#' comment lines in PPM headers."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri"       => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest",  "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake",      "~> 13.0"
end
