# frozen_string_literal: true

# coding_adventures/ml_framework_core — entry point
# =====================================================
#
# Require this file to get the public API:
#
#   require "coding_adventures/ml_framework_core"
#
#   t = CodingAdventures::MLFrameworkCore::Tensor.zeros(2, 3)
#   t.shape   # => [2, 3]
#   (t + 1.0).to_a   # => [1.0, 1.0, 1.0, 1.0, 1.0, 1.0]
#
# v0.1 ships only the Tensor class.  PRs #5-#8 will add `autograd.rb`,
# `ops.rb`, and the dispatch wiring; each will be a single new
# `require_relative` line in this file.

require_relative "ml_framework_core/version"
require_relative "ml_framework_core/tensor"
require_relative "ml_framework_core/autograd"
require_relative "ml_framework_core/ops"

# Short alias for callers who don't want to type the full namespace.
# Mirrors PyTorch's `import torch as t` convention.
MLFrameworkCore = CodingAdventures::MLFrameworkCore unless defined?(MLFrameworkCore)
