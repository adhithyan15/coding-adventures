# frozen_string_literal: true

require_relative "lib/coding_adventures/display/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_display"
  spec.version       = CodingAdventures::Display::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "VGA text-mode framebuffer display driver simulation"
  spec.description   = "Simulates a VGA text-mode framebuffer display with 80x25 default configuration, memory-mapped at 0xFFFB0000, with PutChar, Puts, Scroll, Clear, and cursor management."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
