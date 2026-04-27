require_relative "lib/coding_adventures/lzw/version"

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_lzw"
  spec.version = CodingAdventures::LZW::VERSION
  spec.summary = "LZW lossless compression algorithm (1984) from scratch"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.files   = Dir["lib/**/*.rb"]

  spec.require_paths = ["lib"]
  spec.required_ruby_version = ">= 3.0"
end
