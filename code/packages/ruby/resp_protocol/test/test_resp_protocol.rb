# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_resp_protocol"

class TestRespProtocol < Minitest::Test
  include CodingAdventures::RespProtocol

  def test_encode_and_decode_bulk_string_array
    frame = RespArray.new(values: [
      BulkString.new(value: "SET"),
      BulkString.new(value: "key"),
      BulkString.new(value: "value")
    ])

    encoded = Encoder.new.encode(frame)
    decoded = Decoder.new.decode(encoded).frame

    assert_instance_of RespArray, decoded
    assert_equal %w[SET key value], decoded.values.map(&:value)
  end

  def test_encode_and_decode_simple_and_integer_frames
    simple = SimpleString.new(value: "OK")
    integer = RespInteger.new(value: 42)

    decoder = Decoder.new

    assert_equal "OK", decoder.decode(Encoder.new.encode(simple)).frame.value
    assert_equal 42, decoder.decode(Encoder.new.encode(integer)).frame.value
  end

  def test_decode_null_bulk_string_and_nested_arrays
    encoded = "*2\r\n$-1\r\n*1\r\n:7\r\n"
    frame = Decoder.new.decode(encoded).frame

    assert_instance_of RespArray, frame
    assert_nil frame.values.first.value
    assert_equal 7, frame.values.last.values.first.value
  end

  def test_decode_rejects_truncated_bulk_string
    assert_raises(ParseError) do
      Decoder.new.decode("$5\r\nabc\r\n")
    end
  end
end
