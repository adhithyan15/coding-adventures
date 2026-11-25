# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # Sabotage hooks: the engine entry points the corpus runner calls, as
      # swappable lambdas.  These are the only production indirections added for
      # the negative controls (VER06i section 10): a test replaces one with a broken
      # version, runs the corpus, and the runner must report problems.  Production
      # code never reassigns them (Ruby has no gem-private visibility, so this is a
      # documented test seam, as in Pbt::Hooks).
      module Hooks
        DEFAULTS = {
          bit_flip: ->(*a) { Mutate.bit_flip(*a) },
          byte_flip: ->(*a) { Mutate.byte_flip(*a) },
          byte_arithmetic: ->(*a) { Mutate.byte_arithmetic(*a) },
          boundary_insert: ->(*a) { Mutate.boundary_insert(*a) },
          block_delete: ->(*a) { Mutate.block_delete(*a) },
          block_insert: ->(*a) { Mutate.block_insert(*a) },
          block_duplicate: ->(*a) { Mutate.block_duplicate(*a) },
          splice: ->(*a) { Mutate.splice(*a) },
          mutate_sequence: ->(*a) { Mutate.mutate_sequence_counted(*a) },
          corpus_admit: ->(existing, trace) { Fuzz.corpus_admit(existing, trace) },
          duplicate_crash: ->(*a) { Signature.duplicate_crash?(*a) },
          failure_id: ->(id) { FailureIds.fuzz_failure_id?(id) },
          format_tap: ->(*a) { Signature.format_tap_failure(*a) },
          tfv_valid: ->(text) { Tfv.valid?(text) }
        }.freeze

        class << self
          DEFAULTS.each_key { |name| attr_accessor name }

          def reset
            DEFAULTS.each { |name, fn| public_send(:"#{name}=", fn) }
          end
        end

        reset
      end
    end
  end
end
