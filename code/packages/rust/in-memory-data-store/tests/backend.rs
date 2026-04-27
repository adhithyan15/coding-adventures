use in_memory_data_store::{DataStoreBackend, DataStoreEngine};
use in_memory_data_store_protocol::EngineResponse;

struct BackendClient<B: DataStoreBackend> {
    backend: B,
}

impl<B: DataStoreBackend> BackendClient<B> {
    fn new(backend: B) -> Self {
        Self { backend }
    }

    fn call(&self, command: &[&[u8]]) -> EngineResponse {
        let parts = command.iter().map(|part| part.to_vec()).collect();
        self.backend.execute_owned(parts)
    }

    fn call_strs(&self, command: &str, args: &[&str]) -> EngineResponse {
        let mut parts = Vec::with_capacity(args.len() + 1);
        parts.push(command.as_bytes().to_vec());
        parts.extend(args.iter().map(|arg| arg.as_bytes().to_vec()));
        self.backend.execute_owned(parts)
    }

    fn active_expire_all(&self) {
        self.backend.active_expire_all();
    }
}

fn bulk(value: impl AsRef<[u8]>) -> EngineResponse {
    EngineResponse::BulkString(Some(value.as_ref().to_vec()))
}

fn simple(value: &str) -> EngineResponse {
    EngineResponse::SimpleString(value.to_string())
}

fn integer(value: i64) -> EngineResponse {
    EngineResponse::Integer(value)
}

fn array(values: &[&str]) -> EngineResponse {
    EngineResponse::Array(Some(values.iter().map(|value| bulk(*value)).collect()))
}

fn kv_array(values: &[(&str, &str)]) -> EngineResponse {
    let mut out = Vec::with_capacity(values.len() * 2);
    for (key, value) in values {
        out.push(bulk(*key));
        out.push(bulk(*value));
    }
    EngineResponse::Array(Some(out))
}

fn error_message(value: &EngineResponse) -> Option<&str> {
    match value {
        EngineResponse::Error(err) => Some(err.as_str()),
        _ => None,
    }
}

fn assert_error_contains(value: EngineResponse, needle: &str) {
    let Some(message) = error_message(&value) else {
        panic!("expected error containing {needle:?}, got {value:?}");
    };
    assert!(
        message.contains(needle),
        "expected error containing {needle:?}, got {message:?}"
    );
}

#[test]
fn backend_executes_string_hash_set_and_sorted_set_commands() {
    let app = BackendClient::new(DataStoreEngine::new(None).expect("failed to create backend"));

    assert_eq!(app.call(&[b"PING".as_ref()]), simple("PONG"));
    assert_eq!(app.call_strs("SET", &["alpha", "1"]), simple("OK"));
    assert_eq!(app.call_strs("GET", &["alpha"]), bulk("1"));

    assert_eq!(app.call_strs("HSET", &["hash", "b", "2", "a", "1"]), integer(2));
    assert_eq!(app.call_strs("HGET", &["hash", "a"]), bulk("1"));
    assert_eq!(
        app.call_strs("HGETALL", &["hash"]),
        kv_array(&[("a", "1"), ("b", "2")])
    );

    assert_eq!(app.call_strs("SADD", &["set", "c", "a", "b"]), integer(3));
    assert_eq!(app.call_strs("SMEMBERS", &["set"]), array(&["a", "b", "c"]));

    assert_eq!(app.call_strs("ZADD", &["scores", "2", "b", "1", "a"]), integer(2));
    assert_eq!(app.call_strs("ZRANGE", &["scores", "0", "-1"]), array(&["a", "b"]));
    assert_eq!(app.call_strs("ZRANK", &["scores", "b"]), integer(1));
    assert_eq!(app.call_strs("ZSCORE", &["scores", "a"]), bulk("1"));
}

#[test]
fn backend_tracks_databases_ttls_and_hlls_without_tcp() {
    let app = BackendClient::new(DataStoreEngine::new(None).expect("failed to create backend"));

    assert_eq!(app.call_strs("SET", &["db:key", "value"]), simple("OK"));
    assert_eq!(app.call_strs("DBSIZE", &[]), integer(1));
    assert_eq!(app.call_strs("GET", &["db:key"]), bulk("value"));

    assert_eq!(app.call_strs("PFADD", &["visitors", "a", "b", "c"]), integer(1));
    assert_eq!(app.call_strs("PFCOUNT", &["visitors"]), integer(3));
    assert_eq!(app.call_strs("DBSIZE", &[]), integer(2));

    assert_eq!(app.call_strs("EXPIRE", &["db:key", "0"]), integer(1));
    app.active_expire_all();
    assert_eq!(app.call_strs("DBSIZE", &[]), integer(1));
    assert_eq!(app.call_strs("GET", &["db:key"]), EngineResponse::BulkString(None));
}

#[test]
fn upstream_string_cases_cover_basic_lifecycle_and_conditionals() {
    let app = BackendClient::new(DataStoreEngine::new(None).expect("failed to create backend"));

    assert_eq!(app.call_strs("SET", &["x", "foobar"]), simple("OK"));
    assert_eq!(app.call_strs("GET", &["x"]), bulk("foobar"));
    assert_eq!(app.call_strs("SET", &["empty", ""]), simple("OK"));
    assert_eq!(app.call_strs("GET", &["empty"]), bulk(""));

    assert_eq!(app.call_strs("APPEND", &["x", "bar"]), integer(9));
    assert_eq!(app.call_strs("GET", &["x"]), bulk("foobarbar"));

    assert_eq!(app.call_strs("INCR", &["counter"]), integer(1));
    assert_eq!(app.call_strs("INCRBY", &["counter", "41"]), integer(42));
    assert_eq!(app.call_strs("DECR", &["counter"]), integer(41));
    assert_eq!(app.call_strs("DECRBY", &["counter", "1"]), integer(40));

    assert_eq!(app.call_strs("SET", &["conditional", "1", "NX"]), simple("OK"));
    assert_eq!(
        app.call_strs("SET", &["conditional", "2", "NX"]),
        EngineResponse::BulkString(None)
    );
    assert_eq!(app.call_strs("SET", &["conditional", "3", "XX"]), simple("OK"));
    assert_eq!(app.call_strs("GET", &["conditional"]), bulk("3"));

    assert_eq!(app.call_strs("RENAME", &["x", "renamed"]), simple("OK"));
    assert_eq!(app.call_strs("GET", &["x"]), EngineResponse::BulkString(None));
    assert_eq!(app.call_strs("GET", &["renamed"]), bulk("foobarbar"));
}

#[test]
fn upstream_hash_cases_cover_fields_keys_values_and_deletes() {
    let app = BackendClient::new(DataStoreEngine::new(None).expect("failed to create backend"));

    assert_eq!(
        app.call_strs("HSET", &["hash", "a", "1", "b", "2", "c", "3"]),
        integer(3)
    );
    assert_eq!(app.call_strs("HLEN", &["hash"]), integer(3));
    assert_eq!(app.call_strs("HGET", &["hash", "b"]), bulk("2"));
    assert_eq!(
        app.call_strs("HGETALL", &["hash"]),
        kv_array(&[("a", "1"), ("b", "2"), ("c", "3")])
    );
    assert_eq!(app.call_strs("HKEYS", &["hash"]), array(&["a", "b", "c"]));
    assert_eq!(app.call_strs("HVALS", &["hash"]), array(&["1", "2", "3"]));
    assert_eq!(app.call_strs("HEXISTS", &["hash", "c"]), integer(1));
    assert_eq!(app.call_strs("HDEL", &["hash", "b", "c"]), integer(2));
    assert_eq!(
        app.call_strs("HGETALL", &["hash"]),
        kv_array(&[("a", "1")])
    );
    assert_eq!(app.call_strs("HDEL", &["hash", "a"]), integer(1));
    assert_eq!(app.call_strs("HGET", &["hash", "a"]), EngineResponse::BulkString(None));

    assert_eq!(app.call_strs("LPUSH", &["wrongtype", "foo"]), integer(1));
    assert_error_contains(app.call_strs("HSET", &["wrongtype", "bar", "baz"]), "WRONGTYPE");
}

#[test]
fn upstream_set_cases_cover_membership_cardinality_and_errors() {
    let app = BackendClient::new(DataStoreEngine::new(None).expect("failed to create backend"));

    assert_eq!(app.call_strs("SADD", &["set", "foo", "bar"]), integer(2));
    assert_eq!(app.call_strs("SADD", &["set", "foo"]), integer(0));
    assert_eq!(app.call_strs("SCARD", &["set"]), integer(2));
    assert_eq!(app.call_strs("SISMEMBER", &["set", "foo"]), integer(1));
    assert_eq!(app.call_strs("SISMEMBER", &["set", "baz"]), integer(0));
    assert_eq!(app.call_strs("SMEMBERS", &["set"]), array(&["bar", "foo"]));
    assert_eq!(app.call_strs("SREM", &["set", "foo", "baz"]), integer(1));
    assert_eq!(app.call_strs("SCARD", &["set"]), integer(1));

    assert_eq!(app.call_strs("LPUSH", &["wrongtype", "foo"]), integer(1));
    assert_error_contains(app.call_strs("SADD", &["wrongtype", "bar"]), "WRONGTYPE");
}

#[test]
fn upstream_sorted_set_cases_cover_ordering_scores_and_deletes() {
    let app = BackendClient::new(DataStoreEngine::new(None).expect("failed to create backend"));

    assert_eq!(
        app.call_strs("ZADD", &["scores", "10", "x", "20", "y", "30", "z"]),
        integer(3)
    );
    assert_eq!(app.call_strs("ZRANGE", &["scores", "0", "-1"]), array(&["x", "y", "z"]));

    assert_eq!(app.call_strs("ZADD", &["scores", "1", "y"]), integer(0));
    assert_eq!(app.call_strs("ZRANGE", &["scores", "0", "-1"]), array(&["y", "x", "z"]));
    assert_eq!(app.call_strs("ZRANGEBYSCORE", &["scores", "1", "20"]), array(&["y", "x"]));
    assert_eq!(app.call_strs("ZRANK", &["scores", "x"]), integer(1));
    assert_eq!(app.call_strs("ZSCORE", &["scores", "y"]), bulk("1"));
    assert_eq!(app.call_strs("ZCARD", &["scores"]), integer(3));
    assert_eq!(app.call_strs("ZREM", &["scores", "y", "z"]), integer(2));
    assert_eq!(app.call_strs("ZCARD", &["scores"]), integer(1));
    assert_eq!(app.call_strs("ZRANGE", &["scores", "0", "-1"]), array(&["x"]));

    assert_eq!(app.call_strs("LPUSH", &["wrongtype", "foo"]), integer(1));
    assert_error_contains(app.call_strs("ZADD", &["wrongtype", "1", "bar"]), "WRONGTYPE");
}

#[test]
fn upstream_expire_and_persist_cases_cover_immediate_and_absolute_expiry() {
    let app = BackendClient::new(DataStoreEngine::new(None).expect("failed to create backend"));

    assert_eq!(app.call_strs("SET", &["ttl", "value"]), simple("OK"));
    assert_eq!(app.call_strs("TTL", &["ttl"]), integer(-1));
    assert_eq!(app.call_strs("PERSIST", &["ttl"]), integer(0));
    assert_eq!(app.call_strs("EXPIRE", &["ttl", "0"]), integer(1));
    assert_eq!(app.call_strs("GET", &["ttl"]), EngineResponse::BulkString(None));
    assert_eq!(app.call_strs("TTL", &["ttl"]), integer(-2));

    assert_eq!(app.call_strs("SET", &["persisted", "value", "EX", "100"]), simple("OK"));
    assert_eq!(app.call_strs("PERSIST", &["persisted"]), integer(1));
    assert_eq!(app.call_strs("TTL", &["persisted"]), integer(-1));
    assert_eq!(app.call_strs("GET", &["persisted"]), bulk("value"));

    assert_eq!(app.call_strs("SET", &["past", "value"]), simple("OK"));
    assert_eq!(app.call_strs("EXPIREAT", &["past", "1"]), integer(1));
    assert_eq!(app.call_strs("GET", &["past"]), EngineResponse::BulkString(None));
}

#[test]
fn upstream_hyperloglog_cases_cover_cardinality_and_merge() {
    let app = BackendClient::new(DataStoreEngine::new(None).expect("failed to create backend"));

    assert_eq!(app.call_strs("PFADD", &["hll", "a", "b", "c"]), integer(1));
    assert_eq!(app.call_strs("PFADD", &["hll", "a", "b", "c"]), integer(0));
    assert_eq!(app.call_strs("PFCOUNT", &["hll"]), integer(3));

    assert_eq!(app.call_strs("PFADD", &["hll2", "b", "c", "d"]), integer(1));
    assert_eq!(app.call_strs("PFMERGE", &["merged", "hll", "hll2"]), simple("OK"));
    assert_eq!(app.call_strs("PFCOUNT", &["merged"]), integer(4));
}

#[test]
fn upstream_keyspace_cases_cover_keys_dbsize_exists_and_flush() {
    let app = BackendClient::new(DataStoreEngine::new(None).expect("failed to create backend"));

    for key in ["key_x", "key_y", "key_z", "foo_a", "foo_b", "foo_c"] {
        assert_eq!(app.call_strs("SET", &[key, "hello"]), simple("OK"));
    }

    assert_eq!(app.call_strs("KEYS", &["foo*"]), array(&["foo_a", "foo_b", "foo_c"]));
    assert_eq!(
        app.call_strs("KEYS", &["*"]),
        array(&["foo_a", "foo_b", "foo_c", "key_x", "key_y", "key_z"])
    );
    assert_eq!(app.call_strs("DBSIZE", &[]), integer(6));
    assert_eq!(app.call_strs("EXISTS", &["key_x", "missing"]), integer(1));

    assert_eq!(app.call_strs("SET", &["emptykey", ""]), simple("OK"));
    assert_eq!(app.call_strs("GET", &["emptykey"]), bulk(""));
    assert_eq!(app.call_strs("EXISTS", &["emptykey"]), integer(1));
    assert_eq!(app.call_strs("DEL", &["emptykey"]), integer(1));
    assert_eq!(app.call_strs("EXISTS", &["emptykey"]), integer(0));

    assert_eq!(app.call_strs("FLUSHDB", &[]), simple("OK"));
    assert_eq!(app.call_strs("DBSIZE", &[]), integer(0));

    assert_eq!(app.call_strs("SET", &["all1", "one"]), simple("OK"));
    assert_eq!(app.call_strs("SET", &["all2", "two"]), simple("OK"));
    assert_eq!(app.call_strs("FLUSHALL", &[]), simple("OK"));
    assert_eq!(app.call_strs("DBSIZE", &[]), integer(0));
}
