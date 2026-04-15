# frozen_string_literal: true

require_relative "lib/coding_adventures/huffman_tree/version"

Gem::Specification.new do |spec|
  spec.name    = "coding-adventures-huffman-tree"
  spec.version = CodingAdventures::HuffmanTree::VERSION
  spec.summary = "Huffman tree data structure (DT27) — greedy min-heap construction with canonical codes"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.files   = Dir["lib/**/*.rb"]

  spec.require_paths      = ["lib"]
  spec.required_ruby_version = ">= 3.0"

  spec.add_dependency "coding_adventures_heap", "~> 0.1"

  spec.add_development_dependency "rspec",      "~> 3.0"
  spec.add_development_dependency "simplecov",  "~> 0.22"
end
