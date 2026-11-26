# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # One deduplicated crash.  `minimal_input` is the shortest input known to
      # reproduce the signature; it is PRIVATE (VER06 section 6.3): never in
      # `inspect` or `to_s`, and public output goes through
      # Signature.format_tap_failure only.
      class CrashRecord
        attr_reader :signature, :kind, :frames, :minimal_input, :hits

        def initialize(signature:, kind:, frames:, minimal_input:, hits:, minimized:, input_dropped:)
          @signature = signature.dup.freeze
          @kind = kind.dup.freeze
          @frames = frames.map { |f| f.dup.freeze }.freeze
          @minimal_input = minimal_input.b.freeze
          @hits = hits
          @minimized = minimized
          @input_dropped = input_dropped
          freeze
        end

        # False when minimisation was skipped or cut short by a budget.
        def minimized? = @minimized

        # True when the retained-bytes cap stopped the input being kept.
        def input_dropped? = @input_dropped

        def inspect = "#<Fuzz::CrashRecord #{@signature} #{@kind} frames=#{@frames.size} hits=#{@hits}>"

        alias_method :to_s, :inspect
      end

      # What a campaign found.  Every execution lands in exactly one bucket:
      # `iterations` counts MUTATION executions; crashes/timeouts/ooms count triage
      # and mutation executions; anything that passed is in the corpus or was
      # redundant.  Collections are frozen copies; the seed and corpus never
      # appear in `inspect` or `to_s` (VER06 section 6).
      class Report
        TIER_BRANCHES = "ruby-coverage-branches"
        TIER_REPORTED = "caller-reported"
        TIER_BLACK_BOX = "black-box-uninstrumented"

        attr_reader :iterations, :admitted_seeds, :total_edges, :crashes, :timeouts, :ooms,
          :crashes_dropped, :admissions_dropped, :coverage_tier, :seed, :corpus

        def initialize(iterations:, admitted_seeds:, total_edges:, crashes:, timeouts:, ooms:,
          crashes_dropped:, admissions_dropped:, coverage_tier:, seed:, corpus:)
          @iterations = iterations
          @admitted_seeds = admitted_seeds
          @total_edges = total_edges
          @crashes = crashes.dup.freeze
          @timeouts = timeouts
          @ooms = ooms
          @crashes_dropped = crashes_dropped
          @admissions_dropped = admissions_dropped
          @coverage_tier = coverage_tier
          @seed = seed
          @corpus = corpus.map { |s| s.b.freeze }.freeze
          freeze
        end

        # Can this be read as "the campaign found nothing"?  Only if at least one
        # mutation iteration ran and nothing crashed, timed out, ran out of memory
        # or was dropped.
        def clean?
          @iterations.positive? && @crashes.empty? && @timeouts.zero? && @ooms.zero? && @crashes_dropped.zero?
        end

        def unique_crashes = @crashes.size + @crashes_dropped

        def inspect
          "#<Fuzz::Report iterations=#{@iterations} seeds=#{@admitted_seeds} edges=#{@total_edges} " \
            "crashes=#{unique_crashes} timeouts=#{@timeouts} ooms=#{@ooms} tier=#{@coverage_tier}>"
        end

        alias_method :to_s, :inspect
      end
    end
  end
end
