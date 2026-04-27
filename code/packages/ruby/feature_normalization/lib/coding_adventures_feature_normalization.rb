# frozen_string_literal: true

require_relative "coding_adventures/feature_normalization/version"

module CodingAdventures
  module FeatureNormalization
    StandardScaler = Struct.new(:means, :standard_deviations, keyword_init: true)
    MinMaxScaler = Struct.new(:minimums, :maximums, keyword_init: true)

    module_function

    def fit_standard_scaler(rows)
      matrix = validate_matrix(rows)
      width = matrix.first.length
      means = Array.new(width, 0.0)
      matrix.each { |row| row.each_with_index { |value, col| means[col] += value } }
      means.map! { |value| value / matrix.length }

      standard_deviations = Array.new(width, 0.0)
      matrix.each do |row|
        row.each_with_index do |value, col|
          diff = value - means[col]
          standard_deviations[col] += diff * diff
        end
      end
      standard_deviations.map! { |value| Math.sqrt(value / matrix.length) }

      StandardScaler.new(means: means, standard_deviations: standard_deviations)
    end

    def transform_standard(rows, scaler)
      matrix = validate_matrix(rows)
      raise ArgumentError, "matrix width must match scaler width" unless matrix.first.length == scaler.means.length

      matrix.map do |row|
        row.each_with_index.map do |value, col|
          scaler.standard_deviations[col].zero? ? 0.0 : (value - scaler.means[col]) / scaler.standard_deviations[col]
        end
      end
    end

    def fit_min_max_scaler(rows)
      matrix = validate_matrix(rows)
      width = matrix.first.length
      minimums = Array.new(width) { |col| matrix.map { |row| row[col] }.min }
      maximums = Array.new(width) { |col| matrix.map { |row| row[col] }.max }
      MinMaxScaler.new(minimums: minimums, maximums: maximums)
    end

    def transform_min_max(rows, scaler)
      matrix = validate_matrix(rows)
      raise ArgumentError, "matrix width must match scaler width" unless matrix.first.length == scaler.minimums.length

      matrix.map do |row|
        row.each_with_index.map do |value, col|
          span = scaler.maximums[col] - scaler.minimums[col]
          span.zero? ? 0.0 : (value - scaler.minimums[col]) / span
        end
      end
    end

    def validate_matrix(rows)
      matrix = rows.map { |row| row.map(&:to_f) }
      raise ArgumentError, "matrix must have at least one row and one column" if matrix.empty? || matrix.first.empty?

      width = matrix.first.length
      raise ArgumentError, "all rows must have the same number of columns" if matrix.any? { |row| row.length != width }

      matrix
    end
    private_class_method :validate_matrix
  end
end
