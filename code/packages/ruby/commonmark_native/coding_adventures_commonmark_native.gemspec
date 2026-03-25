# frozen_string_literal: true

require_relative "lib/coding_adventures/commonmark_native/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_commonmark_native"
  spec.version       = CodingAdventures::CommonmarkNative::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Rust-backed CommonMark Markdown to HTML converter for Ruby"
  spec.description   = "A native extension wrapping the commonmark Rust crate via ruby-bridge. " \
                       "Converts CommonMark 0.31.2 Markdown to HTML with full spec compliance, " \
                       "including a safe variant that strips raw HTML to prevent XSS attacks."
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
  spec.extensions    = ["ext/commonmark_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri"    => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
