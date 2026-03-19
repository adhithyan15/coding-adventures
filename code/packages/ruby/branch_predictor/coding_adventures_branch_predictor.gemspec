# frozen_string_literal: true

require_relative "lib/coding_adventures/branch_predictor/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_branch_predictor"
  spec.version       = CodingAdventures::BranchPredictor::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Branch prediction simulators built from first principles"
  spec.description   = "Static predictors (always-taken, BTFNT), dynamic predictors " \
                        "(1-bit, 2-bit saturating counter), and a Branch Target Buffer."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
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
