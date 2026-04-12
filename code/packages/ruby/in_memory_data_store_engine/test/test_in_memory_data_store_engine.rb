# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_in_memory_data_store_engine"

class TestInMemoryDataStoreEngine < Minitest::Test
  include CodingAdventures::InMemoryDataStoreEngine

  def command(name, *argv)
    CodingAdventures::InMemoryDataStoreProtocol::Command.new(name: name, argv: argv)
  end

  def test_strings_and_ttl
    engine = Engine.new

    assert_equal "OK", engine.execute(command("SET", "name", "Ada"))
    assert_equal "Ada", engine.execute(command("GET", "name"))
    assert_equal "string", engine.execute(command("TYPE", "name"))

    assert_equal 1, engine.execute(command("EXPIRE", "name", "1"))
    assert_equal 1, engine.execute(command("PERSIST", "name"))
    assert_equal(-1, engine.execute(command("TTL", "name")))
  end

  def test_hash_set_sorted_set_and_hll
    engine = Engine.new

    assert_equal 2, engine.execute(command("HSET", "user:1", "name", "Ada", "lang", "Ruby"))
    assert_equal "Ada", engine.execute(command("HGET", "user:1", "name"))
    assert_equal %w[lang Ruby name Ada], engine.execute(command("HGETALL", "user:1"))

    assert_equal 2, engine.execute(command("SADD", "colors", "red", "green", "red"))
    assert_equal %w[green red], engine.execute(command("SMEMBERS", "colors"))
    assert_equal 1, engine.execute(command("SISMEMBER", "colors", "red"))

    assert_equal 2, engine.execute(command("ZADD", "scores", "10", "bob", "5", "ada"))
    assert_equal %w[ada bob], engine.execute(command("ZRANGE", "scores", "0", "-1"))
    assert_equal 0, engine.execute(command("ZRANK", "scores", "ada"))

    assert_equal 1, engine.execute(command("PFADD", "visits", "a", "b", "c"))
    assert_in_delta(3, engine.execute(command("PFCOUNT", "visits")), 1)
  end

  def test_databases_and_flush
    engine = Engine.new(database_count: 4)

    assert_equal "OK", engine.execute(command("SET", "k", "v"))
    assert_equal 1, engine.execute(command("DBSIZE"))
    assert_equal "OK", engine.execute(command("SELECT", "1"))
    assert_equal 0, engine.execute(command("DBSIZE"))
    assert_equal "OK", engine.execute(command("FLUSHDB"))
  end

  def test_register_command
    engine = Engine.new
    engine.register_command("PINGX") { |_command| "PONGX" }

    assert_equal "PONGX", engine.execute(command("PINGX"))
  end
end
