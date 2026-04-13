# frozen_string_literal: true

require_relative "lib/coding_adventures/paint_codec_png_native/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_paint_codec_png_native"
  spec.version       = CodingAdventures::PaintCodecPngNative::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Rust-backed PNG codec bridge for Ruby"
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"

  spec.files         = Dir["lib/**/*.rb", "ext/**/*.{rb,rs,toml}", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.extensions    = ["ext/paint_codec_png_native/extconf.rb"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true",
  }

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
