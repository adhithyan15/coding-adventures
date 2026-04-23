# frozen_string_literal: true

require "simplecov"

SimpleCov.start do
  add_filter "/test/"
  add_filter "/ext/"
  minimum_coverage 70
end

require "minitest/autorun"
require "socket"
require_relative "../lib/coding_adventures_mini_redis_native"

