# frozen_string_literal: true

require "simplecov"
SimpleCov.start do
  add_filter %r{_tokens\\.rb$}
  add_filter %r{_grammar\\.rb$}
  add_filter "/test/"
  enable_coverage :branch
end

require "minitest/autorun"
require "coding_adventures_network_stack"
