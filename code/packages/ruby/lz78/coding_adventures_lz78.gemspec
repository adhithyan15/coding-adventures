require_relative "lib/coding_adventures/lz78/version"

Gem::Specification.new do |spec|
  spec.name    = "coding_adventures_lz78"
  spec.version = CodingAdventures::LZ78::VERSION
  spec.summary = "LZ78 lossless compression algorithm (1978) from scratch"
  spec.authors = ["Adhithya Rajasekaran"]
  spec.files   = Dir["lib/**/*.rb"]

  spec.require_paths = ["lib"]
  spec.required_ruby_version = ">= 3.0"
end
