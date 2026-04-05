# frozen_string_literal: true

require_relative "lib/coding_adventures/pixel_container/version"

Gem::Specification.new do |spec|
  spec.name          = "coding-adventures-pixel-container"
  spec.version       = CodingAdventures::PixelContainer::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Fixed RGBA8 pixel buffer for image codec packages"
  spec.description   = "A compact, fixed-size RGBA8 pixel buffer backed by a binary String. " \
                       "Provides O(1) pixel read/write via String#getbyte/setbyte. " \
                       "Used as the common data structure for image codec packages (BMP, PPM, QOI)."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri"       => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # No runtime dependencies — pure Ruby, zero external gems required.

  spec.add_development_dependency "minitest",  "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake",      "~> 13.0"
end
