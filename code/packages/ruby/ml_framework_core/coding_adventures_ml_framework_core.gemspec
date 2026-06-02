# frozen_string_literal: true

require_relative "lib/coding_adventures/ml_framework_core/version"

# coding_adventures_ml_framework_core — idiomatic Ruby Tensor + autograd
# on top of the Rust matrix-cpu execution engine.
#
# Layered design — this gem (v0.1) ships only the bottom Tensor layer.
# PRs #5-#8 will layer on:
#   - PR #5: autograd engine (Function.apply, tensor.backward)
#   - PR #6: forward op dispatch (envelope-based calls into matrix_rust_ruby)
#   - PR #7: backward dispatch + end-to-end MLP test
#   - PR #8: benchmark + RubyGems publishing polish
#
# v0.1 is intentionally pure Ruby — no native ext, no Rust calls.  That
# keeps this PR small, reviewable, and independently testable.  The
# matrix_rust_ruby gem is listed as a runtime dependency so the dependency
# graph is correct from day one; we just don't *call* it until PR #6.
Gem::Specification.new do |spec|
  spec.name          = "coding_adventures_ml_framework_core"
  spec.version       = CodingAdventures::MLFrameworkCore::VERSION
  spec.authors       = ["Adhithya Rajasekaran"]
  spec.summary       = "Idiomatic Ruby Tensor and autograd on top of the Rust matrix-cpu engine"
  spec.description   = "Pure-Ruby Tensor class with factories (zeros/ones/eye/arange/randn/...), " \
                       "shape ops (reshape/transpose/flatten/squeeze/unsqueeze), and operator " \
                       "overloads (+/-/*///**/-). v0.1 is pure Ruby. Future versions dispatch " \
                       "large ops through matrix_rust_ruby to the Rust matrix-cpu engine."
  spec.homepage      = "https://github.com/adhithyan15/coding-adventures"
  spec.license       = "MIT"
  spec.required_ruby_version = ">= 2.6.0"

  spec.files         = Dir[
    "lib/**/*.rb",
    "README.md",
    "CHANGELOG.md"
  ]
  spec.require_paths = ["lib"]

  spec.metadata = {
    "source_code_uri" => "https://github.com/adhithyan15/coding-adventures",
    "rubygems_mfa_required" => "true"
  }

  # Runtime dep on the Rust binding gem.  Listed even though v0.1 doesn't
  # call into it — keeps the dependency graph honest, and means downstream
  # users only need one `gem install` to get the full eventual stack.
  spec.add_runtime_dependency "coding_adventures_matrix_rust_ruby", ">= 0.1"

  spec.add_development_dependency "minitest", "~> 5.0"
  spec.add_development_dependency "rake", "~> 13.0"
end
