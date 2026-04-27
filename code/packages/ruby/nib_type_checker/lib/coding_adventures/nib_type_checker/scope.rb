# frozen_string_literal: true

module CodingAdventures
  module NibTypeChecker
    SymbolRecord = Data.define(
      :name,
      :nib_type,
      :is_fn,
      :fn_params,
      :fn_return_type,
      :is_const,
      :is_static
    ) do
      def initialize(name:, nib_type: nil, is_fn: false, fn_params: [], fn_return_type: nil, is_const: false, is_static: false)
        super
      end
    end

    class ScopeChain
      def initialize
        @global = {}
        @locals = []
      end

      def define_global(name, symbol)
        @global[name] = symbol
      end

      def push
        @locals << {}
      end

      def pop
        @locals.pop
      end

      def define_local(name, symbol)
        current = @locals.last
        if current.nil?
          define_global(name, symbol)
        else
          current[name] = symbol
        end
      end

      def lookup(name)
        @locals.reverse_each do |frame|
          return frame[name] if frame.key?(name)
        end
        @global[name]
      end
    end
  end
end
