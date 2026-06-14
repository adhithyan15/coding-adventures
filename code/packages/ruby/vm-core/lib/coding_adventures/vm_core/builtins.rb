# frozen_string_literal: true

module CodingAdventures
  module VmCore
    class BuiltinRegistry
      def initialize
        @handlers = {}
        register("noop") { |_args| nil }
        register("assert_eq") do |args|
          raise VMError, "assert_eq failed: #{args[0].inspect} != #{args[1].inspect}" unless args[0] == args[1]
          true
        end
      end

      def register(name, callable = nil, &block)
        @handlers[name] = callable || block
      end

      def call(name, args)
        handler = @handlers[name]
        raise VMError, "unknown builtin #{name.inspect}" unless handler

        handler.call(args)
      end

      def names
        @handlers.keys
      end
    end
  end
end
