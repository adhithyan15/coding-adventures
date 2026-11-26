# frozen_string_literal: true

module CodingAdventures
  module TestBootstrap
    # Which exceptions the engine may turn into a verdict (VER05h section 3).
    #
    # A property is user code and can raise anything.  Most of it is data the
    # engine must report (`property-panic`), but a few exceptions are the
    # process talking and must never be swallowed:
    #
    #   NoMemoryError          the VM cannot safely continue
    #   SignalException        Ctrl-C (Interrupt) and friends
    #   SystemExit             `exit` inside a property
    #   Timeout::ExitException a caller's own Timeout.timeout is unwinding us
    #
    # `Minitest::Assertion` is NOT a StandardError, so rescuing only
    # StandardError would let an assertion-style property crash the run.
    module Safe
      def self.recoverable?(error)
        return false if error.is_a?(NoMemoryError) ||
          error.is_a?(SignalException) ||
          error.is_a?(SystemExit)
        return false if defined?(::Timeout::ExitException) && error.is_a?(::Timeout::ExitException)

        true
      end

      # "Class: message" for a verdict, total (a hostile #message or class name
      # cannot break it) and bounded.  Control characters are escaped so a message
      # cannot forge log lines.
      def self.safe_string(error)
        message =
          begin
            error.message.to_s
          rescue Exception => e # rubocop:disable Lint/RescueException
            raise unless recoverable?(e)

            ""
          end
        sanitize("#{class_name(error)}: #{message}")
      end

      # The class name only, for redacted (CI) output.  Read WITHOUT calling any
      # user-defined method: `Kernel#class` and `Module#name` are bound from their
      # defining modules, so a `def self.to_s` / `def self.name` / `def class` on a
      # hostile exception cannot raise or lie.  Escaped and clipped like safe_string.
      def self.class_name(error)
        klass = Kernel.instance_method(:class).bind_call(error)
        name = Module.instance_method(:name).bind_call(klass)
        sanitize(name || "Exception")
      rescue Exception => e # rubocop:disable Lint/RescueException
        raise unless recoverable?(e)

        "Exception"
      end

      def self.sanitize(text)
        text.to_s[0, 200].gsub(/[\x00-\x1f\x7f]/) { |c| format("\\u%04x", c.ord) }
      end
    end
  end
end
