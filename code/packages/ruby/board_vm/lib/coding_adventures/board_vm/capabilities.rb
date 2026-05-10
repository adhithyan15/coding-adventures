# frozen_string_literal: true

module CodingAdventures
  module BoardVM
    class Capability
      attr_reader :id, :version, :flags, :name, :flag_names, :raw

      def initialize(raw)
        @raw = raw
        @id = raw.fetch("id")
        @version = raw.fetch("version")
        @flags = raw.fetch("flags")
        @name = raw.fetch("name")
        @flag_names = Array(raw["flag_names"]).freeze
      end

      def bytecode_callable?
        bool_or_flag?("bytecode_callable")
      end

      def protocol_feature?
        bool_or_flag?("protocol_feature")
      end

      def board_metadata?
        bool_or_flag?("board_metadata")
      end

      private

      def bool_or_flag?(name)
        raw[name] == true || flag_names.include?(name)
      end
    end

    class BoardDescriptor
      attr_reader :board_id, :runtime_id, :max_program_bytes, :max_stack_values,
        :max_handles, :supports_store_program, :capabilities, :raw

      def self.from_decoded_response(decoded_response)
        return nil unless decoded_response && decoded_response["kind"] == "caps_report"

        new(decoded_response.fetch("payload"))
      end

      def initialize(raw)
        @raw = raw
        @board_id = raw.fetch("board_id")
        @runtime_id = raw.fetch("runtime_id")
        @max_program_bytes = raw.fetch("max_program_bytes")
        @max_stack_values = raw.fetch("max_stack_values")
        @max_handles = raw.fetch("max_handles")
        @supports_store_program = raw.fetch("supports_store_program")
        @capabilities = Array(raw.fetch("capabilities")).map { |capability| Capability.new(capability) }.freeze
      end

      def supports?(name_or_id)
        !!capability(name_or_id)
      end

      def capability(name_or_id)
        if name_or_id.is_a?(Integer)
          capabilities.find { |capability| capability.id == name_or_id }
        else
          wanted = name_or_id.to_s
          capabilities.find { |capability| capability.name == wanted }
        end
      end
      alias [] capability

      def capability_names
        capabilities.map(&:name)
      end

      def bytecode_capabilities
        capabilities.select(&:bytecode_callable?)
      end

      def protocol_features
        capabilities.select(&:protocol_feature?)
      end

      def board_metadata
        capabilities.select(&:board_metadata?)
      end

      def gpio
        capabilities.select { |capability| capability.name.start_with?("gpio.") }
      end

      def time
        capabilities.select { |capability| capability.name.start_with?("time.") }
      end

      def program
        capabilities.select { |capability| capability.name.start_with?("program.") }
      end

      def store_program?
        supports_store_program || supports?("program.store")
      end
    end
  end
end
