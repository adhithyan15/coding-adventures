# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_in_memory_data_store_protocol"

class TestInMemoryDataStoreProtocol < Minitest::Test
  include CodingAdventures::InMemoryDataStoreProtocol

  def test_decode_and_encode_command
    translator = Translator.new
    frame = CodingAdventures::RespProtocol::RespArray.new(values: [
      CodingAdventures::RespProtocol::BulkString.new(value: "SET"),
      CodingAdventures::RespProtocol::BulkString.new(value: "key"),
      CodingAdventures::RespProtocol::BulkString.new(value: "value")
    ])

    command = translator.decode(frame)

    assert_equal "SET", command.name
    assert_equal %w[key value], command.argv

    encoded = translator.encode(["OK", 1, nil])
    assert_instance_of CodingAdventures::RespProtocol::RespArray, encoded
    assert_equal "OK", encoded.values[0].value
    assert_equal 1, encoded.values[1].value
    assert_nil encoded.values[2].value
  end

  def test_rejects_non_array_frames
    translator = Translator.new

    assert_raises(ProtocolError) do
      translator.decode(CodingAdventures::RespProtocol::BulkString.new(value: "SET"))
    end
  end
end
