# frozen_string_literal: true

module CodingAdventures
  module Conduit
    class Application
      def self.build(&block)
        new(&block)
      end

      def initialize(&block)
        @router = Router.new
        instance_eval(&block) if block
      end

      def get(pattern, &block)
        @router.add("GET", pattern, &block)
      end

      def call(env)
        @router.call(env)
      end
    end

    def self.app(&block)
      Application.build(&block)
    end
  end
end
