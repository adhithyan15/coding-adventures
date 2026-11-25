# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Pbt
      # The closed failure vocabulary (VER05 5.1).  `property-timeout` is
      # registered but never produced: there is no wall-clock timeout, so a
      # property that loops forever hangs the run.
      module FailureIds
        COUNTEREXAMPLE = "property-counterexample"
        EXHAUSTED = "property-exhausted"
        TIMEOUT = "property-timeout"
        PANIC = "property-panic"
        ALL = [COUNTEREXAMPLE, EXHAUSTED, TIMEOUT, PANIC].freeze

        # True only for a String equal to one of the four registered ids.
        def self.pbt_failure_id?(candidate)
          candidate.is_a?(String) && ALL.include?(candidate)
        end
      end
    end
  end
end
