# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module Fuzz
      # What one execution of the target did (VER06 section 5.1).
      class Outcome
        KINDS = %i[pass crash timeout oom].freeze

        attr_reader :kind, :crash_kind, :frames

        def initialize(kind, crash_kind = nil, frames = [])
          @kind = kind
          @crash_kind = crash_kind&.dup&.freeze
          @frames = frames.map { |f| f.dup.freeze }.freeze
          freeze
        end

        def self.pass = new(:pass)

        def self.timeout = new(:timeout)

        def self.oom = new(:oom)

        def self.crash(kind, frames = []) = new(:crash, kind, frames)

        def crash? = @kind == :crash

        def pass? = @kind == :pass

        # No payload, no message: kind and frames only.
        def inspect = "#<Fuzz::Outcome #{@kind}#{" #{@crash_kind}" if crash?}>"

        alias_method :to_s, :inspect
      end
    end
  end
end
