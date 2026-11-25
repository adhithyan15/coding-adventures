# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # The closed failure vocabulary (VER06 section 6.2).  Nothing else is ever
      # printed in a public failure line.
      module FailureIds
        CRASH = "fuzz-crash"
        TIMEOUT = "fuzz-timeout"
        OOM = "fuzz-oom"
        LEAK = "fuzz-leak"
        PANIC = "fuzz-panic"
        ALL = [CRASH, TIMEOUT, OOM, LEAK, PANIC].freeze

        #   fuzz_failure_id?("fuzz-crash")           # => true
        #   fuzz_failure_id?("fuzz-arbitrary-error") # => false
        #   fuzz_failure_id?(nil)                    # => false (not a string at all)
        def self.fuzz_failure_id?(candidate)
          candidate.is_a?(String) && ALL.include?(candidate)
        end
      end
    end
  end
end
