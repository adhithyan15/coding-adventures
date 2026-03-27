# frozen_string_literal: true

begin
  require "simplecov"
  SimpleCov.start do
    enable_coverage :branch
    minimum_coverage 80
    # _grammar.rb is a generated data file — exclude from coverage
    add_filter "_grammar.rb"
  end
rescue LoadError
end

require "minitest/autorun"
require "coding_adventures_excel_parser"
