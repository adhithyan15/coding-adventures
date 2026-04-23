# frozen_string_literal: true

require_relative "lib/coding_adventures/mini_redis_native/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_mini_redis_native"
  spec.version       = CodingAdventures::MiniRedisNative::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Ruby Mini Redis server backed by the Rust embeddable TCP runtime"
  spec.description   = "A Ruby native extension that wraps the Rust embeddable TCP server runtime and delegates Mini Redis application protocol jobs to a Ruby stdio worker."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0.0"

  spec.files         = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rb,rs,toml}",
    "README.md",
    "CHANGELOG.md"
  ]
  spec.require_paths = ["lib"]
  spec.extensions    = ["ext/mini_redis_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
end
