# frozen_string_literal: true

# Property-based testing engine for Ruby (VER05h).
#
# Require order matters (dependencies before the files that use them):
# the PRNG and limits first, then the budget tree, then generators, shrinking,
# the runner, and last the conformance corpus with its JSON and TFV readers.
require_relative "coding_adventures/test_bootstrap/version"
require_relative "coding_adventures/test_bootstrap/safe"
require_relative "coding_adventures/test_bootstrap/prng/splitmix64"
require_relative "coding_adventures/test_bootstrap/pbt/limits"
require_relative "coding_adventures/test_bootstrap/pbt/hooks"
require_relative "coding_adventures/test_bootstrap/pbt/failure_ids"
require_relative "coding_adventures/test_bootstrap/pbt/shrink_budget"
require_relative "coding_adventures/test_bootstrap/corpus_json/clip"
require_relative "coding_adventures/test_bootstrap/corpus_json/canon"
require_relative "coding_adventures/test_bootstrap/pbt/gen"
require_relative "coding_adventures/test_bootstrap/pbt/shrink"
require_relative "coding_adventures/test_bootstrap/pbt/gens"
require_relative "coding_adventures/test_bootstrap/pbt/runner"
require_relative "coding_adventures/test_bootstrap/corpus_json/strict_json"
require_relative "coding_adventures/test_bootstrap/corpus_json/values"
require_relative "coding_adventures/test_bootstrap/tfv/validator"
require_relative "coding_adventures/test_bootstrap/pbt/corpus"
