# frozen_string_literal: true

require_relative "lib/coding_adventures/clr_simulator/version"

Gem::Specification.new do |spec|
  spec.name = "coding_adventures_clr_simulator"
  spec.version = CodingAdventures::ClrSimulator::VERSION
  spec.authors = ["Adhithya Rajasekaran"]
  spec.summary = "CLR IL bytecode simulator with real opcode values"
  spec.description = "Simulates CLR IL bytecode: ldc.i4 variants, ldloc/stloc, " \
    "add/sub/mul/div, br.s, brfalse.s, brtrue.s, ceq/cgt/clt, ret, nop."
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.4.0"
  spec.files = Dir["lib/**/*.rb"]
  spec.require_paths = ["lib"]

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "simplecov", "~> 0.22"
  spec.add_development_dependency "standard", "~> 1.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
