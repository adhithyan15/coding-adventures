# frozen_string_literal: true

require "prism"

# ============================================================================
# Capability Analyzer — AST-Based Capability Detection for Ruby
# ============================================================================
#
# This module walks a Ruby AST (parsed by Prism) to detect OS-level
# capability usage. Each detected pattern maps to a capability string
# in the format `category:action:target`.
#
# ## How Prism AST Walking Works
#
# Prism parses Ruby source into a tree of typed nodes. For example:
#
#     require "socket"
#     TCPSocket.new("example.com", 80)
#
# Produces a tree like:
#
#     ProgramNode
#     +-- StatementsNode
#         +-- CallNode(name: :require, arguments: ["socket"])
#         +-- CallNode(
#               receiver: ConstantReadNode(name: :TCPSocket),
#               name: :new,
#               arguments: ["example.com", 80])
#
# We walk this tree looking for CallNode patterns that match known
# capability-bearing operations.
#
# ## Detection Strategy
#
# Rather than implementing a full Prism::Visitor subclass, we use a
# recursive tree walker that checks each node. This is simpler and
# easier to understand, at the cost of being slightly less idiomatic.
#
# The walker looks for three kinds of patterns:
#
# 1. **Import detection**: `require "socket"` → net:*:*
#    When Ruby code requires a module, the module name tells us what
#    OS capability the code might use.
#
# 2. **Class method calls**: `File.read("x")` → fs:read:x
#    When code calls a method on a known class (File, Dir, IO, ENV),
#    we map the class+method to a capability.
#
# 3. **Bare function calls**: `system("cmd")` → proc:exec:*
#    Some Ruby methods are called without a receiver (system, exec,
#    spawn). These are Kernel methods available everywhere.
#
# ## Capability Format
#
# Each capability is a triple: `category:action:target`
#
#   - category: fs, net, proc, env, ffi
#   - action: read, write, delete, list, create, connect, exec, etc.
#   - target: specific resource ("file.txt") or "*" (unknown/any)
#
# When the target is a string literal, we record the exact value.
# When it's a variable or expression, we record "*" because we can't
# determine the value statically.
# ============================================================================

module CA
  module CapabilityAnalyzer
    # A single OS capability detected in source code.
    #
    # Think of this as an "evidence record" — it says "I found code at
    # line X in file Y that uses capability Z, and here's the proof."
    DetectedCapability = Struct.new(
      :category,  # The kind of resource (fs, net, proc, env, ffi)
      :action,    # The operation (read, write, connect, exec, etc.)
      :target,    # The specific resource ("file.txt", "*")
      :file,      # The source file where detection occurred
      :line,      # The line number in the source file
      :evidence,  # The code pattern that triggered detection
      keyword_init: true
    ) do
      def to_s
        "#{category}:#{action}:#{target}"
      end

      def to_h
        {
          category: category,
          action: action,
          target: target,
          file: file,
          line: line,
          evidence: evidence
        }
      end
    end

    # ── Import-to-capability mapping ──────────────────────────────────
    #
    # When Ruby code requires a library, the library name tells us what
    # OS capability the code might use. This mapping is conservative:
    # requiring "socket" doesn't mean the code opens a connection, but
    # it *could*. We flag it and let the manifest comparison decide.
    #
    # The keys are the argument to `require`. The values are
    # [category, action] pairs.
    IMPORT_CAPABILITIES = {
      # Filesystem access
      "fileutils" => ["fs", "*"],
      "tempfile" => ["fs", "write"],
      "pathname" => ["fs", "*"],
      "find" => ["fs", "list"],

      # Network access
      "socket" => ["net", "*"],
      "net/http" => ["net", "connect"],
      "net/https" => ["net", "connect"],
      "net/ftp" => ["net", "connect"],
      "net/smtp" => ["net", "connect"],
      "net/pop" => ["net", "connect"],
      "net/imap" => ["net", "connect"],
      "open-uri" => ["net", "connect"],
      "uri" => ["net", "*"],

      # Process execution
      "open3" => ["proc", "exec"],
      "shellwords" => ["proc", "exec"],
      "pty" => ["proc", "exec"],

      # Environment
      "etc" => ["env", "read"],

      # FFI / native code
      "fiddle" => ["ffi", "*"],
      "ffi" => ["ffi", "*"],
      "dl" => ["ffi", "*"]
    }.freeze

    # ── Class method-to-capability mapping ────────────────────────────
    #
    # Beyond imports, specific method calls on known classes indicate
    # capability usage. For example, `File.read("x")` indicates
    # filesystem read capability.
    #
    # The keys are [class_name, method_name] pairs. The values are
    # [category, action] pairs.
    CLASS_METHOD_CAPABILITIES = {
      # File class — the primary way to interact with files in Ruby.
      # File inherits from IO, so many IO methods are available too.
      ["File", "open"] => ["fs", "read"],
      ["File", "read"] => ["fs", "read"],
      ["File", "readlines"] => ["fs", "read"],
      ["File", "binread"] => ["fs", "read"],
      ["File", "foreach"] => ["fs", "read"],
      ["File", "write"] => ["fs", "write"],
      ["File", "binwrite"] => ["fs", "write"],
      ["File", "delete"] => ["fs", "delete"],
      ["File", "unlink"] => ["fs", "delete"],
      ["File", "rename"] => ["fs", "write"],
      ["File", "chmod"] => ["fs", "write"],
      ["File", "chown"] => ["fs", "write"],
      ["File", "symlink"] => ["fs", "create"],
      ["File", "link"] => ["fs", "create"],
      ["File", "stat"] => ["fs", "read"],
      ["File", "lstat"] => ["fs", "read"],
      ["File", "exist?"] => ["fs", "read"],
      ["File", "exists?"] => ["fs", "read"],
      ["File", "size"] => ["fs", "read"],
      ["File", "directory?"] => ["fs", "read"],
      ["File", "file?"] => ["fs", "read"],
      ["File", "expand_path"] => ["fs", "read"],

      # Dir class — directory operations.
      ["Dir", "glob"] => ["fs", "list"],
      ["Dir", "entries"] => ["fs", "list"],
      ["Dir", "foreach"] => ["fs", "list"],
      ["Dir", "children"] => ["fs", "list"],
      ["Dir", "each_child"] => ["fs", "list"],
      ["Dir", "mkdir"] => ["fs", "create"],
      ["Dir", "rmdir"] => ["fs", "delete"],
      ["Dir", "delete"] => ["fs", "delete"],
      ["Dir", "exist?"] => ["fs", "read"],
      ["Dir", "home"] => ["env", "read"],

      # IO class — low-level I/O operations.
      ["IO", "read"] => ["fs", "read"],
      ["IO", "write"] => ["fs", "write"],
      ["IO", "readlines"] => ["fs", "read"],
      ["IO", "foreach"] => ["fs", "read"],
      ["IO", "binread"] => ["fs", "read"],
      ["IO", "binwrite"] => ["fs", "write"],
      ["IO", "copy_stream"] => ["fs", "write"],

      # ENV — environment variable access.
      # ENV["KEY"] is handled separately (subscript detection).
      # These are method-style accesses.
      ["ENV", "fetch"] => ["env", "read"],
      ["ENV", "keys"] => ["env", "read"],
      ["ENV", "values"] => ["env", "read"],
      ["ENV", "to_h"] => ["env", "read"],
      ["ENV", "each"] => ["env", "read"],
      ["ENV", "select"] => ["env", "read"],

      # FileUtils — high-level file operations (from fileutils stdlib).
      ["FileUtils", "rm"] => ["fs", "delete"],
      ["FileUtils", "rm_r"] => ["fs", "delete"],
      ["FileUtils", "rm_rf"] => ["fs", "delete"],
      ["FileUtils", "rmdir"] => ["fs", "delete"],
      ["FileUtils", "cp"] => ["fs", "write"],
      ["FileUtils", "cp_r"] => ["fs", "write"],
      ["FileUtils", "mv"] => ["fs", "write"],
      ["FileUtils", "mkdir"] => ["fs", "create"],
      ["FileUtils", "mkdir_p"] => ["fs", "create"],
      ["FileUtils", "touch"] => ["fs", "write"],
      ["FileUtils", "chmod"] => ["fs", "write"],
      ["FileUtils", "chown"] => ["fs", "write"],
      ["FileUtils", "install"] => ["fs", "write"],
      ["FileUtils", "ln"] => ["fs", "create"],
      ["FileUtils", "ln_s"] => ["fs", "create"],

      # Network classes — socket and HTTP access.
      ["TCPSocket", "new"] => ["net", "connect"],
      ["TCPSocket", "open"] => ["net", "connect"],
      ["UDPSocket", "new"] => ["net", "connect"],
      ["UDPSocket", "open"] => ["net", "connect"],
      ["TCPServer", "new"] => ["net", "listen"],
      ["TCPServer", "open"] => ["net", "listen"],
      ["Socket", "new"] => ["net", "*"],
      ["Socket", "tcp"] => ["net", "connect"],
      ["Socket", "unix"] => ["net", "connect"],

      # Process class — process management.
      ["Process", "spawn"] => ["proc", "exec"],
      ["Process", "exec"] => ["proc", "exec"],
      ["Process", "fork"] => ["proc", "fork"],
      ["Process", "kill"] => ["proc", "signal"],
      ["Process", "wait"] => ["proc", "wait"],
      ["Process", "waitpid"] => ["proc", "wait"]
    }.freeze

    # ── Bare method-to-capability mapping ─────────────────────────────
    #
    # Some Ruby methods are called without a receiver. These are Kernel
    # methods available everywhere. `system`, `exec`, and backtick
    # execution are the most common examples.
    BARE_METHOD_CAPABILITIES = {
      "system" => ["proc", "exec"],
      "exec" => ["proc", "exec"],
      "spawn" => ["proc", "exec"],
      "fork" => ["proc", "fork"],
      "`" => ["proc", "exec"],
      "open" => ["fs", "read"]
    }.freeze

    # ── Net::HTTP special case ────────────────────────────────────────
    #
    # Net::HTTP uses a nested constant (Net::HTTP), which appears in
    # Prism as a ConstantPathNode rather than a simple ConstantReadNode.
    # We handle it separately.
    NET_HTTP_METHODS = %w[get post put patch delete head start new].to_set.freeze

    # ────────────────────────────────────────────────────────────────────
    # Analyzer — the core AST walker
    # ────────────────────────────────────────────────────────────────────

    class Analyzer
      attr_reader :detected, :filename

      def initialize(filename)
        @filename = filename
        @detected = []
      end

      # Analyze Ruby source code and return detected capabilities.
      #
      # @param source [String] Ruby source code to analyze.
      # @return [Array<DetectedCapability>] detected capabilities.
      def analyze(source)
        result = Prism.parse(source)
        walk(result.value)
        @detected
      end

      private

      # Record a detected capability.
      def add(category, action, target, line, evidence)
        @detected << DetectedCapability.new(
          category: category,
          action: action,
          target: target,
          file: @filename,
          line: line,
          evidence: evidence
        )
      end

      # ── AST Walking ──────────────────────────────────────────────────
      #
      # We walk the tree recursively, checking each node. This is a
      # depth-first traversal. At each CallNode, we check if it matches
      # a known capability pattern.

      def walk(node)
        return unless node.is_a?(Prism::Node)

        case node
        when Prism::CallNode
          check_call(node)
        when Prism::XStringNode
          # Backtick string: `cmd`
          add("proc", "exec", "*", node.location.start_line, "`...` (backtick execution)")
        when Prism::InterpolatedXStringNode
          # Interpolated backtick string: `cmd #{expr}`
          add("proc", "exec", "*", node.location.start_line, "`...\#{...}` (interpolated backtick)")
        end

        # Visit all child nodes
        node.child_nodes.each do |child|
          walk(child) if child
        end
      end

      # ── Call Node Checks ─────────────────────────────────────────────
      #
      # A CallNode in Prism represents any method call. It has:
      #   - receiver: the object the method is called on (nil for bare calls)
      #   - name: the method name (a Symbol)
      #   - arguments: the ArgumentsNode containing arguments

      def check_call(node)
        check_require(node)
        check_class_method_call(node)
        check_bare_method_call(node)
        check_env_subscript(node)
        check_net_http(node)
      end

      # ── Require Detection ────────────────────────────────────────────
      #
      # `require "socket"` and `require_relative "..."` are the primary
      # way Ruby code pulls in libraries. The library name tells us what
      # capabilities the code might use.
      #
      # In Prism, `require "socket"` is a CallNode with:
      #   - receiver: nil (it's a Kernel method)
      #   - name: :require
      #   - arguments: [StringNode("socket")]

      def check_require(node)
        return unless node.receiver.nil?
        return unless %i[require require_relative].include?(node.name)

        # Extract the string argument (the library name)
        lib_name = extract_first_string_arg(node)
        return unless lib_name

        # Look up the library in our import-to-capability mapping
        if IMPORT_CAPABILITIES.key?(lib_name)
          category, action = IMPORT_CAPABILITIES[lib_name]
          add(category, action, "*", node.location.start_line, "require \"#{lib_name}\"")
        end
      end

      # ── Class Method Call Detection ──────────────────────────────────
      #
      # `File.read("x")` is a CallNode with:
      #   - receiver: ConstantReadNode(name: :File)
      #   - name: :read
      #   - arguments: [StringNode("x")]
      #
      # We check if the receiver is a known class and the method name
      # matches a known capability-bearing method.

      def check_class_method_call(node)
        return unless node.receiver.is_a?(Prism::ConstantReadNode)

        class_name = node.receiver.name.to_s
        method_name = node.name.to_s

        key = [class_name, method_name]
        return unless CLASS_METHOD_CAPABILITIES.key?(key)

        category, action = CLASS_METHOD_CAPABILITIES[key]

        # Try to extract a specific target from the first string argument.
        # For example, File.read("config.yml") → target is "config.yml".
        target = extract_first_string_arg(node) || "*"

        add(
          category,
          action,
          target,
          node.location.start_line,
          "#{class_name}.#{method_name}(#{target == "*" ? "..." : target.inspect})"
        )
      end

      # ── Bare Method Call Detection ───────────────────────────────────
      #
      # `system("ls")` is a CallNode with:
      #   - receiver: nil
      #   - name: :system
      #   - arguments: [StringNode("ls")]
      #
      # These are Kernel methods available everywhere in Ruby.

      def check_bare_method_call(node)
        return unless node.receiver.nil?

        method_name = node.name.to_s
        return unless BARE_METHOD_CAPABILITIES.key?(method_name)

        # Skip require/require_relative — handled separately
        return if %w[require require_relative].include?(method_name)

        category, action = BARE_METHOD_CAPABILITIES[method_name]
        target = extract_first_string_arg(node) || "*"

        add(
          category,
          action,
          target,
          node.location.start_line,
          "#{method_name}(#{target == "*" ? "..." : target.inspect})"
        )
      end

      # ── ENV Subscript Detection ──────────────────────────────────────
      #
      # `ENV["KEY"]` in Prism is a CallNode with:
      #   - receiver: ConstantReadNode(name: :ENV)
      #   - name: :[] (the subscript operator)
      #   - arguments: [StringNode("KEY")]
      #
      # This is how most Ruby code reads environment variables.

      def check_env_subscript(node)
        return unless node.receiver.is_a?(Prism::ConstantReadNode)
        return unless node.receiver.name == :ENV
        return unless node.name == :[]

        key = extract_first_string_arg(node) || "*"
        add("env", "read", key, node.location.start_line, "ENV[#{key == "*" ? "..." : key.inspect}]")
      end

      # ── Net::HTTP Detection ──────────────────────────────────────────
      #
      # `Net::HTTP.get(uri)` uses a ConstantPathNode for the receiver:
      #   - receiver: ConstantPathNode(parent: ConstantReadNode(:Net), name: :HTTP)
      #   - name: :get
      #
      # We handle this as a special case because nested constants are
      # common in Ruby's standard library.

      def check_net_http(node)
        return unless node.receiver.is_a?(Prism::ConstantPathNode)

        # Check for Net::HTTP pattern
        path_node = node.receiver
        return unless path_node.name == :HTTP
        return unless path_node.parent.is_a?(Prism::ConstantReadNode)
        return unless path_node.parent.name == :Net

        method_name = node.name.to_s
        return unless NET_HTTP_METHODS.include?(method_name)

        add(
          "net",
          "connect",
          "*",
          node.location.start_line,
          "Net::HTTP.#{method_name}(...)"
        )
      end

      # ── Helper: Extract First String Argument ────────────────────────
      #
      # Many capability-bearing calls take a string literal as their
      # first argument (e.g., `File.read("config.yml")`). This helper
      # extracts that string if it exists, returning nil otherwise.

      def extract_first_string_arg(node)
        return nil unless node.arguments
        args = node.arguments.arguments
        return nil if args.empty?

        first_arg = args.first
        case first_arg
        when Prism::StringNode
          first_arg.unescaped
        else
          nil
        end
      end
    end

    # ── Module-Level Convenience Methods ─────────────────────────────

    # Analyze a single Ruby file for capability usage.
    #
    # @param filepath [String] path to the Ruby source file.
    # @return [Array<DetectedCapability>] detected capabilities.
    # @raise [Errno::ENOENT] if the file does not exist.
    def self.analyze_file(filepath)
      source = File.read(filepath)
      analyzer = Analyzer.new(filepath)
      analyzer.analyze(source)
    end

    # Analyze all Ruby files in a directory tree.
    #
    # Walks the directory recursively, parsing each `.rb` file and
    # collecting all detected capabilities.
    #
    # @param directory [String] root directory to analyze.
    # @param exclude_tests [Boolean] if true, skip test/ directories.
    # @return [Array<DetectedCapability>] all detected capabilities.
    def self.analyze_directory(directory, exclude_tests: false)
      skip_dirs = %w[.git vendor node_modules .bundle]
      skip_dirs += %w[test tests spec] if exclude_tests

      all_detected = []

      Dir.glob(File.join(directory, "**", "*.rb")).each do |rb_file|
        # Skip excluded directories
        parts = rb_file.split(File::SEPARATOR)
        next if parts.any? { |part| skip_dirs.include?(part) }

        begin
          detected = analyze_file(rb_file)
          all_detected.concat(detected)
        rescue => _e
          # Skip files that can't be parsed
          nil
        end
      end

      all_detected
    end
  end
end
