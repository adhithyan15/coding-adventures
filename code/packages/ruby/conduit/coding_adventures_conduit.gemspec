# frozen_string_literal: true

require_relative "lib/coding_adventures/conduit/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_conduit"
  spec.version       = CodingAdventures::Conduit::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "A tiny Rack-like Ruby layer backed by the Rust HTTP runtime"
  spec.description   = "Conduit provides a small Ruby app and router API on top of the Rust embeddable HTTP server."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 2.6.0"

  spec.files         = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rb,rs,toml}",
    "README.md",
    "CHANGELOG.md"
  ]
  spec.require_paths = ["lib"]
  spec.extensions    = ["ext/conduit_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
