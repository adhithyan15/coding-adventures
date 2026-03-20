# frozen_string_literal: true

require "prism"

# ============================================================================
# Banned Construct Detector — Finding Dynamic Execution in Ruby
# ============================================================================
#
# This module detects dynamic execution constructs that are banned outright
# in the capability security system. These constructs are the primary
# mechanism for evading static analysis.
#
# ## Why Ban These Constructs?
#
# Consider an attacker trying to exfiltrate data from a package that
# declares zero network capabilities. They can't write:
#
#     require "socket"  # caught by the capability analyzer
#
# So they try:
#
#     send(:require, "socket")     # dynamic dispatch — evades analyzer!
#     eval('require "socket"')     # string evaluation — evades analyzer!
#     Kernel.const_get(:TCPSocket) # dynamic constant — evades analyzer!
#
# By banning eval, dynamic send, and similar constructs, we force
# attackers to use direct requires, which the capability analyzer catches.
#
# ## Banned Constructs in Ruby
#
# Ruby is an extraordinarily dynamic language. Unlike Python, where the
# main evasion vectors are eval/exec/__import__, Ruby has many more:
#
# | Construct             | Why Dangerous                              |
# |-----------------------|--------------------------------------------|
# | eval(...)             | Executes arbitrary Ruby from a string      |
# | instance_eval(...)    | Evaluates string/block in object context   |
# | class_eval(...)       | Evaluates string/block in class context    |
# | module_eval(...)      | Same as class_eval                         |
# | Binding.eval(...)     | Eval with access to local variables        |
# | send(:sym, ...)       | Dynamic dispatch — calls any method        |
# | public_send(:sym,...) | Like send but respects visibility          |
# | Object.const_get(...) | Dynamic constant lookup — loads any class  |
# | define_method(...)    | Defines methods dynamically                |
# | method_missing(...)   | Intercepts undefined method calls          |
# | system("cmd")         | With interpolation — injection risk        |
# | `cmd`                 | Backtick execution — subprocess            |
# | %x{cmd}              | Same as backticks                          |
# | require(var)          | Dynamic require — loads any library        |
#
# ## Exception Process
#
# If a package genuinely needs a banned construct (e.g., a template
# engine that uses eval), it must declare the exception in
# `required_capabilities.json` under `banned_construct_exceptions`
# with a justification.
# ============================================================================

module CA
  module CapabilityAnalyzer
    # A banned dynamic execution construct found in source code.
    #
    # This is an "evidence record" for security violations — constructs
    # that are forbidden regardless of what capabilities a package declares.
    BannedConstructViolation = Struct.new(
      :construct, # The name of the banned construct (e.g., "eval")
      :file,      # The source file where the violation was found
      :line,      # The line number
      :evidence,  # The code pattern that triggered the violation
      keyword_init: true
    ) do
      def to_s
        "BANNED #{construct} at #{file}:#{line}: #{evidence}"
      end
    end

    # ── Banned bare method names ──────────────────────────────────────
    #
    # These are method names that are always banned when called without
    # a receiver (i.e., as Kernel methods). `eval` is the most dangerous
    # because it can execute arbitrary Ruby code from a string.
    BANNED_BARE_METHODS = %w[eval].to_set.freeze

    # ── Banned receiver+method patterns ───────────────────────────────
    #
    # These are [receiver_class, method_name] pairs that are banned.
    # For example, Binding.eval allows eval with access to local scope.
    BANNED_CLASS_METHODS = [
      %w[Binding eval],
      %w[Kernel eval],
      %w[Kernel exec],
      %w[Kernel system]
    ].to_set.freeze

    # ── Eval-family methods ───────────────────────────────────────────
    #
    # These methods accept a string and evaluate it as Ruby code.
    # They're dangerous because the string content can't be analyzed
    # statically. We flag them when called with a string argument
    # (since block form is less dangerous — the code is visible in AST).
    EVAL_FAMILY = %w[
      instance_eval
      class_eval
      module_eval
    ].to_set.freeze

    # ── Dynamic dispatch methods ──────────────────────────────────────
    #
    # `send` and `public_send` call any method by name. When the method
    # name is a literal symbol (e.g., `send(:to_s)`), it's equivalent
    # to a direct call and not dangerous. But when it's a variable
    # (e.g., `send(user_input)`), any method could be called.
    DYNAMIC_DISPATCH_METHODS = %w[send public_send __send__].to_set.freeze

    class BannedConstructDetector
      attr_reader :violations, :filename

      def initialize(filename)
        @filename = filename
        @violations = []
      end

      # Scan Ruby source code for banned constructs.
      #
      # @param source [String] Ruby source code to scan.
      # @return [Array<BannedConstructViolation>] violations found.
      def detect(source)
        result = Prism.parse(source)
        walk(result.value)
        @violations
      end

      private

      # Record a banned construct violation.
      def add(construct, line, evidence)
        @violations << BannedConstructViolation.new(
          construct: construct,
          file: @filename,
          line: line,
          evidence: evidence
        )
      end

      # ── AST Walking ──────────────────────────────────────────────────
      #
      # We walk the tree recursively, just like the capability analyzer.
      # At each node, we check for banned patterns.

      def walk(node)
        return unless node.is_a?(Prism::Node)

        case node
        when Prism::CallNode
          check_call(node)
        when Prism::XStringNode
          # Backtick string: `cmd`
          # Backticks execute shell commands. They're the Ruby equivalent
          # of system() but with implicit string interpolation risk.
          add("backtick_execution", node.location.start_line, "`...` (backtick execution)")
        when Prism::InterpolatedXStringNode
          # Interpolated backtick: `cmd #{expr}`
          # Even more dangerous — the command includes runtime values.
          add("backtick_execution", node.location.start_line, "`...\#{...}` (interpolated backtick)")
        when Prism::DefNode
          # Check if this defines method_missing
          check_method_missing_definition(node)
        end

        # Visit all child nodes
        node.child_nodes.each do |child|
          walk(child) if child
        end
      end

      # ── Call Node Checks ─────────────────────────────────────────────

      def check_call(node)
        check_banned_bare_methods(node)
        check_eval_family(node)
        check_dynamic_dispatch(node)
        check_dynamic_require(node)
        check_const_get(node)
        check_define_method(node)
        check_banned_class_methods(node)
        check_system_interpolation(node)
      end

      # ── Bare eval Detection ──────────────────────────────────────────
      #
      # `eval("code")` with no receiver is the most straightforward
      # way to execute arbitrary Ruby code from a string.

      def check_banned_bare_methods(node)
        return unless node.receiver.nil?

        method_name = node.name.to_s
        return unless BANNED_BARE_METHODS.include?(method_name)

        add(method_name, node.location.start_line, "#{method_name}(...)")
      end

      # ── Eval Family Detection ────────────────────────────────────────
      #
      # instance_eval, class_eval, and module_eval can all accept a
      # string argument that gets evaluated as Ruby code. When called
      # with a string (not a block), they're effectively eval().
      #
      # We flag these when the first argument is a string OR when
      # the first argument is non-literal (variable/expression).

      def check_eval_family(node)
        method_name = node.name.to_s
        return unless EVAL_FAMILY.include?(method_name)

        # If called with a string argument, it's banned
        if node.arguments
          first_arg = node.arguments.arguments.first
          if first_arg.is_a?(Prism::StringNode) || first_arg.is_a?(Prism::InterpolatedStringNode)
            add(method_name, node.location.start_line, "#{method_name}(\"...\")")
            return
          end
        end

        # If called with a block, it's less dangerous (code is in AST),
        # but we still flag it for review when there's no block and
        # a non-literal argument (could be a variable holding code).
        if node.arguments && !node.block
          add(method_name, node.location.start_line, "#{method_name}(<dynamic>)")
        end
      end

      # ── Dynamic Dispatch Detection ───────────────────────────────────
      #
      # `obj.send(:method_name)` with a literal symbol is equivalent to
      # `obj.method_name` — safe and analyzable. But `obj.send(variable)`
      # could call ANY method, making it an evasion vector.
      #
      # We only flag send/public_send when the first argument is NOT
      # a literal symbol or string.

      def check_dynamic_dispatch(node)
        method_name = node.name.to_s
        return unless DYNAMIC_DISPATCH_METHODS.include?(method_name)
        return unless node.arguments

        first_arg = node.arguments.arguments.first
        return unless first_arg

        # Literal symbol or string arguments are safe — we can see
        # exactly which method will be called.
        return if first_arg.is_a?(Prism::SymbolNode)
        return if first_arg.is_a?(Prism::StringNode)

        add(
          "dynamic_#{method_name}",
          node.location.start_line,
          "#{method_name}(<dynamic>) — non-literal method name"
        )
      end

      # ── Dynamic Require Detection ────────────────────────────────────
      #
      # `require "socket"` with a literal string is handled by the
      # capability analyzer. But `require(variable)` could load ANY
      # library, completely evading static analysis.

      def check_dynamic_require(node)
        return unless node.receiver.nil?
        return unless %i[require require_relative].include?(node.name)
        return unless node.arguments

        first_arg = node.arguments.arguments.first
        return unless first_arg

        # Literal string is fine — handled by capability analyzer
        return if first_arg.is_a?(Prism::StringNode)

        add(
          "dynamic_require",
          node.location.start_line,
          "#{node.name}(<dynamic>) — non-literal library name"
        )
      end

      # ── Object.const_get Detection ───────────────────────────────────
      #
      # `Object.const_get(:TCPSocket)` can load any class by name.
      # With a literal argument it's analyzable; with a variable it
      # evades static analysis.

      def check_const_get(node)
        return unless node.name == :const_get
        return unless node.arguments

        first_arg = node.arguments.arguments.first
        return unless first_arg

        # Literal symbol or string — we can see the constant name
        return if first_arg.is_a?(Prism::SymbolNode)
        return if first_arg.is_a?(Prism::StringNode)

        receiver_name = if node.receiver.is_a?(Prism::ConstantReadNode)
          node.receiver.name.to_s
        else
          "<receiver>"
        end

        add(
          "dynamic_const_get",
          node.location.start_line,
          "#{receiver_name}.const_get(<dynamic>) — non-literal constant name"
        )
      end

      # ── define_method Detection ──────────────────────────────────────
      #
      # `define_method(:name) { ... }` with a literal name is fine.
      # `define_method(variable) { ... }` creates a method whose name
      # can't be determined statically.

      def check_define_method(node)
        return unless node.name == :define_method
        return unless node.arguments

        first_arg = node.arguments.arguments.first
        return unless first_arg

        # Literal symbol or string — method name is known
        return if first_arg.is_a?(Prism::SymbolNode)
        return if first_arg.is_a?(Prism::StringNode)

        add(
          "dynamic_define_method",
          node.location.start_line,
          "define_method(<dynamic>) — non-literal method name"
        )
      end

      # ── Banned Class Method Detection ────────────────────────────────
      #
      # Some class-level method calls are banned outright, like
      # Binding.eval and Kernel.eval.

      def check_banned_class_methods(node)
        return unless node.receiver.is_a?(Prism::ConstantReadNode)

        key = [node.receiver.name.to_s, node.name.to_s]
        return unless BANNED_CLASS_METHODS.include?(key)

        add(
          "#{key[0]}.#{key[1]}",
          node.location.start_line,
          "#{key[0]}.#{key[1]}(...)"
        )
      end

      # ── System with Interpolation Detection ──────────────────────────
      #
      # `system("rm -rf #{path}")` is a command injection vulnerability.
      # We flag system/exec calls where the argument contains string
      # interpolation.

      def check_system_interpolation(node)
        return unless node.receiver.nil?
        return unless %i[system exec].include?(node.name)
        return unless node.arguments

        first_arg = node.arguments.arguments.first
        return unless first_arg.is_a?(Prism::InterpolatedStringNode)

        add(
          "#{node.name}_interpolation",
          node.location.start_line,
          "#{node.name}(\"...\#{...}\") — string interpolation in shell command"
        )
      end

      # ── method_missing Definition Detection ──────────────────────────
      #
      # Defining method_missing allows a class to intercept any method
      # call. This makes static analysis unreliable because the behavior
      # of any method call on that object is unknowable without running
      # the code.

      def check_method_missing_definition(node)
        return unless node.name == :method_missing

        add(
          "method_missing_definition",
          node.location.start_line,
          "def method_missing(...) — intercepts undefined method calls"
        )
      end
    end

    # ── Module-Level Convenience Methods ─────────────────────────────

    # Scan a single Ruby file for banned constructs.
    #
    # @param filepath [String] path to the Ruby source file.
    # @return [Array<BannedConstructViolation>] violations found.
    def self.detect_banned(filepath)
      source = File.read(filepath)
      detector = BannedConstructDetector.new(filepath)
      detector.detect(source)
    end

    # Scan all Ruby files in a directory for banned constructs.
    #
    # @param directory [String] root directory to scan.
    # @return [Array<BannedConstructViolation>] all violations found.
    def self.detect_banned_in_directory(directory)
      skip_dirs = %w[.git vendor node_modules .bundle]
      all_violations = []

      Dir.glob(File.join(directory, "**", "*.rb")).each do |rb_file|
        parts = rb_file.split(File::SEPARATOR)
        next if parts.any? { |part| skip_dirs.include?(part) }

        begin
          violations = detect_banned(rb_file)
          all_violations.concat(violations)
        rescue => _e
          nil
        end
      end

      all_violations
    end
  end
end
