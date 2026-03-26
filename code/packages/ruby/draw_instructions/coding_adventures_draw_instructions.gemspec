# frozen_string_literal: true

require_relative "lib/coding_adventures/draw_instructions/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_draw_instructions"
  spec.version       = CodingAdventures::DrawInstructions::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Backend-neutral 2D draw instructions for reusable scene generation"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri"        => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required"  => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
