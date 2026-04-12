require_relative "lib/coding_adventures/lzss/version"

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_lzss"
  spec.version = CodingAdventures::LZSS::VERSION
  spec.summary = "LZSS lossless compression algorithm (1982) from scratch"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.files   = Dir["lib/**/*.rb"]

  spec.require_paths = ["lib"]
  spec.required_ruby_version = ">= 3.0"
end
