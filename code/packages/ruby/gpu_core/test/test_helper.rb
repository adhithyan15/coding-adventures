# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  add_filter %r{_tokens\\.rb$}
  add_filter %r{_grammar\\.rb$}
  add_filter "/test/"
  enable_coverage :branch
end

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "coding_adventures_gpu_core"
require "minitest/autorun"
