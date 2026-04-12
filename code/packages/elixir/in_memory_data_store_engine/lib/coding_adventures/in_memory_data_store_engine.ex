defmodule CodingAdventures.InMemoryDataStoreEngine do
  @moduledoc """
  Registry-driven in-memory datastore engine.
  """

  alias CodingAdventures.HashMap
  alias CodingAdventures.HashSet
  alias CodingAdventures.HyperLogLog
  alias CodingAdventures.ArrayList
  alias CodingAdventures.InMemoryDataStoreProtocol.Command
  alias CodingAdventures.RadixTree
  alias CodingAdventures.RESPProtocol

  defmodule Entry do
    @enforce_keys [:type, :value]
    defstruct [:type, :value, expires_at_ms: nil]
  end

  defmodule Store do
    @enforce_keys [:dbs, :key_indexes, :selected_db]
    defstruct dbs: %{0 => HashMap.new()}, key_indexes: %{0 => RadixTree.new()}, selected_db: 0
  end

  defstruct store: nil, commands: %{}

  def new(opts \\ []) do
    engine = %__MODULE__{store: %Store{dbs: %{0 => HashMap.new()}, key_indexes: %{0 => RadixTree.new()}, selected_db: 0}}
    engine = register_builtin_commands(engine)
    Enum.reduce(Keyword.get(opts, :register, []), engine, fn {name, handler}, acc ->
      register_command(acc, name, false, false, handler)
    end)
  end

  def store(%__MODULE__{store: store}), do: store

  def register_command(%__MODULE__{} = engine, name, mutating, skip_lazy_expire, handler)
      when is_function(handler, 2) do
    name = normalize_name(name)

    %{engine | commands: Map.put(engine.commands, name, %{mutating: mutating, skip_lazy_expire: skip_lazy_expire, handler: handler})}
  end

  def register_module(%__MODULE__{} = engine, module) when is_atom(module) do
    if function_exported?(module, :commands, 0) do
      Enum.reduce(module.commands(), engine, fn %{name: name, mutating: mutating, skip_lazy_expire: skip, handler: handler}, acc ->
        register_command(acc, name, mutating, skip, handler)
      end)
    else
      engine
    end
  end

  def execute(%__MODULE__{} = engine, %Command{name: name, args: args}) do
    execute(engine, name, args)
  end

  def execute(%__MODULE__{} = engine, name, args \\ []) do
    name = normalize_name(name)

    case Map.fetch(engine.commands, name) do
      :error ->
        {engine, RESPProtocol.error("ERR unknown command '#{name}'")}

      {:ok, %{handler: handler, skip_lazy_expire: skip_lazy_expire}} ->
        store = if skip_lazy_expire, do: engine.store, else: purge_expired(engine.store)
        {store, reply} = handler.(store, Enum.map(args, &to_string/1))
        {%{engine | store: store}, reply}
    end
  end

  defp register_builtin_commands(engine) do
    specs = [
      {"PING", false, true, &cmd_ping/2},
      {"ECHO", false, true, &cmd_echo/2},
      {"SET", true, false, &cmd_set/2},
      {"GET", false, false, &cmd_get/2},
      {"DEL", true, false, &cmd_del/2},
      {"EXISTS", false, false, &cmd_exists/2},
      {"TYPE", false, false, &cmd_type/2},
      {"RENAME", true, false, &cmd_rename/2},
      {"INCR", true, false, &cmd_incr/2},
      {"DECR", true, false, &cmd_decr/2},
      {"INCRBY", true, false, &cmd_incrby/2},
      {"DECRBY", true, false, &cmd_decrby/2},
      {"APPEND", true, false, &cmd_append/2},
      {"LPUSH", true, false, &cmd_lpush/2},
      {"RPUSH", true, false, &cmd_rpush/2},
      {"LPOP", true, false, &cmd_lpop/2},
      {"RPOP", true, false, &cmd_rpop/2},
      {"LLEN", false, false, &cmd_llen/2},
      {"LRANGE", false, false, &cmd_lrange/2},
      {"LINDEX", false, false, &cmd_lindex/2},
      {"HSET", true, false, &cmd_hset/2},
      {"HGET", false, false, &cmd_hget/2},
      {"HDEL", true, false, &cmd_hdel/2},
      {"HGETALL", false, false, &cmd_hgetall/2},
      {"HLEN", false, false, &cmd_hlen/2},
      {"HEXISTS", false, false, &cmd_hexists/2},
      {"HKEYS", false, false, &cmd_hkeys/2},
      {"HVALS", false, false, &cmd_hvals/2},
      {"SADD", true, false, &cmd_sadd/2},
      {"SREM", true, false, &cmd_srem/2},
      {"SISMEMBER", false, false, &cmd_sismember/2},
      {"SMEMBERS", false, false, &cmd_smembers/2},
      {"SCARD", false, false, &cmd_scard/2},
      {"SUNION", false, false, &cmd_sunion/2},
      {"SINTER", false, false, &cmd_sinter/2},
      {"SDIFF", false, false, &cmd_sdiff/2},
      {"ZADD", true, false, &cmd_zadd/2},
      {"ZREM", true, false, &cmd_zrem/2},
      {"ZCARD", false, false, &cmd_zcard/2},
      {"ZRANGE", false, false, &cmd_zrange/2},
      {"ZRANK", false, false, &cmd_zrank/2},
      {"ZREVRANGE", false, false, &cmd_zrevrange/2},
      {"ZRANGEBYSCORE", false, false, &cmd_zrangebyscore/2},
      {"ZSCORE", false, false, &cmd_zscore/2},
      {"PFADD", true, false, &cmd_pfadd/2},
      {"PFCOUNT", false, false, &cmd_pfcount/2},
      {"PFMERGE", true, false, &cmd_pfmerge/2},
      {"EXPIRE", true, false, &cmd_expire/2},
      {"EXPIREAT", true, false, &cmd_expireat/2},
      {"TTL", false, false, &cmd_ttl/2},
      {"PTTL", false, false, &cmd_pttl/2},
      {"PERSIST", true, false, &cmd_persist/2},
      {"SELECT", true, true, &cmd_select/2},
      {"FLUSHDB", true, true, &cmd_flushdb/2},
      {"FLUSHALL", true, true, &cmd_flushall/2},
      {"DBSIZE", false, true, &cmd_dbsize/2},
      {"INFO", false, true, &cmd_info/2},
      {"KEYS", false, false, &cmd_keys/2}
    ]

    Enum.reduce(specs, engine, fn {name, mutating, skip_lazy_expire, handler}, acc ->
      register_command(acc, name, mutating, skip_lazy_expire, handler)
    end)
  end

  defp normalize_name(name), do: name |> to_string() |> String.upcase()

  defp now_ms, do: System.system_time(:millisecond)

  defp purge_expired(%Store{} = store) do
    db = current_db(store)
    live_entries =
      db
      |> HashMap.entries()
      |> Enum.reject(fn {_key, %Entry{} = entry} -> expired?(entry) end)

    live_db =
      live_entries
      |> HashMap.from_list(strategy: HashMap.strategy(db), hash_fn: HashMap.hash_fn(db), capacity: max(HashMap.capacity(db), 16))

    live_index =
      live_entries
      |> Enum.map(&{elem(&1, 0), true})
      |> RadixTree.from_list()

    store
    |> put_current_db(live_db)
    |> put_current_key_index(live_index)
  end

  defp current_db(%Store{dbs: dbs, selected_db: selected_db}) do
    Map.get(dbs, selected_db, HashMap.new())
  end

  defp current_key_index(%Store{key_indexes: key_indexes, selected_db: selected_db}) do
    Map.get(key_indexes, selected_db, RadixTree.new())
  end

  defp put_current_db(%Store{dbs: dbs, selected_db: selected_db} = store, db) do
    %{store | dbs: Map.put(dbs, selected_db, db)}
  end

  defp put_current_key_index(%Store{key_indexes: key_indexes, selected_db: selected_db} = store, key_index) do
    %{store | key_indexes: Map.put(key_indexes, selected_db, key_index)}
  end

  defp select_db(%Store{} = store, index) when index >= 0 do
    %{
      store
      | selected_db: index,
        dbs: Map.put_new(store.dbs, index, HashMap.new()),
        key_indexes: Map.put_new(store.key_indexes, index, RadixTree.new())
    }
  end

  defp put_entry(%Store{} = store, key, %Entry{} = entry) do
    db = current_db(store) |> HashMap.put(key, entry)
    key_index = current_key_index(store) |> RadixTree.put(key, true)

    store
    |> put_current_db(db)
    |> put_current_key_index(key_index)
  end

  defp delete_entry(%Store{} = store, key) do
    db = current_db(store)
    existed = HashMap.has_key?(db, key)

    next_store =
      if existed do
        store
        |> put_current_db(HashMap.delete(db, key))
        |> put_current_key_index(RadixTree.delete(current_key_index(store), key))
      else
        store
      end

    {next_store, existed}
  end

  defp fetch_entry(%Store{} = store, key) do
    db = current_db(store)

    case HashMap.get(db, key) do
      nil -> {store, nil}
      %Entry{} = entry ->
        if expired?(entry) do
          {deleted_store, _} = delete_entry(store, key)
          {deleted_store, nil}
        else
          {store, entry}
        end
    end
  end

  defp expired?(%Entry{expires_at_ms: nil}), do: false
  defp expired?(%Entry{expires_at_ms: expires_at}), do: now_ms() >= expires_at

  defp entry(type, value, expires_at_ms \\ nil), do: %Entry{type: type, value: value, expires_at_ms: expires_at_ms}

  defp wrong_type(), do: RESPProtocol.error("WRONGTYPE Operation against a key holding the wrong kind of value")
  defp ok(), do: RESPProtocol.simple_string("OK")

  defp fetch_string(%Store{} = store, key) do
    {store, entry} = fetch_entry(store, key)
    case entry do
      nil -> {store, nil}
      %Entry{type: :string, value: value} -> {store, value}
      _ -> {store, :wrong_type}
    end
  end

  defp fetch_hash(%Store{} = store, key) do
    {store, entry} = fetch_entry(store, key)
    case entry do
      nil -> {store, HashMap.new()}
      %Entry{type: :hash, value: value} -> {store, value}
      _ -> {store, :wrong_type}
    end
  end

  defp fetch_set(%Store{} = store, key) do
    {store, entry} = fetch_entry(store, key)
    case entry do
      nil -> {store, HashSet.new()}
      %Entry{type: :set, value: value} -> {store, value}
      _ -> {store, :wrong_type}
    end
  end

  defp fetch_list(%Store{} = store, key) do
    {store, entry} = fetch_entry(store, key)
    case entry do
      nil -> {store, ArrayList.new()}
      %Entry{type: :list, value: value} -> {store, value}
      _ -> {store, :wrong_type}
    end
  end

  defp fetch_zset(%Store{} = store, key) do
    {store, entry} = fetch_entry(store, key)
    case entry do
      nil -> {store, HashMap.new()}
      %Entry{type: :zset, value: value} -> {store, value}
      _ -> {store, :wrong_type}
    end
  end

  defp fetch_hll(%Store{} = store, key) do
    {store, entry} = fetch_entry(store, key)
    case entry do
      nil -> {store, HyperLogLog.new()}
      %Entry{type: :hll, value: value} -> {store, value}
      _ -> {store, :wrong_type}
    end
  end

  defp parse_int(value) do
    case Integer.parse(value) do
      {int, ""} -> {:ok, int}
      _ -> :error
    end
  end

  defp parse_float(value) do
    case Float.parse(value) do
      {float, ""} -> {:ok, float}
      _ ->
        case Integer.parse(value) do
          {int, ""} -> {:ok, int * 1.0}
          _ -> :error
        end
    end
  end

  defp parse_score_bound("-inf"), do: {:ok, :neg_inf}
  defp parse_score_bound("+inf"), do: {:ok, :pos_inf}
  defp parse_score_bound(value), do: parse_float(value)

  defp split_pairs([]), do: {:ok, []}
  defp split_pairs([_]), do: :error
  defp split_pairs([field, value | rest]) do
    case split_pairs(rest) do
      {:ok, pairs} -> {:ok, [{field, value} | pairs]}
      error -> error
    end
  end

  defp parse_expiry(nil), do: nil
  defp parse_expiry({:seconds, seconds}), do: now_ms() + seconds * 1000
  defp parse_expiry({:millis, millis}), do: now_ms() + millis
  defp parse_expiry({:at_seconds, unix_seconds}), do: unix_seconds * 1000

  defp sorted_zentries(%HashMap{} = zset) do
    zset
    |> HashMap.entries()
    |> Enum.sort_by(fn {member, score} -> {score, member} end)
  end

  defp zrange_slice(entries, start, stop) do
    length = length(entries)
    start = if start < 0, do: length + start, else: start
    stop = if stop < 0, do: length + stop, else: stop
    start = max(start, 0)
    stop = min(stop, length - 1)

    if start > stop or start >= length do
      []
    else
      entries
      |> Enum.slice(start..stop)
    end
  end

  defp score_in_range?(_score, :neg_inf, :pos_inf), do: true
  defp score_in_range?(score, :neg_inf, max), do: score <= max
  defp score_in_range?(score, min, :pos_inf), do: score >= min
  defp score_in_range?(score, min, max), do: score >= min and score <= max

  defp cmd_ping(store, []), do: {store, RESPProtocol.simple_string("PONG")}
  defp cmd_ping(store, [value]), do: {store, RESPProtocol.bulk_string(value)}
  defp cmd_ping(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'PING'")}

  defp cmd_echo(store, [value]), do: {store, RESPProtocol.bulk_string(value)}
  defp cmd_echo(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'ECHO'")}

  defp cmd_set(store, args) do
    case args do
      [key, value | opts] ->
        case parse_set_options(opts) do
          {:ok, expiry, nx?, xx?} ->
            {store, existing} = fetch_entry(store, key)
            exists? = not is_nil(existing)

            cond do
              nx? and exists? -> {store, RESPProtocol.null_bulk_string()}
              xx? and not exists? -> {store, RESPProtocol.null_bulk_string()}
              true ->
                {put_entry(store, key, entry(:string, value, parse_expiry(expiry))), ok()}
            end

          {:error, reply} ->
            {store, reply}
        end

      _ ->
        {store, RESPProtocol.error("ERR wrong number of arguments for 'SET'")}
    end
  end

  defp parse_set_options(opts), do: parse_set_options(opts, nil, false, false)

  defp parse_set_options([], expiry, nx?, xx?) do
    if nx? and xx? do
      {:error, RESPProtocol.error("ERR syntax error")}
    else
      {:ok, expiry, nx?, xx?}
    end
  end

  defp parse_set_options(["NX" | rest], expiry, _nx?, xx?), do: parse_set_options(rest, expiry, true, xx?)
  defp parse_set_options(["XX" | rest], expiry, nx?, _xx?), do: parse_set_options(rest, expiry, nx?, true)

  defp parse_set_options(["EX", seconds | rest], _expiry, nx?, xx?) do
    case parse_int(seconds) do
      {:ok, value} -> parse_set_options(rest, {:seconds, value}, nx?, xx?)
      :error -> {:error, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end

  defp parse_set_options(["PX", millis | rest], _expiry, nx?, xx?) do
    case parse_int(millis) do
      {:ok, value} -> parse_set_options(rest, {:millis, value}, nx?, xx?)
      :error -> {:error, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end

  defp parse_set_options(_bad, _expiry, _nx?, _xx?), do: {:error, RESPProtocol.error("ERR syntax error")}

  defp cmd_get(store, [key]) do
    {store, value} = fetch_string(store, key)
    case value do
      nil -> {store, RESPProtocol.null_bulk_string()}
      :wrong_type -> {store, wrong_type()}
      value -> {store, RESPProtocol.bulk_string(value)}
    end
  end
  defp cmd_get(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'GET'")}

  defp cmd_del(store, args) when args != [] do
    {next_store, count} =
      Enum.reduce(args, {store, 0}, fn key, {acc_store, acc_count} ->
        {updated_store, removed?} = delete_entry(acc_store, key)
        {updated_store, if(removed?, do: acc_count + 1, else: acc_count)}
      end)

    {next_store, RESPProtocol.integer(count)}
  end
  defp cmd_del(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'DEL'")}

  defp cmd_exists(store, args) when args != [] do
    store = purge_expired(store)
    db = current_db(store)
    count = Enum.count(args, fn key -> HashMap.has_key?(db, key) end)
    {store, RESPProtocol.integer(count)}
  end
  defp cmd_exists(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'EXISTS'")}

  defp cmd_type(store, [key]) do
    {store, entry} = fetch_entry(store, key)
    type =
      case entry do
        nil -> "none"
        %Entry{type: type} -> to_string(type)
      end
    {store, RESPProtocol.simple_string(type)}
  end
  defp cmd_type(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'TYPE'")}

  defp cmd_rename(store, [src, dest]) do
    {store, entry} = fetch_entry(store, src)
    case entry do
      nil -> {store, RESPProtocol.error("ERR no such key")}
      _ ->
        {store, _} = delete_entry(store, src)
        {put_entry(store, dest, entry), ok()}
    end
  end
  defp cmd_rename(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'RENAME'")}

  defp cmd_incrby(store, [key, delta_str]) do
    with {:ok, delta} <- parse_int(delta_str) do
      {store, current} = fetch_string(store, key)
      case current do
        :wrong_type -> {store, wrong_type()}
        nil ->
          value = delta
          {put_entry(store, key, entry(:string, Integer.to_string(value))), RESPProtocol.integer(value)}
        current ->
          with {:ok, current_int} <- parse_int(current) do
            value = current_int + delta
            {put_entry(store, key, entry(:string, Integer.to_string(value))), RESPProtocol.integer(value)}
          else
            :error -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
          end
      end
    else
      :error -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_incrby(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'INCRBY'")}

  defp cmd_incr(store, [key]), do: cmd_incrby(store, [key, "1"])
  defp cmd_incr(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'INCR'")}
  defp cmd_decr(store, [key]), do: cmd_incrby(store, [key, "-1"])
  defp cmd_decr(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'DECR'")}

  defp cmd_decrby(store, [key, delta_str]) do
    with {:ok, delta} <- parse_int(delta_str) do
      cmd_incrby(store, [key, Integer.to_string(-delta)])
    else
      :error -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_decrby(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'INCRBY'")}

  defp cmd_append(store, [key, suffix]) do
    {store, current} = fetch_string(store, key)
    case current do
      :wrong_type -> {store, wrong_type()}
      nil ->
        value = suffix
        {put_entry(store, key, entry(:string, value)), RESPProtocol.integer(byte_size(value))}
      current ->
        value = current <> suffix
        {put_entry(store, key, entry(:string, value)), RESPProtocol.integer(byte_size(value))}
    end
  end
  defp cmd_append(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'APPEND'")}

  defp cmd_lpush(store, args) do
    case args do
      [key | values] when values != [] ->
        {store, list} = fetch_list(store, key)
        case list do
          :wrong_type -> {store, wrong_type()}
          list ->
            next_list = Enum.reduce(values, list, &ArrayList.push_left(&2, &1))
            {put_entry(store, key, entry(:list, next_list)), RESPProtocol.integer(ArrayList.len(next_list))}
        end
      _ -> {store, RESPProtocol.error("ERR wrong number of arguments for 'LPUSH'")}
    end
  end

  defp cmd_rpush(store, args) do
    case args do
      [key | values] when values != [] ->
        {store, list} = fetch_list(store, key)
        case list do
          :wrong_type -> {store, wrong_type()}
          list ->
            next_list = Enum.reduce(values, list, &ArrayList.push_right(&2, &1))
            {put_entry(store, key, entry(:list, next_list)), RESPProtocol.integer(ArrayList.len(next_list))}
        end
      _ -> {store, RESPProtocol.error("ERR wrong number of arguments for 'RPUSH'")}
    end
  end

  defp cmd_lpop(store, [key]) do
    {store, entry} = fetch_entry(store, key)

    case entry do
      nil -> {store, RESPProtocol.null_bulk_string()}
      %Entry{type: :list, value: list} ->
        {next_list, value} = ArrayList.pop_left(list)
        case value do
          nil -> {store, RESPProtocol.null_bulk_string()}
          value ->
            next_store = if ArrayList.is_empty(next_list), do: elem(delete_entry(store, key), 0), else: put_entry(store, key, entry(:list, next_list))
            {next_store, RESPProtocol.bulk_string(value)}
        end
      _ -> {store, wrong_type()}
    end
  end
  defp cmd_lpop(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'LPOP'")}

  defp cmd_rpop(store, [key]) do
    {store, entry} = fetch_entry(store, key)

    case entry do
      nil -> {store, RESPProtocol.null_bulk_string()}
      %Entry{type: :list, value: list} ->
        {next_list, value} = ArrayList.pop_right(list)
        case value do
          nil -> {store, RESPProtocol.null_bulk_string()}
          value ->
            next_store = if ArrayList.is_empty(next_list), do: elem(delete_entry(store, key), 0), else: put_entry(store, key, entry(:list, next_list))
            {next_store, RESPProtocol.bulk_string(value)}
        end
      _ -> {store, wrong_type()}
    end
  end
  defp cmd_rpop(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'RPOP'")}

  defp cmd_llen(store, [key]) do
    {store, list} = fetch_list(store, key)
    case list do
      :wrong_type -> {store, wrong_type()}
      list -> {store, RESPProtocol.integer(ArrayList.len(list))}
    end
  end
  defp cmd_llen(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'LLEN'")}

  defp cmd_lrange(store, [key, start_str, stop_str]) do
    with {:ok, start} <- parse_int(start_str),
         {:ok, stop} <- parse_int(stop_str) do
      {store, list} = fetch_list(store, key)
      case list do
        :wrong_type -> {store, wrong_type()}
        list ->
          values =
            list
            |> ArrayList.range(start, stop)
            |> Enum.map(&RESPProtocol.bulk_string/1)
          {store, RESPProtocol.array(values)}
      end
    else
      _ -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_lrange(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'LRANGE'")}

  defp cmd_lindex(store, [key, index_str]) do
    with {:ok, index} <- parse_int(index_str) do
      {store, list} = fetch_list(store, key)
      case list do
        :wrong_type -> {store, wrong_type()}
        list ->
          case ArrayList.index(list, index) do
            nil -> {store, RESPProtocol.null_bulk_string()}
            value -> {store, RESPProtocol.bulk_string(value)}
          end
      end
    else
      _ -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_lindex(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'LINDEX'")}

  defp cmd_hset(store, args) do
    case args do
      [key | field_values] ->
        case split_pairs(field_values) do
          {:ok, pairs} ->
            {store, hash} = fetch_hash(store, key)
            if hash == :wrong_type do
              {store, wrong_type()}
            else
              {next_hash, added} =
                Enum.reduce(pairs, {hash, 0}, fn {field, value}, {acc_hash, count} ->
                  new? = not HashMap.has_key?(acc_hash, field)
                  {HashMap.put(acc_hash, field, value), if(new?, do: count + 1, else: count)}
                end)
              {put_entry(store, key, entry(:hash, next_hash)), RESPProtocol.integer(added)}
            end
          :error -> {store, RESPProtocol.error("ERR wrong number of arguments for 'HSET'")}
        end
      _ -> {store, RESPProtocol.error("ERR wrong number of arguments for 'HSET'")}
    end
  end

  defp cmd_hget(store, [key, field]) do
    {store, hash} = fetch_hash(store, key)
    case hash do
      :wrong_type -> {store, wrong_type()}
      hash ->
        case HashMap.get(hash, field) do
          nil -> {store, RESPProtocol.null_bulk_string()}
          value -> {store, RESPProtocol.bulk_string(value)}
        end
    end
  end
  defp cmd_hget(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'HGET'")}

  defp cmd_hdel(store, args) do
    case args do
      [key | fields] when fields != [] ->
        {store, hash} = fetch_hash(store, key)
        case hash do
          :wrong_type -> {store, wrong_type()}
          hash ->
            {next_hash, count} =
              Enum.reduce(fields, {hash, 0}, fn field, {acc, removed} ->
                if HashMap.has_key?(acc, field) do
                  {HashMap.delete(acc, field), removed + 1}
                else
                  {acc, removed}
                end
              end)
            {put_entry(store, key, entry(:hash, next_hash)), RESPProtocol.integer(count)}
        end
      _ -> {store, RESPProtocol.error("ERR wrong number of arguments for 'HDEL'")}
    end
  end

  defp cmd_hgetall(store, [key]) do
    {store, hash} = fetch_hash(store, key)
    case hash do
      :wrong_type -> {store, wrong_type()}
      hash ->
        flat =
          hash
          |> HashMap.entries()
          |> Enum.flat_map(fn {field, value} -> [RESPProtocol.bulk_string(field), RESPProtocol.bulk_string(value)] end)
        {store, RESPProtocol.array(flat)}
    end
  end
  defp cmd_hgetall(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'HGETALL'")}

  defp cmd_hlen(store, [key]) do
    {store, hash} = fetch_hash(store, key)
    case hash do
      :wrong_type -> {store, wrong_type()}
      hash -> {store, RESPProtocol.integer(HashMap.size(hash))}
    end
  end
  defp cmd_hlen(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'HLEN'")}

  defp cmd_hexists(store, [key, field]) do
    {store, hash} = fetch_hash(store, key)
    case hash do
      :wrong_type -> {store, wrong_type()}
      hash -> {store, RESPProtocol.integer(if(HashMap.has_key?(hash, field), do: 1, else: 0))}
    end
  end
  defp cmd_hexists(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'HEXISTS'")}

  defp cmd_hkeys(store, [key]) do
    {store, hash} = fetch_hash(store, key)
    case hash do
      :wrong_type -> {store, wrong_type()}
      hash -> {store, RESPProtocol.array(Enum.map(HashMap.keys(hash), &RESPProtocol.bulk_string/1))}
    end
  end
  defp cmd_hkeys(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'HKEYS'")}

  defp cmd_hvals(store, [key]) do
    {store, hash} = fetch_hash(store, key)
    case hash do
      :wrong_type -> {store, wrong_type()}
      hash -> {store, RESPProtocol.array(Enum.map(HashMap.values(hash), &RESPProtocol.bulk_string/1))}
    end
  end
  defp cmd_hvals(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'HVALS'")}

  defp cmd_sadd(store, args) do
    case args do
      [key | members] when members != [] ->
        {store, set} = fetch_set(store, key)
        case set do
          :wrong_type -> {store, wrong_type()}
          set ->
            {next_set, count} =
              Enum.reduce(members, {set, 0}, fn member, {acc, added} ->
                if HashSet.has?(acc, member) do
                  {acc, added}
                else
                  {HashSet.add(acc, member), added + 1}
                end
              end)
            {put_entry(store, key, entry(:set, next_set)), RESPProtocol.integer(count)}
        end
      _ -> {store, RESPProtocol.error("ERR wrong number of arguments for 'SADD'")}
    end
  end

  defp cmd_srem(store, args) do
    case args do
      [key | members] when members != [] ->
        {store, set} = fetch_set(store, key)
        case set do
          :wrong_type -> {store, wrong_type()}
          set ->
            {next_set, count} =
              Enum.reduce(members, {set, 0}, fn member, {acc, removed} ->
                if HashSet.has?(acc, member) do
                  {HashSet.delete(acc, member), removed + 1}
                else
                  {acc, removed}
                end
              end)
            {put_entry(store, key, entry(:set, next_set)), RESPProtocol.integer(count)}
        end
      _ -> {store, RESPProtocol.error("ERR wrong number of arguments for 'SREM'")}
    end
  end

  defp cmd_sismember(store, [key, member]) do
    {store, set} = fetch_set(store, key)
    case set do
      :wrong_type -> {store, wrong_type()}
      set -> {store, RESPProtocol.integer(if(HashSet.has?(set, member), do: 1, else: 0))}
    end
  end
  defp cmd_sismember(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'SISMEMBER'")}

  defp cmd_smembers(store, [key]) do
    {store, set} = fetch_set(store, key)
    case set do
      :wrong_type -> {store, wrong_type()}
      set -> {store, RESPProtocol.array(Enum.map(HashSet.values(set), &RESPProtocol.bulk_string/1))}
    end
  end
  defp cmd_smembers(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'SMEMBERS'")}

  defp cmd_scard(store, [key]) do
    {store, set} = fetch_set(store, key)
    case set do
      :wrong_type -> {store, wrong_type()}
      set -> {store, RESPProtocol.integer(HashSet.size(set))}
    end
  end
  defp cmd_scard(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'SCARD'")}

  defp cmd_sunion(store, args) when args != [] do
    case Enum.reduce_while(args, {store, HashSet.new()}, fn key, {acc_store, acc_set} ->
           {next_store, set} = fetch_set(acc_store, key)
           case set do
             :wrong_type -> {:halt, {:error, next_store}}
             set -> {:cont, {next_store, HashSet.union(acc_set, set)}}
           end
         end) do
      {:error, store} -> {store, wrong_type()}
      {store, set} -> {store, RESPProtocol.array(Enum.sort(HashSet.values(set)) |> Enum.map(&RESPProtocol.bulk_string/1))}
    end
  end
  defp cmd_sunion(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'SUNION'")}

  defp cmd_sinter(store, args) when args != [] do
    case Enum.reduce_while(args, {store, nil}, fn key, {acc_store, acc_set} ->
           {next_store, set} = fetch_set(acc_store, key)
           case set do
             :wrong_type -> {:halt, {:error, next_store}}
             set ->
               next_set =
                 case acc_set do
                   nil -> set
                   acc -> HashSet.intersection(acc, set)
                 end

               {:cont, {next_store, next_set}}
           end
         end) do
      {:error, store} -> {store, wrong_type()}
      {store, nil} -> {store, RESPProtocol.array([])}
      {store, set} -> {store, RESPProtocol.array(Enum.sort(HashSet.values(set)) |> Enum.map(&RESPProtocol.bulk_string/1))}
    end
  end
  defp cmd_sinter(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'SINTER'")}

  defp cmd_sdiff(store, args) when args != [] do
    case Enum.reduce_while(args, {store, nil}, fn key, {acc_store, acc_set} ->
           {next_store, set} = fetch_set(acc_store, key)
           case set do
             :wrong_type -> {:halt, {:error, next_store}}
             set ->
               next_set =
                 case acc_set do
                   nil -> set
                   acc -> HashSet.difference(acc, set)
                 end

               {:cont, {next_store, next_set}}
           end
         end) do
      {:error, store} -> {store, wrong_type()}
      {store, nil} -> {store, RESPProtocol.array([])}
      {store, set} -> {store, RESPProtocol.array(Enum.sort(HashSet.values(set)) |> Enum.map(&RESPProtocol.bulk_string/1))}
    end
  end
  defp cmd_sdiff(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'SDIFF'")}

  defp cmd_zadd(store, args) do
    case args do
      [key | rest] ->
        case split_pairs(rest) do
          {:ok, pairs} ->
            {store, zset} = fetch_zset(store, key)
            case zset do
              :wrong_type -> {store, wrong_type()}
              zset ->
                case Enum.reduce_while(pairs, {zset, 0}, fn {score_str, member}, {acc, added} ->
                       case parse_float(score_str) do
                         {:ok, score} ->
                           new? = not HashMap.has_key?(acc, member)
                           {:cont, {HashMap.put(acc, member, score), if(new?, do: added + 1, else: added)}}

                         :error ->
                           {:halt, {:error, RESPProtocol.error("ERR value is not a valid float")}}
                       end
                     end) do
                  {:error, reply} -> {store, reply}
                  {next_zset, count} -> {put_entry(store, key, entry(:zset, next_zset)), RESPProtocol.integer(count)}
                end
            end
          :error -> {store, RESPProtocol.error("ERR wrong number of arguments for 'ZADD'")}
        end
      _ -> {store, RESPProtocol.error("ERR wrong number of arguments for 'ZADD'")}
    end
  end

  defp cmd_zrem(store, args) do
    case args do
      [key | members] when members != [] ->
        {store, zset} = fetch_zset(store, key)
        case zset do
          :wrong_type -> {store, wrong_type()}
          zset ->
            {next_zset, count} =
              Enum.reduce(members, {zset, 0}, fn member, {acc, removed} ->
                if HashMap.has_key?(acc, member) do
                  {HashMap.delete(acc, member), removed + 1}
                else
                  {acc, removed}
                end
              end)
            {put_entry(store, key, entry(:zset, next_zset)), RESPProtocol.integer(count)}
        end
      _ -> {store, RESPProtocol.error("ERR wrong number of arguments for 'ZREM'")}
    end
  end

  defp cmd_zcard(store, [key]) do
    {store, zset} = fetch_zset(store, key)
    case zset do
      :wrong_type -> {store, wrong_type()}
      zset -> {store, RESPProtocol.integer(HashMap.size(zset))}
    end
  end
  defp cmd_zcard(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'ZCARD'")}

  defp cmd_zrange(store, [key, start_str, stop_str]) do
    with {:ok, start} <- parse_int(start_str),
         {:ok, stop} <- parse_int(stop_str) do
      {store, zset} = fetch_zset(store, key)
      case zset do
        :wrong_type -> {store, wrong_type()}
        zset ->
          values =
            zset
            |> sorted_zentries()
            |> zrange_slice(start, stop)
            |> Enum.flat_map(fn {member, score} -> [RESPProtocol.bulk_string(member), RESPProtocol.bulk_string(Float.to_string(score))] end)
          {store, RESPProtocol.array(values)}
      end
    else
      _ -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_zrange(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'ZRANGE'")}

  defp cmd_zrevrange(store, [key, start_str, stop_str]) do
    with {:ok, start} <- parse_int(start_str),
         {:ok, stop} <- parse_int(stop_str) do
      {store, zset} = fetch_zset(store, key)
      case zset do
        :wrong_type -> {store, wrong_type()}
        zset ->
          values =
            zset
            |> sorted_zentries()
            |> Enum.reverse()
            |> zrange_slice(start, stop)
            |> Enum.flat_map(fn {member, score} -> [RESPProtocol.bulk_string(member), RESPProtocol.bulk_string(Float.to_string(score))] end)
          {store, RESPProtocol.array(values)}
      end
    else
      _ -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_zrevrange(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'ZREVRANGE'")}

  defp cmd_zrangebyscore(store, [key, min_str, max_str]) do
    with {:ok, min} <- parse_score_bound(min_str),
         {:ok, max} <- parse_score_bound(max_str) do
      {store, zset} = fetch_zset(store, key)
      case zset do
        :wrong_type -> {store, wrong_type()}
        zset ->
          values =
            zset
            |> sorted_zentries()
            |> Enum.filter(fn {_member, score} -> score_in_range?(score, min, max) end)
            |> Enum.flat_map(fn {member, score} -> [RESPProtocol.bulk_string(member), RESPProtocol.bulk_string(Float.to_string(score))] end)
          {store, RESPProtocol.array(values)}
      end
    else
      _ -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_zrangebyscore(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'ZRANGEBYSCORE'")}

  defp cmd_zscore(store, [key, member]) do
    {store, zset} = fetch_zset(store, key)
    case zset do
      :wrong_type -> {store, wrong_type()}
      zset ->
        case HashMap.get(zset, member) do
          nil -> {store, RESPProtocol.null_bulk_string()}
          score -> {store, RESPProtocol.bulk_string(Float.to_string(score))}
        end
    end
  end
  defp cmd_zscore(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'ZSCORE'")}

  defp cmd_zrank(store, [key, member]) do
    {store, zset} = fetch_zset(store, key)
    case zset do
      :wrong_type -> {store, wrong_type()}
      zset ->
        rank =
          zset
          |> sorted_zentries()
          |> Enum.find_index(fn {m, _} -> m == member end)
        case rank do
          nil -> {store, RESPProtocol.null_bulk_string()}
          rank -> {store, RESPProtocol.integer(rank)}
        end
    end
  end
  defp cmd_zrank(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'ZRANK'")}

  defp cmd_pfadd(store, args) do
    case args do
      [key | members] when members != [] ->
        {store, hll} = fetch_hll(store, key)
        case hll do
          :wrong_type -> {store, wrong_type()}
          hll ->
            updated = Enum.reduce(members, hll, &HyperLogLog.add(&2, &1))
            changed = updated != hll
            {put_entry(store, key, entry(:hll, updated)), RESPProtocol.integer(if(changed, do: 1, else: 0))}
        end
      _ -> {store, RESPProtocol.error("ERR wrong number of arguments for 'PFADD'")}
    end
  end

  defp cmd_pfcount(store, keys) when keys != [] do
    {store, merged} =
      Enum.reduce(keys, {store, nil}, fn key, {acc_store, acc_hll} ->
        {next_store, hll} = fetch_hll(acc_store, key)
        case hll do
          :wrong_type -> {next_store, :wrong_type}
          nil -> {next_store, acc_hll}
          hll ->
            merged = case acc_hll do
              nil -> hll
              :wrong_type -> :wrong_type
              acc -> HyperLogLog.merge(acc, hll)
            end
            {next_store, merged}
        end
      end)
    case merged do
      :wrong_type -> {store, wrong_type()}
      nil -> {store, RESPProtocol.integer(0)}
      hll -> {store, RESPProtocol.integer(HyperLogLog.count(hll))}
    end
  end
  defp cmd_pfcount(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'PFCOUNT'")}

  defp cmd_pfmerge(store, [dest | sources]) when sources != [] do
    {store, merged} =
      Enum.reduce(sources, {store, HyperLogLog.new()}, fn key, {acc_store, acc_hll} ->
        {next_store, hll} = fetch_hll(acc_store, key)
        case hll do
          :wrong_type -> {next_store, :wrong_type}
          nil -> {next_store, acc_hll}
          hll -> {next_store, HyperLogLog.merge(acc_hll, hll)}
        end
      end)
    case merged do
      :wrong_type -> {store, wrong_type()}
      hll -> {put_entry(store, dest, entry(:hll, hll)), ok()}
    end
  end
  defp cmd_pfmerge(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'PFMERGE'")}

  defp cmd_expire(store, [key, seconds_str]) do
    with {:ok, seconds} <- parse_int(seconds_str) do
      {store, entry} = fetch_entry(store, key)
      case entry do
        nil -> {store, RESPProtocol.integer(0)}
        %Entry{} = entry ->
          {put_entry(store, key, %{entry | expires_at_ms: now_ms() + seconds * 1000}), RESPProtocol.integer(1)}
      end
    else
      _ -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_expire(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'EXPIRE'")}

  defp cmd_expireat(store, [key, unix_seconds_str]) do
    with {:ok, unix_seconds} <- parse_int(unix_seconds_str) do
      {store, entry} = fetch_entry(store, key)
      case entry do
        nil -> {store, RESPProtocol.integer(0)}
        %Entry{} = entry ->
          {put_entry(store, key, %{entry | expires_at_ms: parse_expiry({:at_seconds, unix_seconds})}), RESPProtocol.integer(1)}
      end
    else
      _ -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_expireat(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'EXPIREAT'")}

  defp cmd_ttl(store, [key]) do
    {store, entry} = fetch_entry(store, key)
    case entry do
      nil -> {store, RESPProtocol.integer(-2)}
      %Entry{expires_at_ms: nil} -> {store, RESPProtocol.integer(-1)}
      %Entry{expires_at_ms: expires_at} -> {store, RESPProtocol.integer(max(div(expires_at - now_ms(), 1000), -2))}
    end
  end
  defp cmd_ttl(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'TTL'")}

  defp cmd_pttl(store, [key]) do
    {store, entry} = fetch_entry(store, key)
    case entry do
      nil -> {store, RESPProtocol.integer(-2)}
      %Entry{expires_at_ms: nil} -> {store, RESPProtocol.integer(-1)}
      %Entry{expires_at_ms: expires_at} -> {store, RESPProtocol.integer(max(expires_at - now_ms(), -2))}
    end
  end
  defp cmd_pttl(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'PTTL'")}

  defp cmd_persist(store, [key]) do
    {store, entry} = fetch_entry(store, key)
    case entry do
      nil -> {store, RESPProtocol.integer(0)}
      %Entry{expires_at_ms: nil} -> {store, RESPProtocol.integer(0)}
      %Entry{} = entry -> {put_entry(store, key, %{entry | expires_at_ms: nil}), RESPProtocol.integer(1)}
    end
  end
  defp cmd_persist(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'PERSIST'")}

  defp cmd_select(%Store{} = store, [index_str]) do
    with {:ok, index} <- parse_int(index_str) do
      if index < 0 do
        {store, RESPProtocol.error("ERR invalid DB index")}
      else
        {%{select_db(store, index) | dbs: Map.put_new(store.dbs, index, HashMap.new())}, ok()}
      end
    else
      _ -> {store, RESPProtocol.error("ERR value is not an integer or out of range")}
    end
  end
  defp cmd_select(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'SELECT'")}

  defp cmd_flushdb(%Store{} = store, []),
    do: {%{store | dbs: Map.put(store.dbs, store.selected_db, HashMap.new()), key_indexes: Map.put(store.key_indexes, store.selected_db, RadixTree.new())}, ok()}
  defp cmd_flushdb(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'FLUSHDB'")}

  defp cmd_flushall(%Store{} = store, []), do: {%{store | dbs: %{0 => HashMap.new()}, key_indexes: %{0 => RadixTree.new()}, selected_db: 0}, ok()}
  defp cmd_flushall(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'FLUSHALL'")}

  defp cmd_dbsize(%Store{} = store, []) do
    store = purge_expired(store)
    {store, RESPProtocol.integer(HashMap.size(current_db(store)))}
  end
  defp cmd_dbsize(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'DBSIZE'")}

  defp cmd_info(%Store{} = store, []) do
    store = purge_expired(store)
    db = current_db(store)
    info = "db=#{store.selected_db} keys=#{HashMap.size(db)}"
    {store, RESPProtocol.bulk_string(info)}
  end
  defp cmd_info(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'INFO'")}

  defp cmd_keys(%Store{} = store, [pattern]) do
    store = purge_expired(store)
    keys = matching_keys(store, pattern) |> Enum.map(&RESPProtocol.bulk_string/1)
    {store, RESPProtocol.array(keys)}
  end
  defp cmd_keys(store, _), do: {store, RESPProtocol.error("ERR wrong number of arguments for 'KEYS'")}

  defp matching_keys(store, "*"), do: RadixTree.keys(current_key_index(store))

  defp matching_keys(store, pattern) do
    if prefix_pattern?(pattern) do
      pattern
      |> String.trim_trailing("*")
      |> then(&RadixTree.words_with_prefix(current_key_index(store), &1))
    else
      db = current_db(store)
      regex = glob_to_regex(pattern)

      db
      |> HashMap.keys()
      |> Enum.filter(&Regex.match?(regex, &1))
      |> Enum.sort()
    end
  end

  defp prefix_pattern?(pattern) do
    String.ends_with?(pattern, "*") and not String.contains?(String.trim_trailing(pattern, "*"), ["*", "?", "["])
  end

  defp glob_to_regex(pattern) do
    pattern
    |> Regex.escape()
    |> String.replace("\\*", ".*")
    |> then(&Regex.compile!("^" <> &1 <> "$"))
  end
end
