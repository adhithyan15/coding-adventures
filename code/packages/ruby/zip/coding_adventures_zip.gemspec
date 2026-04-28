# frozen_string_literal: true

require_relative "lib/coding_adventures/zip/version"

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_zip"
  spec.version = CodingAdventures::Zip::VERSION
  spec.summary = "ZIP archive format (PKZIP 1989) from scratch — CMP09"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.files   = Dir["lib/**/*.rb"]

  spec.require_paths         = ["lib"]
  spec.required_ruby_version = ">= 3.0"

  spec.add_dependency "coding_adventures_lzss", "~> 0.1"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake"
  spec.add_development_dependency "standard", "~> 1.0"
end
