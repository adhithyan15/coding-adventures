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

      # "Class: message" for a verdict, total (a hostile #message cannot break it)
      # and bounded.  Control characters are escaped so a message cannot forge log lines.
      def self.safe_string(error)
        text = "#{error.class}: #{error.message}"
        text = text.to_s[0, 200]
        text.gsub(/[\x00-\x1f\x7f]/) { |c| format("\\u%04x", c.ord) }
      rescue Exception => e # rubocop:disable Lint/RescueException
        raise unless recoverable?(e)

        error.class.to_s
      end

      # The class name only, for redacted (CI) output.
      def self.class_name(error)
        error.class.to_s
      rescue Exception => e # rubocop:disable Lint/RescueException
        raise unless recoverable?(e)

        "Exception"
      end
    end
  end
end
