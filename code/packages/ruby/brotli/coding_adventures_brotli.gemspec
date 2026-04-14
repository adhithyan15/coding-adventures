# frozen_string_literal: true

require_relative "lib/coding_adventures/brotli/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_brotli"
  spec.version = CodingAdventures::Brotli::VERSION
  spec.summary = "Brotli compression (CMP06) — context modeling + insert-copy + large window"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.files = Dir["lib/**/*.rb"]

  spec.require_paths = ["lib"]
  spec.required_ruby_version = ">= 3.0"

  spec.add_dependency "coding-adventures-huffman-tree", "~> 0.1"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake"
  spec.add_development_dependency "standard", "~> 1.0"
end
