# frozen_string_literal: true

require "simplecov"

SimpleCov.start do
  if respond_to?(:skip)
    skip "/test/"
  else
    add_filter "/test/"
  end
  minimum_coverage 80
end

require "minitest/autorun"
require_relative "../lib/coding_adventures_hamurabi"
