# frozen_string_literal: true

require "simplecov"

SimpleCov.start do
  add_filter "/test/"
  minimum_coverage 90
end

require "minitest/autorun"
require "coding_adventures/document_ast_sanitizer"
