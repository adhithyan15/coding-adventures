# frozen_string_literal: true

require "open3"

module CodingAdventures
  module BoardVM
    CommandResult = Struct.new(:argv, :chdir, :stdout, :stderr, :exitstatus) do
      def success?
        exitstatus == 0
      end

      def output
        [stdout, stderr].compact.join("\n")
      end
    end

    class CommandError < StandardError
      attr_reader :result

      def initialize(result)
        @result = result
        super("command failed with exit #{result.exitstatus}: #{result.argv.join(" ")}")
      end
    end

    class CommandRunner
      def call(argv, chdir: nil)
        stdout, stderr, status = Open3.capture3(*argv, chdir: chdir)
        result = CommandResult.new(argv, chdir, stdout, stderr, status.exitstatus)
        raise CommandError.new(result) unless result.success?

        result
      end
    end
  end
end
