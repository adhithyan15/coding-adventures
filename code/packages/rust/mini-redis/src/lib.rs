//! DT25 Mini-Redis.
//!
//! This crate combines the RESP protocol, TCP server, and a pure command
//! layer to provide a single-node Redis-compatible in-memory server.
//!
//! The design goal is to build the common Redis surface with our own packages
//! first, then add distributed features later. That means the crate currently
//! focuses on core command semantics, storage, TTLs, and AOF replay rather
//! than replication, cluster mode, pub/sub, scripting, or RESP3. The backend
//! logic is exposed through a transport-agnostic interface so tests and future
//! clients can talk to the engine without opening sockets.

mod commands;
mod server;
mod store;
mod types;

pub use commands::{
    active_expire, cmd_append, cmd_dbsize, cmd_del, cmd_echo, cmd_exists,
    cmd_expire, cmd_expireat, cmd_flushall, cmd_flushdb, cmd_get, cmd_hdel,
    cmd_hexists, cmd_hget, cmd_hgetall, cmd_hkeys, cmd_hlen, cmd_hvals, cmd_hset,
    cmd_info, cmd_incr, cmd_incrby, cmd_keys, cmd_lindex, cmd_llen, cmd_lpop,
    cmd_lrange, cmd_lpush, cmd_pfadd, cmd_pfcount, cmd_pfmerge, cmd_persist,
    cmd_ping, cmd_pttl, cmd_rpop, cmd_rpush, cmd_scard, cmd_select, cmd_set,
    cmd_sinter, cmd_smembers, cmd_srem, cmd_sdiff, cmd_sismember, cmd_sunion,
    cmd_ttl, cmd_type, cmd_zadd, cmd_zcard, cmd_zrange, cmd_zrangebyscore,
    cmd_zrank, cmd_zrem, cmd_zscore, dispatch, is_mutating,
};
pub use server::{MiniRedis, MiniRedisEngine, RedisBackend};
pub use store::{current_time_ms, Database, Store};
pub use types::{Entry, EntryType, EntryValue, OrderedF64, SortedSet};
