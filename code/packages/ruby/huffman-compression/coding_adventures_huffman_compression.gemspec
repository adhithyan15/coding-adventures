# frozen_string_literal: true

require_relative "lib/coding_adventures/huffman_compression/version"

Gem::Specification.new do |spec|
  spec.name    = "coding-adventures-huffman-compression"
  spec.version = CodingAdventures::HuffmanCompression::VERSION
  spec.summary = "Huffman compression (CMP04) — entropy coding using DT27 canonical Huffman tree"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.files   = Dir["lib/**/*.rb"]

  spec.require_paths      = ["lib"]
  spec.required_ruby_version = ">= 3.0"

  spec.add_dependency "coding-adventures-huffman-tree", "~> 0.1"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake"
  spec.add_development_dependency "standard", "~> 1.0"
end
