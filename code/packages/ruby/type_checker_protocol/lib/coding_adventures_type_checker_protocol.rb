# frozen_string_literal: true

module CodingAdventures
  module TypeCheckerProtocol
    VERSION = "0.1.0"

    TypeErrorDiagnostic = Data.define(:message, :line, :column)
    TypeCheckResult = Data.define(:typed_ast, :errors, :ok)

    module TypeChecker
      def check(_ast)
        raise NotImplementedError, "#{self.class} must implement #check"
      end
    end

    class GenericTypeChecker
      NOT_HANDLED = Object.new.freeze

      def initialize(node_kind: nil, locate: nil)
        @hooks = Hash.new { |hash, key| hash[key] = [] }
        @errors = []
        @node_kind = node_kind
        @locate = locate || ->(_subject) { [1, 1] }
      end

      attr_reader :errors

      def reset
        @errors = []
        self
      end

      def register_hook(phase, kind, &hook)
        key_kind = kind == "*" ? "*" : normalize_kind(kind)
        @hooks["#{phase}:#{key_kind}"] << hook
        self
      end

      def dispatch(phase, node, *args)
        kind = @node_kind ? normalize_kind(@node_kind.call(node).to_s) : ""

        ["#{phase}:#{kind}", "#{phase}:*"].each do |key|
          @hooks[key].each do |hook|
            result = hook.call(node, *args)
            return result unless result.equal?(NOT_HANDLED)
          end
        end

        nil
      end

      def not_handled
        NOT_HANDLED
      end

      def error(message, subject)
        line, column = @locate.call(subject)
        @errors << TypeErrorDiagnostic.new(message: message, line: line, column: column)
        nil
      end

      def check(ast)
        reset
        run(ast)
        TypeCheckResult.new(typed_ast: ast, errors: @errors.dup, ok: @errors.empty?)
      end

      def run(_ast)
        nil
      end

      private

      def normalize_kind(kind)
        normalized = +""
        last_was_underscore = false

        kind.each_char do |char|
          if char.match?(/[[:alnum:]]/)
            normalized << char
            last_was_underscore = false
          elsif !last_was_underscore
            normalized << "_"
            last_was_underscore = true
          end
        end

        normalized.gsub(/\A_+|_+\z/, "")
      end
    end
  end
end
