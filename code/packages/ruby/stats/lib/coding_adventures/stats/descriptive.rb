# frozen_string_literal: true

# Descriptive statistics -- scalar functions operating on arrays of floats.
#
# Each function takes an array of numbers and returns a single float summary.
# These are the building blocks of statistical analysis: they tell you about
# the center (mean, median, mode), spread (variance, standard deviation, range),
# and boundaries (min, max) of a dataset.
#
# Worked Example
# ==============
# Given the dataset [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]:
#
#   mean     = (2+4+4+4+5+5+7+9) / 8 = 40 / 8 = 5.0
#   median   = average of 4th and 5th values (sorted) = (4+5)/2 = 4.5
#   mode     = 4.0 (appears 3 times, more than any other)
#   variance = sample: sum of squared deviations / (n-1)
#            = [(2-5)^2 + (4-5)^2 + ... + (9-5)^2] / 7
#            = 32 / 7 = 4.571428...

module CodingAdventures
  module Stats
    module Descriptive
      module_function

      # Arithmetic mean: sum of all values divided by the count.
      #
      # The mean is the most common measure of central tendency. It uses every
      # data point, which makes it sensitive to outliers.
      #
      # Formula: mean = (x_1 + x_2 + ... + x_n) / n
      def mean(values)
        raise ArgumentError, "mean requires at least one value" if values.empty?

        values.sum.to_f / values.length
      end

      # Median: the middle value when sorted.
      #
      # For odd-length arrays, the median is the middle element.
      # For even-length arrays, it is the average of the two middle elements.
      #
      # The median is robust to outliers -- unlike the mean, extreme values
      # do not pull it away from the center.
      def median(values)
        raise ArgumentError, "median requires at least one value" if values.empty?

        sorted = values.sort
        n = sorted.length
        mid = n / 2

        # Odd length: single middle element.
        if n.odd?
          sorted[mid].to_f
        else
          # Even length: average of two middle elements.
          (sorted[mid - 1] + sorted[mid]) / 2.0
        end
      end

      # Mode: the most frequently occurring value.
      #
      # If multiple values share the highest frequency, the one that appears
      # first in the original array wins. This "first occurrence" tie-breaking
      # rule ensures deterministic results across all languages.
      def mode(values)
        raise ArgumentError, "mode requires at least one value" if values.empty?

        # Step 1: count occurrences.
        counts = Hash.new(0)
        values.each { |v| counts[v] += 1 }

        # Step 2: find the maximum frequency.
        max_count = counts.values.max

        # Step 3: return the first value with that frequency.
        values.find { |v| counts[v] == max_count }.to_f
      end

      # Variance: average of squared deviations from the mean.
      #
      # Two flavors:
      #   - Sample variance (population: false, default): divides by n-1.
      #     Uses Bessel's correction for unbiased estimation.
      #   - Population variance (population: true): divides by n.
      #
      # Formula: variance = Sum((x_i - mean)^2) / d
      # where d = n (population) or n-1 (sample)
      def variance(values, population: false)
        raise ArgumentError, "variance requires at least one value" if values.empty?

        n = values.length
        if n == 1 && !population
          raise ArgumentError, "sample variance requires at least two values"
        end

        m = mean(values)

        # Sum of squared deviations from the mean.
        sum_sq = values.sum { |x| (x - m)**2 }

        divisor = population ? n : (n - 1)
        sum_sq / divisor.to_f
      end

      # Standard deviation: square root of variance.
      #
      # It has the same units as the original data, making it more
      # interpretable than variance.
      def standard_deviation(values, population: false)
        Math.sqrt(variance(values, population: population))
      end

      # Minimum value in the dataset.
      def min(values)
        raise ArgumentError, "min requires at least one value" if values.empty?

        values.min.to_f
      end

      # Maximum value in the dataset.
      def max(values)
        raise ArgumentError, "max requires at least one value" if values.empty?

        values.max.to_f
      end

      # Range: max - min, the simplest measure of spread.
      def range(values)
        max(values) - min(values)
      end
    end
  end
end
