# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_in_memory_data_store"

class TestInMemoryDataStore < Minitest::Test
  include CodingAdventures::InMemoryDataStore

  def command(*parts)
    values = parts.map { |part| CodingAdventures::RespProtocol::BulkString.new(value: part) }
    CodingAdventures::RespProtocol::RespArray.new(values: values)
  end

  def decode(encoded)
    CodingAdventures::RespProtocol::Decoder.new.decode(encoded).frame
  end

  def test_end_to_end_string_commands
    store = InMemoryDataStore.new

    ok = decode(store.execute(command("SET", "name", "Ada")))
    assert_equal "OK", ok.value

    value = decode(store.execute(command("GET", "name")))
    assert_equal "Ada", value.value
  end

  def test_end_to_end_hash_and_set_commands
    store = InMemoryDataStore.new

    decode(store.execute(command("HSET", "user:1", "name", "Ada", "lang", "Ruby")))
    hgetall = decode(store.execute(command("HGETALL", "user:1")))
    assert_equal %w[lang Ruby name Ada], hgetall.values.map(&:value)

    decode(store.execute(command("SADD", "colors", "red", "green")))
    smembers = decode(store.execute(command("SMEMBERS", "colors")))
    assert_equal %w[green red], smembers.values.map(&:value)
  end

  def test_error_frames_are_returned_for_wrong_types
    store = InMemoryDataStore.new
    decode(store.execute(command("SET", "name", "Ada")))

    error = decode(store.execute(command("HGET", "name", "field")))
    assert_instance_of CodingAdventures::RespProtocol::Error, error
  end

  def test_multiple_commands_in_one_payload
    store = InMemoryDataStore.new
    encoder = CodingAdventures::RespProtocol::Encoder.new
    payload = encoder.encode(command("SET", "name", "Ada")) + encoder.encode(command("GET", "name"))

    frames = CodingAdventures::RespProtocol::Decoder.new.decode_all(store.execute(payload))
    assert_equal %w[OK Ada], frames.map(&:value)
  end
end
