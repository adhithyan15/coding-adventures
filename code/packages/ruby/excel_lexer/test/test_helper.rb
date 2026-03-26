# frozen_string_literal: true

begin
  require "simplecov"
  SimpleCov.start do
    enable_coverage :branch
    minimum_coverage 80
  end
rescue LoadError
end

require "minitest/autorun"
require "coding_adventures_excel_lexer"
