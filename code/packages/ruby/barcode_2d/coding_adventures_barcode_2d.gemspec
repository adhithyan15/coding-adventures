# frozen_string_literal: true

require_relative "lib/coding_adventures/barcode_2d/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_barcode_2d"
  spec.version       = CodingAdventures::Barcode2D::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Shared 2D barcode abstraction: ModuleGrid and layout() → PaintScene"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"      => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true",
  }

  spec.add_dependency "coding_adventures_paint_instructions", "~> 0.1"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
