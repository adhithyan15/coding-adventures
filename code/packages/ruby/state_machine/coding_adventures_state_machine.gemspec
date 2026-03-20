# frozen_string_literal: true

require_relative "lib/coding_adventures/state_machine/version"

Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_state_machine"
  spec.version       = CodingAdventures::StateMachine::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "State machine implementations: DFA, NFA, PDA, minimization, and modal machines"
  spec.description   = "Formal automata theory in Ruby — deterministic and non-deterministic finite " \
                        "automata, pushdown automata, Hopcroft minimization, and modal state machines. " \
                        "Part of the coding-adventures computing stack."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files         = Dir["lib/**/*.rb", "README.md", "CHANGELOG.md", "LICENSE"]
  spec.require_paths = ["lib"]
  spec.metadata      = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }
  # Development dependencies
  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
