# frozen_string_literal: true

require_relative "lib/coding_adventures/silicon_rust_ruby/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_silicon_rust_ruby"
  spec.version       = CodingAdventures::SiliconRustRuby::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Ruby bindings for the Rust silicon simulation stack"
  spec.description   = "Exposes device-physics, mosfet-models, and fab-process-simulation " \
                       "to Ruby via a zero-dependency native extension built with ruby-bridge " \
                       "(raw extern \"C\" Ruby C API — no Magnus, no rb-sys, no bindgen)."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 2.7.0"

  spec.files = Dir[
    "lib/**/*.rb",
    "ext/**/*.rb",
    "README.md",
    "CHANGELOG.md"
  ]
  spec.require_paths = ["lib"]

  spec.extensions = ["ext/silicon_rust_ruby_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri"       => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake",     "~> 13.0"
end
