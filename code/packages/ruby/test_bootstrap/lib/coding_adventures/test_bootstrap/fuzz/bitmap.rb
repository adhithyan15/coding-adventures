# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # A 65,536-bit execution map (VER06 section 4.1); bit e set means edge e
      # was executed.  Ids are reduced with floored `%` (collisions are the known
      # price of a fixed-size map; AFL pays the same one).
      class Bitmap
        def initialize
          @map = ("\x00" * Limits::COVERAGE_BYTES).b
          # Beside the map so listing one execution's edges costs O(edges hit).
          @edges = {}
        end

        def self.reduce(edge)
          Limits.check_int!(edge, "edge")
          edge % Limits::COVERAGE_MAP_SIZE
        end

        def record(edge)
          e = Bitmap.reduce(edge)
          @map.setbyte(e / 8, @map.getbyte(e / 8) | (1 << (e % 8)))
          @edges[e] = true
          nil
        end

        def has?(edge)
          e = Bitmap.reduce(edge)
          @map.getbyte(e / 8).anybits?(1 << (e % 8))
        end

        # OR a trace into this (cumulative) map; returns [has_new, count_new].
        def admit(trace)
          count_new = 0
          trace.each do |edge|
            e = Bitmap.reduce(edge)
            next if @edges.key?(e)

            record(e)
            count_new += 1
          end
          [count_new.positive?, count_new]
        end

        def covered_edges = @edges.keys.sort

        def count = @edges.size

        def merge(other)
          other.covered_edges.each { |e| record(e) }
          nil
        end

        def reset
          @map = ("\x00" * Limits::COVERAGE_BYTES).b
          @edges = {}
          nil
        end

        def raw = @map.dup

        def inspect = "#<Fuzz::Bitmap edges=#{@edges.size}>"

        alias_method :to_s, :inspect
      end

      # Pure novelty check: does `trace` hold an edge absent from `existing`?
      # Returns [admitted, new_edges] counting UNIQUE novel edges over the full,
      # unreduced ids.
      #
      #   corpus_admit([1, 2, 3], [2, 4])  # => [true, 1]
      #   corpus_admit([1, 2, 3], [1, 2])  # => [false, 0]
      def self.corpus_admit(existing, trace)
        novel = trace.to_set - existing.to_set
        [!novel.empty?, novel.size]
      end

      # Mutation slots for a seed: max(1, round(base * (1 + new/total) / sqrt(len))).
      # The edge bonus is 1.0 while nothing is covered.  Rounding is half away
      # from zero (floor(x + 0.5)) so every lane agrees on exact halves.
      #
      # base 10, len 4, 3 new of 6 total: 10 * 1.5 / 2 = 7.5 rounds to 8.
      def self.calculate_energy(base_energy, seed_len, new_edges, total_edges)
        base = [1, base_energy].max.to_f
        bonus = total_edges.positive? ? 1.0 + (new_edges.to_f / total_edges) : 1.0
        [1, ((base * bonus / Math.sqrt([1, seed_len].max)) + 0.5).floor].max
      end
    end
  end
end
