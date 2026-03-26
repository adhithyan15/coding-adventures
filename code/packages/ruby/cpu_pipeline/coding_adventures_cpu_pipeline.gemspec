# frozen_string_literal: true

require_relative "lib/coding_adventures/cpu_pipeline/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_cpu_pipeline"
  spec.version       = CodingAdventures::CpuPipeline::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Configurable N-stage CPU instruction pipeline simulator"
  spec.description   = "Simulates a CPU instruction pipeline (IF -> ID -> EX -> MEM -> WB) with stall, flush, and forwarding support. ISA-independent via callback injection."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.0.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
