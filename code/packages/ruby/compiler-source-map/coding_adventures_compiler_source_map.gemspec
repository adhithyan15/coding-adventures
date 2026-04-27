# frozen_string_literal: true

require_relative "lib/coding_adventures/compiler_source_map/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_compiler_source_map"
  spec.version       = CodingAdventures::CompilerSourceMap::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Source map chain sidecar for the AOT compiler pipeline"
  spec.description   = "Provides the multi-segment source map chain that flows through " \
                        "every stage of the AOT compiler: SourceToAst, AstToIr, IrToIr " \
                        "(per optimiser pass), and IrToMachineCode. Supports composite " \
                        "forward (source → machine code) and reverse (machine code → source) " \
                        "queries for debugging and profiling."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "rake", "~> 13.0"
end
