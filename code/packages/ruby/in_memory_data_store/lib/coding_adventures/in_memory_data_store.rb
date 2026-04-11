# frozen_string_literal: true

require "coding_adventures_resp_protocol"
require "coding_adventures_in_memory_data_store_protocol"
require "coding_adventures_in_memory_data_store_engine"

module CodingAdventures
  module InMemoryDataStore
    class InMemoryDataStore
      def initialize(
        engine: InMemoryDataStoreEngine::Engine.new,
        translator: InMemoryDataStoreProtocol::Translator.new,
        decoder: RespProtocol::Decoder.new,
        encoder: RespProtocol::Encoder.new
      )
        @engine = engine
        @translator = translator
        @decoder = decoder
        @encoder = encoder
      end

      def execute(input)
        case input
        when ::String
          @decoder.decode_all(input).map { |frame| execute_frame(frame) }.join
        else
          execute_frame(input)
        end
      end

      def execute_frame(frame)
        command =
          if frame.is_a?(InMemoryDataStoreProtocol::Command)
            frame
          else
            @translator.decode(frame)
          end

        response = @engine.execute(command)
        @encoder.encode(@translator.encode(response))
      rescue InMemoryDataStoreEngine::CommandError => e
        @encoder.encode(RespProtocol::Error.new(value: e.message))
      rescue StandardError => e
        @encoder.encode(RespProtocol::Error.new(value: e.message))
      end
    end
  end
end
