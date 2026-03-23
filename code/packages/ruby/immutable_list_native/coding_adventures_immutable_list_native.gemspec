# frozen_string_literal: true

require_relative "lib/coding_adventures/immutable_list_native/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_immutable_list_native"
  spec.version       = CodingAdventures::ImmutableListNative::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Rust-backed persistent immutable list with structural sharing"
  spec.description   = "A native extension wrapping the immutable-list Rust crate via ruby-bridge. " \
                        "Implements a 32-way trie persistent vector where push, set, and pop return " \
                        "new lists without modifying the original. Near-constant-time operations " \
                        "via structural sharing and a tail buffer optimization."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0.0"

  spec.files         = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rb,rs,toml}",
    "README.md",
    "CHANGELOG.md",
  ]
  spec.require_paths = ["lib"]
  spec.extensions    = ["ext/immutable_list_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
