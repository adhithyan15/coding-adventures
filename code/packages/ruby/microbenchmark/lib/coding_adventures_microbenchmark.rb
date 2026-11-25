# frozen_string_literal: true

# Load the dependency first: analysis methods below refer to this public API.
require "coding_adventures_rank_statistics"
require "json"
require "rbconfig"

require_relative "coding_adventures/microbenchmark/version"
require_relative "coding_adventures/microbenchmark/model"
require_relative "coding_adventures/microbenchmark/analysis"
require_relative "coding_adventures/microbenchmark/collector"
require_relative "coding_adventures/microbenchmark/report"
require_relative "coding_adventures/microbenchmark/array_driver"
