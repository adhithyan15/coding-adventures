# frozen_string_literal: true

require_relative "lib/coding_adventures/microbenchmark/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_microbenchmark"
  spec.version = CodingAdventures::Microbenchmark::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "Paired benchmark collection and replay"
  spec.description = "Deterministic paired measurements, exact report replay, and an indexed-array " \
                     "reference driver following VER07b and VER07d."
  spec.homepage = "https://github.com/adhithyan15/coding-adventures"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb", "bin/*", "testdata/*.json", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.bindir = "bin"
  spec.executables = ["array-microbenchmark"]
  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  spec.add_dependency "coding_adventures_rank_statistics", "~> 0.1.0"
  spec.add_development_dependency "coding_adventures_test_bootstrap", "~> 0.2.2"
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "standard", "~> 1.0"
end
