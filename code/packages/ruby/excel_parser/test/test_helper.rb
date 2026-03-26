# frozen_string_literal: true

begin
  require "simplecov"
  SimpleCov.start do
  add_filter %r{_tokens\\.rb$}
  add_filter %r{_grammar\\.rb$}
    enable_coverage :branch
    minimum_coverage 80
  end
rescue LoadError
end

require "minitest/autorun"
require "coding_adventures_excel_parser"
