# frozen_string_literal: true

# coding_adventures_stats.rb -- Descriptive statistics, frequency analysis,
# and cryptanalysis helpers.
#
# Overview
# ========
# This gem provides three categories of pure functions:
#
# 1. Descriptive statistics (mean, median, mode, variance, standard deviation,
#    min, max, range) -- operate on arrays of floats.
# 2. Frequency analysis (frequency_count, frequency_distribution, chi_squared,
#    chi_squared_text) -- operate on text strings or parallel arrays.
# 3. Cryptanalysis helpers (index_of_coincidence, entropy, ENGLISH_FREQUENCIES)
#    -- tools for breaking classical ciphers.
#
# Usage
# =====
#   require "coding_adventures_stats"
#
#   CodingAdventures::Stats::Descriptive.mean([1, 2, 3, 4, 5])  # => 3.0
#   CodingAdventures::Stats::Frequency.frequency_count("Hello")  # => {"H"=>1, ...}
#   CodingAdventures::Stats::Cryptanalysis.index_of_coincidence("AABB")  # => 0.333...

require_relative "coding_adventures/stats/version"
require_relative "coding_adventures/stats/descriptive"
require_relative "coding_adventures/stats/frequency"
require_relative "coding_adventures/stats/cryptanalysis"
