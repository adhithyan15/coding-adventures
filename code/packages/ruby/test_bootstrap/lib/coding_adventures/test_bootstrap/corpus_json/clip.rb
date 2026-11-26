# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    module CorpusJson
      # Bounded fragments for reports: fixture-derived text must never make a
      # problem line (or a log) unboundedly large.
      module Clip
        def self.clip(text, limit)
          text = text.to_s
          (text.length > limit) ? "#{text[0, limit]}..." : text
        end
      end
    end
  end
end
