# frozen_string_literal: true

require_relative "lib/coding_adventures/markov_chain/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_markov_chain"
  spec.version       = CodingAdventures::MarkovChainVersion::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "General-purpose Markov Chain (DT28) built on directed-graph (DT01)"
  spec.description   = "A general-purpose Markov Chain library supporting order-k chains, " \
                        "Laplace/Lidstone smoothing, sequence generation, and stationary " \
                        "distribution via power iteration.  Built on the directed-graph " \
                        "package (DT01) for topology management."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.3.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Runtime dependencies
  spec.add_dependency "coding_adventures_directed_graph"
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
