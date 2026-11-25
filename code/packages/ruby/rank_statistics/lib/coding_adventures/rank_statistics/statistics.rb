# frozen_string_literal: true

module CodingAdventures
  # Finite-sample order statistics with an exact binomial cutoff.
  module RankStatistics
    module_function

    # Convert only real numeric inputs. Complex values and custom objects are
    # rejected rather than silently taking a real component or calling to_f.
    def finite_samples(values)
      raise TypeError, "values must be an Array" unless values.is_a?(Array)
      raise ArgumentError, "values must not be empty" if values.empty?

      values.map do |value|
        unless value.is_a?(Numeric) && !value.is_a?(Complex)
          raise TypeError, "sample values must be real numbers"
        end

        number = Float(value)
        raise ArgumentError, "sample values must be finite" unless number.finite?

        number
      rescue RangeError, TypeError, ArgumentError => error
        raise error if error.message == "sample values must be finite"

        raise ArgumentError, "sample values must be finite binary64 numbers"
      end.sort
    end

    # Return the binary64 sample median without mutating the caller's array.
    def median(values)
      ordered = finite_samples(values)
      middle = ordered.length / 2
      return ordered[middle] if ordered.length.odd?

      low = ordered[middle - 1]
      high = ordered[middle]
      sum = low + high
      sum.finite? ? sum / 2.0 : low / 2.0 + high / 2.0
    end

    # Return the closed order-statistic interval, or nil when the sample is
    # too small to meet the requested exact confidence cutoff.
    def median_interval(values, confidence_percent)
      ordered = finite_samples(values)
      unless confidence_percent.instance_of?(Integer) && (1..99).cover?(confidence_percent)
        raise ArgumentError, "confidence_percent must be an integer from 1 through 99"
      end

      rank = exact_median_rank(ordered.length, confidence_percent)
      return nil if rank.zero?

      [ordered[rank - 1], ordered[ordered.length - rank]].freeze
    end

    # This exact recurrence keeps the running binomial coefficient and tail as
    # arbitrary-precision Integers; only the sample observations use Float.
    def exact_median_rank(sample_size, confidence_percent)
      denominator = 1 << sample_size
      coefficient = 1
      cumulative = 0
      largest_rank = 0

      0.upto(sample_size / 2) do |j|
        cumulative += coefficient
        break if 200 * cumulative > (100 - confidence_percent) * denominator

        largest_rank = j + 1
        coefficient = coefficient * (sample_size - j) / (j + 1)
      end

      largest_rank
    end
    private_class_method :exact_median_rank
  end
end
