defmodule CodingAdventures.InMemoryDataStoreEngineTest do
  use ExUnit.Case, async: true

  alias CodingAdventures.InMemoryDataStoreEngine
  alias CodingAdventures.InMemoryDataStoreProtocol
  alias CodingAdventures.RESPProtocol

  defmodule CustomModule do
    def commands do
      [
        %{
          name: "CUSTOM",
          mutating: false,
          skip_lazy_expire: true,
          handler: &__MODULE__.handle/2
        }
      ]
    end

    def handle(store, [value]) do
      {store, RESPProtocol.bulk_string("custom:" <> value)}
    end
  end

  defmodule EmptyModule do
  end

  defp execute(engine, name, args \\ []) do
    InMemoryDataStoreEngine.execute(engine, InMemoryDataStoreProtocol.build(name, args))
  end

  test "registering commands and modules works" do
    engine = InMemoryDataStoreEngine.new()
    assert %InMemoryDataStoreEngine.Store{} = InMemoryDataStoreEngine.store(engine)

    engine =
      InMemoryDataStoreEngine.register_command(engine, "CUSTOM2", false, true, fn store, args ->
        {store, RESPProtocol.bulk_string(Enum.join(args, ","))}
      end)

    {engine, reply} = execute(engine, "CUSTOM2", ["a", "b"])
    assert reply == {:bulk_string, "a,b"}

    engine = InMemoryDataStoreEngine.register_module(engine, CustomModule)
    {engine, reply} = execute(engine, "CUSTOM", ["x"])
    assert reply == {:bulk_string, "custom:x"}
    assert InMemoryDataStoreEngine.register_module(engine, EmptyModule) == engine

    {_, reply} = InMemoryDataStoreEngine.execute(engine, "MISSING", [])
    assert reply == {:error, "ERR unknown command 'MISSING'"}
  end

  test "arity and option errors are surfaced" do
    engine = InMemoryDataStoreEngine.new()

    cases = [
      {"PING", ["a", "b"], "ERR wrong number of arguments for 'PING'"},
      {"ECHO", [], "ERR wrong number of arguments for 'ECHO'"},
      {"SET", ["a"], "ERR wrong number of arguments for 'SET'"},
      {"GET", [], "ERR wrong number of arguments for 'GET'"},
      {"DEL", [], "ERR wrong number of arguments for 'DEL'"},
      {"EXISTS", [], "ERR wrong number of arguments for 'EXISTS'"},
      {"TYPE", [], "ERR wrong number of arguments for 'TYPE'"},
      {"RENAME", ["a"], "ERR wrong number of arguments for 'RENAME'"},
      {"INCR", ["a", "b"], "ERR wrong number of arguments for 'INCR'"},
      {"DECR", ["a", "b"], "ERR wrong number of arguments for 'DECR'"},
      {"INCRBY", ["a"], "ERR wrong number of arguments for 'INCRBY'"},
      {"DECRBY", ["a"], "ERR wrong number of arguments for 'INCRBY'"},
      {"APPEND", ["a"], "ERR wrong number of arguments for 'APPEND'"},
      {"HSET", ["h", "f1"], "ERR wrong number of arguments for 'HSET'"},
      {"HGET", ["h"], "ERR wrong number of arguments for 'HGET'"},
      {"HDEL", ["h"], "ERR wrong number of arguments for 'HDEL'"},
      {"HGETALL", [], "ERR wrong number of arguments for 'HGETALL'"},
      {"HLEN", [], "ERR wrong number of arguments for 'HLEN'"},
      {"HEXISTS", ["h"], "ERR wrong number of arguments for 'HEXISTS'"},
      {"HKEYS", [], "ERR wrong number of arguments for 'HKEYS'"},
      {"HVALS", [], "ERR wrong number of arguments for 'HVALS'"},
      {"LPUSH", ["list"], "ERR wrong number of arguments for 'LPUSH'"},
      {"RPUSH", ["list"], "ERR wrong number of arguments for 'RPUSH'"},
      {"LPOP", [], "ERR wrong number of arguments for 'LPOP'"},
      {"RPOP", [], "ERR wrong number of arguments for 'RPOP'"},
      {"LLEN", [], "ERR wrong number of arguments for 'LLEN'"},
      {"LRANGE", ["list", "0"], "ERR wrong number of arguments for 'LRANGE'"},
      {"LINDEX", ["list"], "ERR wrong number of arguments for 'LINDEX'"},
      {"SADD", ["s"], "ERR wrong number of arguments for 'SADD'"},
      {"SREM", ["s"], "ERR wrong number of arguments for 'SREM'"},
      {"SISMEMBER", ["s"], "ERR wrong number of arguments for 'SISMEMBER'"},
      {"SMEMBERS", [], "ERR wrong number of arguments for 'SMEMBERS'"},
      {"SCARD", [], "ERR wrong number of arguments for 'SCARD'"},
      {"SUNION", [], "ERR wrong number of arguments for 'SUNION'"},
      {"SINTER", [], "ERR wrong number of arguments for 'SINTER'"},
      {"SDIFF", [], "ERR wrong number of arguments for 'SDIFF'"},
      {"ZADD", ["z", "1"], "ERR wrong number of arguments for 'ZADD'"},
      {"ZREM", ["z"], "ERR wrong number of arguments for 'ZREM'"},
      {"ZCARD", [], "ERR wrong number of arguments for 'ZCARD'"},
      {"ZRANGE", ["z", "0"], "ERR wrong number of arguments for 'ZRANGE'"},
      {"ZRANK", ["z"], "ERR wrong number of arguments for 'ZRANK'"},
      {"ZREVRANGE", ["z", "0"], "ERR wrong number of arguments for 'ZREVRANGE'"},
      {"ZRANGEBYSCORE", ["z", "0"], "ERR wrong number of arguments for 'ZRANGEBYSCORE'"},
      {"ZSCORE", ["z"], "ERR wrong number of arguments for 'ZSCORE'"},
      {"PFADD", ["hll"], "ERR wrong number of arguments for 'PFADD'"},
      {"PFCOUNT", [], "ERR wrong number of arguments for 'PFCOUNT'"},
      {"PFMERGE", ["dest"], "ERR wrong number of arguments for 'PFMERGE'"},
      {"EXPIRE", ["a"], "ERR wrong number of arguments for 'EXPIRE'"},
      {"EXPIREAT", ["a"], "ERR wrong number of arguments for 'EXPIREAT'"},
      {"TTL", [], "ERR wrong number of arguments for 'TTL'"},
      {"PTTL", [], "ERR wrong number of arguments for 'PTTL'"},
      {"PERSIST", [], "ERR wrong number of arguments for 'PERSIST'"},
      {"SELECT", [], "ERR wrong number of arguments for 'SELECT'"},
      {"FLUSHDB", ["x"], "ERR wrong number of arguments for 'FLUSHDB'"},
      {"FLUSHALL", ["x"], "ERR wrong number of arguments for 'FLUSHALL'"},
      {"DBSIZE", ["x"], "ERR wrong number of arguments for 'DBSIZE'"},
      {"INFO", ["x"], "ERR wrong number of arguments for 'INFO'"},
      {"KEYS", [], "ERR wrong number of arguments for 'KEYS'"}
    ]

    Enum.each(cases, fn {name, args, message} ->
      {_, reply} = execute(engine, name, args)
      assert reply == {:error, message}
    end)

    {_, reply} = execute(engine, "SET", ["k", "1", "NX", "XX"])
    assert reply == {:error, "ERR syntax error"}

    {_, reply} = execute(engine, "SET", ["k", "1", "EX", "nope"])
    assert reply == {:error, "ERR value is not an integer or out of range"}

    {_, reply} = execute(engine, "SET", ["k", "1", "FOO"])
    assert reply == {:error, "ERR syntax error"}

    {_, reply} = execute(engine, "ZADD", ["z", "bad", "member"])
    assert reply == {:error, "ERR value is not a valid float"}

    {_, reply} = execute(engine, "ZRANGE", ["z", "bad", "1"])
    assert reply == {:error, "ERR value is not an integer or out of range"}

    {_, reply} = execute(engine, "ZRANGEBYSCORE", ["z", "bad", "1"])
    assert reply == {:error, "ERR value is not an integer or out of range"}

    {_, reply} = execute(engine, "EXPIRE", ["k", "nope"])
    assert reply == {:error, "ERR value is not an integer or out of range"}
  end

  test "string commands, expiry, and type reporting work" do
    engine = InMemoryDataStoreEngine.new()

    {engine, reply} = execute(engine, "PING")
    assert reply == {:simple_string, "PONG"}

    {engine, reply} = execute(engine, "PING", ["hello"])
    assert reply == {:bulk_string, "hello"}

    {engine, reply} = execute(engine, "ECHO", ["hello"])
    assert reply == {:bulk_string, "hello"}

    {engine, reply} = execute(engine, "SET", ["counter", "1"])
    assert reply == {:simple_string, "OK"}

    {engine, reply} = execute(engine, "SET", ["counter", "2", "NX"])
    assert reply == :null_bulk_string

    {engine, reply} = execute(engine, "SET", ["counter", "2", "XX"])
    assert reply == {:simple_string, "OK"}

    {engine, reply} = execute(engine, "INCR", ["counter"])
    assert reply == {:integer, 3}

    {engine, reply} = execute(engine, "DECR", ["counter"])
    assert reply == {:integer, 2}

    {engine, reply} = execute(engine, "INCRBY", ["counter", "5"])
    assert reply == {:integer, 7}

    {engine, reply} = execute(engine, "DECRBY", ["counter", "2"])
    assert reply == {:integer, 5}

    {engine, reply} = execute(engine, "APPEND", ["counter", "x"])
    assert reply == {:integer, 2}

    {engine, reply} = execute(engine, "GET", ["counter"])
    assert reply == {:bulk_string, "5x"}

    {engine, reply} = execute(engine, "TYPE", ["counter"])
    assert reply == {:simple_string, "string"}

    {engine, reply} = execute(engine, "SET", ["number", "abc"])
    assert reply == {:simple_string, "OK"}

    {engine, reply} = execute(engine, "INCR", ["number"])
    assert reply == {:error, "ERR value is not an integer or out of range"}

    {engine, reply} = execute(engine, "GET", ["missing"])
    assert reply == :null_bulk_string

    {engine, reply} = execute(engine, "SET", ["expiring", "value", "PX", "0"])
    assert reply == {:simple_string, "OK"}

    {engine, reply} = execute(engine, "GET", ["expiring"])
    assert reply == :null_bulk_string

    {engine, reply} = execute(engine, "TYPE", ["expiring"])
    assert reply == {:simple_string, "none"}

    {engine, reply} = execute(engine, "EXISTS", ["counter", "missing", "number"])
    assert reply == {:integer, 2}

    {engine, reply} = execute(engine, "RENAME", ["counter", "renamed"])
    assert reply == {:simple_string, "OK"}

    {_, reply} = execute(engine, "TYPE", ["renamed"])
    assert reply == {:simple_string, "string"}
  end

  test "hash list set zset and hll commands work" do
    engine = InMemoryDataStoreEngine.new()

    {engine, reply} = execute(engine, "HSET", ["hash", "f1", "v1", "f2", "v2"])
    assert reply == {:integer, 2}
    {engine, reply} = execute(engine, "HGET", ["hash", "f1"])
    assert reply == {:bulk_string, "v1"}
    {engine, reply} = execute(engine, "HGETALL", ["hash"])
    assert reply == {:array, [bulk_string: "f1", bulk_string: "v1", bulk_string: "f2", bulk_string: "v2"]}
    {engine, reply} = execute(engine, "HLEN", ["hash"])
    assert reply == {:integer, 2}
    {engine, reply} = execute(engine, "HEXISTS", ["hash", "f1"])
    assert reply == {:integer, 1}
    {engine, reply} = execute(engine, "HKEYS", ["hash"])
    assert reply == {:array, [bulk_string: "f1", bulk_string: "f2"]}
    {engine, reply} = execute(engine, "HVALS", ["hash"])
    assert reply == {:array, [bulk_string: "v1", bulk_string: "v2"]}
    {engine, reply} = execute(engine, "HDEL", ["hash", "f1"])
    assert reply == {:integer, 1}

    {engine, reply} = execute(engine, "LPUSH", ["queue", "b", "a"])
    assert reply == {:integer, 2}
    {engine, reply} = execute(engine, "RPUSH", ["queue", "c"])
    assert reply == {:integer, 3}
    {engine, reply} = execute(engine, "LLEN", ["queue"])
    assert reply == {:integer, 3}
    {engine, reply} = execute(engine, "LRANGE", ["queue", "0", "-1"])
    assert reply == {:array, [bulk_string: "a", bulk_string: "b", bulk_string: "c"]}
    {engine, reply} = execute(engine, "LINDEX", ["queue", "1"])
    assert reply == {:bulk_string, "b"}
    {engine, reply} = execute(engine, "LINDEX", ["queue", "9"])
    assert reply == :null_bulk_string
    {engine, reply} = execute(engine, "LPOP", ["queue"])
    assert reply == {:bulk_string, "a"}
    {engine, reply} = execute(engine, "RPOP", ["queue"])
    assert reply == {:bulk_string, "c"}
    {engine, reply} = execute(engine, "LPOP", ["queue"])
    assert reply == {:bulk_string, "b"}
    {engine, reply} = execute(engine, "RPOP", ["queue"])
    assert reply == :null_bulk_string

    {engine, reply} = execute(engine, "SADD", ["set", "a", "b", "a"])
    assert reply == {:integer, 2}
    {engine, reply} = execute(engine, "SREM", ["set", "b"])
    assert reply == {:integer, 1}
    {engine, reply} = execute(engine, "SISMEMBER", ["set", "a"])
    assert reply == {:integer, 1}
    {engine, reply} = execute(engine, "SMEMBERS", ["set"])
    assert reply == {:array, [bulk_string: "a"]}
    {engine, reply} = execute(engine, "SCARD", ["set"])
    assert reply == {:integer, 1}
    {engine, reply} = execute(engine, "SADD", ["set2", "b", "c"])
    assert reply == {:integer, 2}
    {engine, reply} = execute(engine, "SUNION", ["set", "set2"])
    assert reply == {:array, [bulk_string: "a", bulk_string: "b", bulk_string: "c"]}
    {engine, reply} = execute(engine, "SINTER", ["set", "set2"])
    assert reply == {:array, []}
    {engine, reply} = execute(engine, "SDIFF", ["set2", "set"])
    assert reply == {:array, [bulk_string: "b", bulk_string: "c"]}

    {engine, reply} = execute(engine, "ZADD", ["scores", "1", "a", "2", "b", "1.5", "c"])
    assert reply == {:integer, 3}
    {engine, reply} = execute(engine, "ZCARD", ["scores"])
    assert reply == {:integer, 3}
    {engine, reply} = execute(engine, "ZRANGE", ["scores", "0", "-1"])
    assert reply == {:array, [bulk_string: "a", bulk_string: "1.0", bulk_string: "c", bulk_string: "1.5", bulk_string: "b", bulk_string: "2.0"]}
    {engine, reply} = execute(engine, "ZRANK", ["scores", "b"])
    assert reply == {:integer, 2}
    {engine, reply} = execute(engine, "ZREVRANGE", ["scores", "0", "0"])
    assert reply == {:array, [bulk_string: "b", bulk_string: "2.0"]}
    {engine, reply} = execute(engine, "ZRANGEBYSCORE", ["scores", "1", "1.5"])
    assert reply == {:array, [bulk_string: "a", bulk_string: "1.0", bulk_string: "c", bulk_string: "1.5"]}
    {engine, reply} = execute(engine, "ZRANGEBYSCORE", ["scores", "-inf", "+inf"])
    assert reply == {:array, [bulk_string: "a", bulk_string: "1.0", bulk_string: "c", bulk_string: "1.5", bulk_string: "b", bulk_string: "2.0"]}
    {engine, reply} = execute(engine, "ZSCORE", ["scores", "c"])
    assert reply == {:bulk_string, "1.5"}
    {engine, reply} = execute(engine, "ZRANK", ["scores", "missing"])
    assert reply == :null_bulk_string
    {engine, reply} = execute(engine, "ZSCORE", ["scores", "missing"])
    assert reply == :null_bulk_string
    {engine, reply} = execute(engine, "ZREM", ["scores", "a"])
    assert reply == {:integer, 1}

    {engine, reply} = execute(engine, "PFADD", ["hll", "a", "b", "a"])
    assert reply == {:integer, 1}
    {engine, reply} = execute(engine, "PFCOUNT", ["hll"])
    assert match?({:integer, value} when value > 0, reply)
    {engine, reply} = execute(engine, "PFCOUNT", ["missing"])
    assert reply == {:integer, 0}
    {engine, reply} = execute(engine, "PFMERGE", ["merged", "hll"])
    assert reply == {:simple_string, "OK"}
    {engine, reply} = execute(engine, "PFMERGE", ["merged_missing", "missing"])
    assert reply == {:simple_string, "OK"}
    {_, reply} = execute(engine, "PFCOUNT", ["merged"])
    assert match?({:integer, value} when value > 0, reply)

    for {key, expected_type} <- [
          {"hash", "hash"},
          {"queue", "none"},
          {"set", "set"},
          {"scores", "zset"},
          {"hll", "hll"},
          {"merged", "hll"}
        ] do
      {_, reply} = execute(engine, "TYPE", [key])
      assert reply == {:simple_string, expected_type}
    end
  end

  test "admin commands and key index work together" do
    engine = InMemoryDataStoreEngine.new()

    {engine, _} = execute(engine, "SET", ["alpha", "1"])
    {engine, _} = execute(engine, "SET", ["beta", "2"])
    {engine, _} = execute(engine, "SET", ["group:one", "x"])
    {engine, _} = execute(engine, "SET", ["group:two", "y"])
    {engine, _} = execute(engine, "SET", ["temp", "gone", "EX", "0"])

    {engine, reply} = execute(engine, "KEYS", ["group:*"])
    assert reply == {:array, [bulk_string: "group:one", bulk_string: "group:two"]}

    {engine, reply} = execute(engine, "KEYS", ["*"])
    assert match?({:array, values} when length(values) >= 4, reply)

    {engine, reply} = execute(engine, "DBSIZE")
    assert reply == {:integer, 4}

    {engine, reply} = execute(engine, "INFO")
    assert {:bulk_string, info} = reply
    assert String.contains?(info, "db=0")

    {engine, reply} = execute(engine, "SELECT", ["1"])
    assert reply == {:simple_string, "OK"}
    {engine, reply} = execute(engine, "DBSIZE")
    assert reply == {:integer, 0}
    {engine, reply} = execute(engine, "SET", ["scoped", "1"])
    assert reply == {:simple_string, "OK"}

    {engine, reply} = execute(engine, "FLUSHDB")
    assert reply == {:simple_string, "OK"}
    {engine, reply} = execute(engine, "DBSIZE")
    assert reply == {:integer, 0}

    {engine, reply} = execute(engine, "SELECT", ["0"])
    assert reply == {:simple_string, "OK"}
    {engine, reply} = execute(engine, "DBSIZE")
    assert reply == {:integer, 4}

    {engine, reply} = execute(engine, "FLUSHALL")
    assert reply == {:simple_string, "OK"}
    {engine, reply} = execute(engine, "DBSIZE")
    assert reply == {:integer, 0}

    {engine, reply} = execute(engine, "SET", ["ttl", "value"])
    assert reply == {:simple_string, "OK"}
    {engine, reply} = execute(engine, "EXPIRE", ["ttl", "0"])
    assert reply == {:integer, 1}
    {engine, reply} = execute(engine, "TTL", ["ttl"])
    assert reply == {:integer, -2}
    {engine, reply} = execute(engine, "PTTL", ["ttl"])
    assert reply == {:integer, -2}
    {engine, reply} = execute(engine, "EXPIREAT", ["ttl", "0"])
    assert reply == {:integer, 0}
    {engine, reply} = execute(engine, "PERSIST", ["ttl"])
    assert reply == {:integer, 0}
    {engine, reply} = execute(engine, "EXISTS", ["ttl"])
    assert reply == {:integer, 0}
    {_, reply} = execute(engine, "TYPE", ["ttl"])
    assert reply == {:simple_string, "none"}
  end
end
