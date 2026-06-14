# frozen_string_literal: true

require_relative "text_backend"

module CodingAdventures
  module CodegenCore
    class BackendRegistry
      DEFAULT_TARGETS = %i[pure_vm jvm clr wasm].freeze

      def self.default
        new.tap do |registry|
          DEFAULT_TARGETS.each { |target| registry.register(target, TextBackend.new(target)) }
        end
      end

      def initialize
        @backends = {}
      end

      def register(name, backend)
        @backends[name.to_sym] = backend
      end

      def fetch(name)
        @backends.fetch(name.to_sym) { raise KeyError, "unknown LANG backend #{name.inspect}" }
      end

      def compile(mod, target:)
        fetch(target).compile_module(mod)
      end

      def targets
        @backends.keys
      end
    end
  end
end
