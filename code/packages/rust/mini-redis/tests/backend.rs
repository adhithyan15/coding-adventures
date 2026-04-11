use mini_redis::MiniRedis;
use resp_protocol::RespValue;

struct BackendHarness {
    app: MiniRedis,
}

impl BackendHarness {
    fn new() -> Self {
        Self {
            app: MiniRedis::new(0),
        }
    }

    fn call(&self, command: &[&[u8]]) -> RespValue {
        let parts = command.iter().map(|part| part.to_vec()).collect();
        self.app.execute_owned(parts)
    }

    fn call_strs(&self, command: &str, args: &[&str]) -> RespValue {
        let mut parts = Vec::with_capacity(args.len() + 1);
        parts.push(command.as_bytes().to_vec());
        parts.extend(args.iter().map(|arg| arg.as_bytes().to_vec()));
        self.app.execute_owned(parts)
    }
}

fn bulk(value: impl AsRef<[u8]>) -> RespValue {
    RespValue::BulkString(Some(value.as_ref().to_vec()))
}

fn simple(value: &str) -> RespValue {
    RespValue::SimpleString(value.to_string())
}

fn integer(value: i64) -> RespValue {
    RespValue::Integer(value)
}

fn array(values: &[&str]) -> RespValue {
    RespValue::Array(Some(values.iter().map(|value| bulk(*value)).collect()))
}

fn kv_array(values: &[(&str, &str)]) -> RespValue {
    let mut out = Vec::with_capacity(values.len() * 2);
    for (key, value) in values {
        out.push(bulk(*key));
        out.push(bulk(*value));
    }
    RespValue::Array(Some(out))
}

#[test]
fn backend_executes_string_hash_set_and_sorted_set_commands() {
    let app = BackendHarness::new();

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
    let app = BackendHarness::new();

    assert_eq!(app.call_strs("SELECT", &["1"]), simple("OK"));
    assert_eq!(app.call_strs("SET", &["db:key", "value"]), simple("OK"));
    assert_eq!(app.call_strs("DBSIZE", &[]), integer(1));
    assert_eq!(app.call_strs("SELECT", &["0"]), simple("OK"));
    assert_eq!(app.call_strs("DBSIZE", &[]), integer(0));
    assert_eq!(app.call_strs("SELECT", &["1"]), simple("OK"));
    assert_eq!(app.call_strs("GET", &["db:key"]), bulk("value"));

    assert_eq!(app.call_strs("PFADD", &["visitors", "a", "b", "c"]), integer(1));
    assert_eq!(app.call_strs("PFCOUNT", &["visitors"]), integer(3));

    assert_eq!(app.call_strs("EXPIRE", &["db:key", "0"]), integer(1));
    assert_eq!(app.call_strs("GET", &["db:key"]), RespValue::BulkString(None));
}
