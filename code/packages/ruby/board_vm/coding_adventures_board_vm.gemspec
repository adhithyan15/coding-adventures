# frozen_string_literal: true

require_relative "lib/coding_adventures/board_vm/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_board_vm"
  spec.version = CodingAdventures::BoardVM::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Ruby DSL for Board VM hardware sessions"
  spec.description = "Provides a small Ruby DSL for flashing a Board VM runtime and driving LED blink sessions through the shared Rust host tools."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 2.6.0"
  spec.files = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
