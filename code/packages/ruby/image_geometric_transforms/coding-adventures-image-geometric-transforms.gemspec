# frozen_string_literal: true

Gem::Specification.new do |spec|
  spec.name          = "coding-adventures-image-geometric-transforms"
  spec.version       = "0.1.0"
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "IMG04: Geometric transforms on PixelContainer"
  spec.description   = "Flip, rotate, crop, pad, scale, rotate, affine, and perspective-warp " \
                       "operations on RGBA8 PixelContainers. Nearest, bilinear, and bicubic " \
                       "(Catmull-Rom) interpolation with correct sRGB ↔ linear-light conversion."
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
