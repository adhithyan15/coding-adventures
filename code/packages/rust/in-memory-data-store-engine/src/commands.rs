use in_memory_data_store_protocol::EngineResponse;
use std::collections::VecDeque;

use hash_map::HashMap as DtHashMap;
use hash_set::HashSet as DtHashSet;
use hyperloglog::HyperLogLog;


use crate::engine_core::DataStoreEngine;
use crate::store::{current_time_ms, Store};
use crate::types::{Entry, EntryValue, SortedSet};

macro_rules! register {
    ($engine:expr, $name:literal, $mutating:expr, $skip_lazy_expire:expr, $handler:ident) => {
        $engine.register_command($name, $mutating, $skip_lazy_expire, $handler);
    };
}

pub fn register_builtin_commands(engine: &DataStoreEngine) {
    register!(engine, "PING", false, true, cmd_ping);
    register!(engine, "ECHO", false, true, cmd_echo);
    register!(engine, "SET", true, false, cmd_set);
    register!(engine, "GET", false, false, cmd_get);
    register!(engine, "DEL", true, false, cmd_del);
    register!(engine, "EXISTS", false, false, cmd_exists);
    register!(engine, "TYPE", false, false, cmd_type);
    register!(engine, "RENAME", true, false, cmd_rename);
    register!(engine, "INCR", true, false, cmd_incr);
    register!(engine, "DECR", true, false, cmd_decr);
    register!(engine, "INCRBY", true, false, cmd_incrby);
    register!(engine, "DECRBY", true, false, cmd_decrby);
    register!(engine, "APPEND", true, false, cmd_append);
    register!(engine, "HSET", true, false, cmd_hset);
    register!(engine, "HGET", false, false, cmd_hget);
    register!(engine, "HDEL", true, false, cmd_hdel);
    register!(engine, "HGETALL", false, false, cmd_hgetall);
    register!(engine, "HLEN", false, false, cmd_hlen);
    register!(engine, "HEXISTS", false, false, cmd_hexists);
    register!(engine, "HKEYS", false, false, cmd_hkeys);
    register!(engine, "HVALS", false, false, cmd_hvals);
    register!(engine, "LPUSH", true, false, cmd_lpush);
    register!(engine, "RPUSH", true, false, cmd_rpush);
    register!(engine, "LPOP", true, false, cmd_lpop);
    register!(engine, "RPOP", true, false, cmd_rpop);
    register!(engine, "LLEN", false, false, cmd_llen);
    register!(engine, "LRANGE", false, false, cmd_lrange);
    register!(engine, "LINDEX", false, false, cmd_lindex);
    register!(engine, "SADD", true, false, cmd_sadd);
    register!(engine, "SREM", true, false, cmd_srem);
    register!(engine, "SISMEMBER", false, false, cmd_sismember);
    register!(engine, "SMEMBERS", false, false, cmd_smembers);
    register!(engine, "SCARD", false, false, cmd_scard);
    register!(engine, "SUNION", false, false, cmd_sunion);
    register!(engine, "SINTER", false, false, cmd_sinter);
    register!(engine, "SDIFF", false, false, cmd_sdiff);
    register!(engine, "ZADD", true, false, cmd_zadd);
    register!(engine, "ZRANGE", false, false, cmd_zrange);
    register!(engine, "ZRANGEBYSCORE", false, false, cmd_zrangebyscore);
    register!(engine, "ZRANK", false, false, cmd_zrank);
    register!(engine, "ZSCORE", false, false, cmd_zscore);
    register!(engine, "ZCARD", false, false, cmd_zcard);
    register!(engine, "ZREM", true, false, cmd_zrem);
    register!(engine, "PFADD", true, false, cmd_pfadd);
    register!(engine, "PFCOUNT", false, false, cmd_pfcount);
    register!(engine, "PFMERGE", true, false, cmd_pfmerge);
    register!(engine, "EXPIRE", true, false, cmd_expire);
    register!(engine, "EXPIREAT", true, false, cmd_expireat);
    register!(engine, "TTL", false, false, cmd_ttl);
    register!(engine, "PTTL", false, false, cmd_pttl);
    register!(engine, "PERSIST", true, false, cmd_persist);
    register!(engine, "SELECT", true, true, cmd_select);
    register!(engine, "FLUSHDB", true, true, cmd_flushdb);
    register!(engine, "FLUSHALL", true, true, cmd_flushall);
    register!(engine, "DBSIZE", false, true, cmd_dbsize);
    register!(engine, "INFO", false, true, cmd_info);
    register!(engine, "KEYS", false, false, cmd_keys);
}

pub fn dispatch(store: Store, parts: &[Vec<u8>]) -> (Store, EngineResponse) {
    if parts.is_empty() {
        return (store, err("ERR empty command"));
    }

    let engine = DataStoreEngine::from_store(store);
    let response = engine.execute_parts(parts);
    (engine.store(), response)
}

pub fn is_mutating(parts: &[Vec<u8>]) -> bool {
    if parts.is_empty() {
        return false;
    }
    matches!(
        ascii_upper(&parts[0]).as_str(),
        "SET"
            | "DEL"
            | "RENAME"
            | "INCR"
            | "DECR"
            | "INCRBY"
            | "DECRBY"
            | "APPEND"
            | "HSET"
            | "HDEL"
            | "LPUSH"
            | "RPUSH"
            | "LPOP"
            | "RPOP"
            | "SADD"
            | "SREM"
            | "ZADD"
            | "ZREM"
            | "PFADD"
            | "PFMERGE"
            | "EXPIRE"
            | "EXPIREAT"
            | "PERSIST"
            | "SELECT"
            | "FLUSHDB"
            | "FLUSHALL"
    )
}

pub fn active_expire(store: Store) -> Store {
    store.active_expire_all()
}

pub fn cmd_ping(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [] => (store, EngineResponse::SimpleString("PONG".to_string())),
        [message] => (store, EngineResponse::BulkString(Some(message.clone()))),
        _ => (store, err("ERR wrong number of arguments for 'PING'")),
    }
}

pub fn cmd_echo(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [message] => (store, EngineResponse::BulkString(Some(message.clone()))),
        _ => (store, err("ERR wrong number of arguments for 'ECHO'")),
    }
}

pub fn cmd_set(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 2 {
        return (store, err("ERR wrong number of arguments for 'SET'"));
    }
    let key = args[0].clone();
    let value = args[1].clone();
    let mut expires_at = None;
    let mut nx = false;
    let mut xx = false;
    let mut i = 2;
    while i < args.len() {
        let opt = ascii_upper(&args[i]);
        match opt.as_str() {
            "EX" if i + 1 < args.len() => {
                let seconds = match parse_i64(&args[i + 1]) {
                    Ok(value) => value,
                    Err(err) => return (store, err),
                };
                expires_at = Some(expiration_from_seconds(seconds));
                i += 2;
            }
            "PX" if i + 1 < args.len() => {
                let millis = match parse_i64(&args[i + 1]) {
                    Ok(value) => value,
                    Err(err) => return (store, err),
                };
                expires_at = Some(expiration_from_millis(millis));
                i += 2;
            }
            "NX" => {
                nx = true;
                i += 1;
            }
            "XX" => {
                xx = true;
                i += 1;
            }
            _ => return (store, err("ERR syntax error")),
        }
    }
    if nx && xx {
        return (store, err("ERR syntax error"));
    }

    let exists = store.get(&key).is_some();
    if nx && exists {
        return (store, EngineResponse::BulkString(None));
    }
    if xx && !exists {
        return (store, EngineResponse::BulkString(None));
    }

    let entry = Entry::string(value, expires_at);
    (store.set(key, entry), ok())
}

pub fn cmd_get(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::String(bytes) => (store, EngineResponse::BulkString(Some(bytes.clone()))),
                _ => (store, wrong_type()),
            },
            None => (store, EngineResponse::BulkString(None)),
        },
        _ => (store, err("ERR wrong number of arguments for 'GET'")),
    }
}

pub fn cmd_del(mut store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'DEL'"));
    }
    let mut removed = 0i64;
    for key in args {
        if store.get(key).is_some() {
            removed += 1;
            store = store.delete(key);
        }
    }
    (store, integer(removed))
}

pub fn cmd_exists(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'EXISTS'"));
    }
    let count = args.iter().filter(|key| store.get(key).is_some()).count() as i64;
    (store, integer(count))
}

pub fn cmd_type(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.type_of(key) {
            Some(entry_type) => (store, EngineResponse::SimpleString(entry_type.to_string())),
            None => (store, EngineResponse::SimpleString("none".to_string())),
        },
        _ => (store, err("ERR wrong number of arguments for 'TYPE'")),
    }
}

pub fn cmd_rename(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [src, dst] => {
            let Some(entry) = store.get(src).cloned() else {
                return (store, err("ERR no such key"));
            };
            let mut store = store.delete(src);
            store = store.set(dst.clone(), entry);
            (store, ok())
        }
        _ => (store, err("ERR wrong number of arguments for 'RENAME'")),
    }
}

pub fn cmd_incr(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => cmd_incrby(store, &[key.clone(), b"1".to_vec()]),
        _ => (store, err("ERR wrong number of arguments for 'INCR'")),
    }
}

pub fn cmd_decr(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => cmd_incrby(store, &[key.clone(), b"-1".to_vec()]),
        _ => (store, err("ERR wrong number of arguments for 'DECR'")),
    }
}

pub fn cmd_incrby(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, delta_bytes] => match parse_i64(delta_bytes) {
            Ok(delta) => adjust_integer(store, key.clone(), delta),
            Err(err) => (store, err),
        },
        _ => (store, err("ERR wrong number of arguments for 'INCRBY'")),
    }
}

pub fn cmd_decrby(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, delta_bytes] => match parse_i64(delta_bytes) {
            Ok(delta) => adjust_integer(store, key.clone(), -delta),
            Err(err) => (store, err),
        },
        _ => (store, err("ERR wrong number of arguments for 'DECRBY'")),
    }
}

pub fn cmd_append(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, suffix] => {
            let expires_at = store.get(key).and_then(|entry| entry.expires_at);
            let mut value = match store.get(key) {
                Some(entry) => match &entry.value {
                    EntryValue::String(bytes) => bytes.clone(),
                    _ => return (store, wrong_type()),
                },
                None => Vec::new(),
            };
            value.extend_from_slice(suffix);
            let len = value.len() as i64;
            let store = store.set(key.clone(), Entry::string(value, expires_at));
            (store, integer(len))
        }
        _ => (store, err("ERR wrong number of arguments for 'APPEND'")),
    }
}

pub fn cmd_hset(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 3 || args.len() % 2 == 0 {
        return (store, err("ERR wrong number of arguments for 'HSET'"));
    }
    let key = args[0].clone();
    let expires_at = store.get(&key).and_then(|entry| entry.expires_at);
    let mut map = match store.get(&key) {
        Some(entry) => match &entry.value {
            EntryValue::Hash(map) => map.clone(),
            _ => return (store, wrong_type()),
        },
        None => DtHashMap::default(),
    };
    let mut added = 0i64;
    for pair in args[1..].chunks(2) {
        let field = pair[0].clone();
        let value = pair[1].clone();
        if !map.has(&field) {
            added += 1;
        }
        map = map.set(field, value);
    }
    let store = store.set(key, Entry::hash(map, expires_at));
    (store, integer(added))
}

pub fn cmd_hget(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, field] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => match map.get(field) {
                    Some(value) => (store, EngineResponse::BulkString(Some(value.clone()))),
                    None => (store, EngineResponse::BulkString(None)),
                },
                _ => (store, wrong_type()),
            },
            None => (store, EngineResponse::BulkString(None)),
        },
        _ => (store, err("ERR wrong number of arguments for 'HGET'")),
    }
}

pub fn cmd_hdel(mut store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 2 {
        return (store, err("ERR wrong number of arguments for 'HDEL'"));
    }
    let key = &args[0];
    let expires_at = store.get(key).and_then(|entry| entry.expires_at);
    let mut map = match store.get(key) {
        Some(entry) => match &entry.value {
            EntryValue::Hash(map) => map.clone(),
            _ => return (store, wrong_type()),
        },
        None => return (store, integer(0)),
    };
    let mut removed = 0i64;
    for field in &args[1..] {
        if map.has(field) {
            removed += 1;
            map = map.delete(field);
        }
    }
    if map.size() == 0 {
        store = store.delete(key);
    } else {
        store = store.set(key.clone(), Entry::hash(map, expires_at));
    }
    (store, integer(removed))
}

pub fn cmd_hgetall(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => {
                    let mut entries = map.entries();
                    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                    let mut out = Vec::with_capacity(entries.len() * 2);
                    for (field, value) in entries {
                        out.push(EngineResponse::BulkString(Some(field.clone())));
                        out.push(EngineResponse::BulkString(Some(value.clone())));
                    }
                    (store, EngineResponse::Array(Some(out)))
                }
                _ => (store, wrong_type()),
            },
            None => (store, EngineResponse::Array(Some(Vec::new()))),
        },
        _ => (store, err("ERR wrong number of arguments for 'HGETALL'")),
    }
}

pub fn cmd_hlen(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => (store, integer(map.size() as i64)),
                _ => (store, wrong_type()),
            },
            None => (store, integer(0)),
        },
        _ => (store, err("ERR wrong number of arguments for 'HLEN'")),
    }
}

pub fn cmd_hexists(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, field] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => (store, integer(map.has(field) as i64)),
                _ => (store, wrong_type()),
            },
            None => (store, integer(0)),
        },
        _ => (store, err("ERR wrong number of arguments for 'HEXISTS'")),
    }
}

pub fn cmd_hkeys(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => (
                    store,
                    EngineResponse::Array(Some(
                        {
                            let mut keys = map.keys();
                            keys.sort();
                            keys
                        }
                        .into_iter()
                        .map(|field| EngineResponse::BulkString(Some(field)))
                        .collect(),
                    )),
                ),
                _ => (store, wrong_type()),
            },
            None => (store, EngineResponse::Array(Some(Vec::new()))),
        },
        _ => (store, err("ERR wrong number of arguments for 'HKEYS'")),
    }
}

pub fn cmd_hvals(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => (
                    store,
                    EngineResponse::Array(Some(
                        {
                            let mut entries = map.entries();
                            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                            entries
                        }
                        .into_iter()
                        .map(|(_, value)| EngineResponse::BulkString(Some(value)))
                        .collect(),
                    )),
                ),
                _ => (store, wrong_type()),
            },
            None => (store, EngineResponse::Array(Some(Vec::new()))),
        },
        _ => (store, err("ERR wrong number of arguments for 'HVALS'")),
    }
}

pub fn cmd_lpush(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 2 {
        return (store, err("ERR wrong number of arguments for 'LPUSH'"));
    }
    let key = args[0].clone();
    let expires_at = store.get(&key).and_then(|entry| entry.expires_at);
    let mut list = match store.get(&key) {
        Some(entry) => match &entry.value {
            EntryValue::List(list) => list.clone(),
            _ => return (store, wrong_type()),
        },
        None => VecDeque::new(),
    };
    for value in &args[1..] {
        list.push_front(value.clone());
    }
    let len = list.len() as i64;
    let store = store.set(key, Entry::list(list, expires_at));
    (store, integer(len))
}

pub fn cmd_rpush(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 2 {
        return (store, err("ERR wrong number of arguments for 'RPUSH'"));
    }
    let key = args[0].clone();
    let expires_at = store.get(&key).and_then(|entry| entry.expires_at);
    let mut list = match store.get(&key) {
        Some(entry) => match &entry.value {
            EntryValue::List(list) => list.clone(),
            _ => return (store, wrong_type()),
        },
        None => VecDeque::new(),
    };
    for value in &args[1..] {
        list.push_back(value.clone());
    }
    let len = list.len() as i64;
    let store = store.set(key, Entry::list(list, expires_at));
    (store, integer(len))
}

pub fn cmd_lpop(mut store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => {
            let expires_at = store.get(key).and_then(|entry| entry.expires_at);
            let mut list = match store.get(key) {
                Some(entry) => match &entry.value {
                    EntryValue::List(list) => list.clone(),
                    _ => return (store, wrong_type()),
                },
                None => return (store, EngineResponse::BulkString(None)),
            };
            let value = list.pop_front();
            if list.is_empty() {
                store = store.delete(key);
            } else {
                store = store.set(key.clone(), Entry::list(list, expires_at));
            }
            (
                store,
                value.map_or(EngineResponse::BulkString(None), |v| {
                    EngineResponse::BulkString(Some(v))
                }),
            )
        }
        _ => (store, err("ERR wrong number of arguments for 'LPOP'")),
    }
}

pub fn cmd_rpop(mut store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => {
            let expires_at = store.get(key).and_then(|entry| entry.expires_at);
            let mut list = match store.get(key) {
                Some(entry) => match &entry.value {
                    EntryValue::List(list) => list.clone(),
                    _ => return (store, wrong_type()),
                },
                None => return (store, EngineResponse::BulkString(None)),
            };
            let value = list.pop_back();
            if list.is_empty() {
                store = store.delete(key);
            } else {
                store = store.set(key.clone(), Entry::list(list, expires_at));
            }
            (
                store,
                value.map_or(EngineResponse::BulkString(None), |v| {
                    EngineResponse::BulkString(Some(v))
                }),
            )
        }
        _ => (store, err("ERR wrong number of arguments for 'RPOP'")),
    }
}

pub fn cmd_llen(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::List(list) => (store, integer(list.len() as i64)),
                _ => (store, wrong_type()),
            },
            None => (store, integer(0)),
        },
        _ => (store, err("ERR wrong number of arguments for 'LLEN'")),
    }
}

pub fn cmd_lrange(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, start, end] => {
            let start = match parse_isize(start) {
                Ok(value) => value,
                Err(err) => return (store, err),
            };
            let end = match parse_isize(end) {
                Ok(value) => value,
                Err(err) => return (store, err),
            };
            match store.get(key) {
                Some(entry) => match &entry.value {
                    EntryValue::List(list) => {
                        let len = list.len() as isize;
                        let start = if start < 0 { len + start } else { start };
                        let end = if end < 0 { len + end } else { end };
                        if start < 0 || end < 0 || start >= len || start > end {
                            return (store, EngineResponse::Array(Some(Vec::new())));
                        }
                        let slice = list
                            .iter()
                            .skip(start as usize)
                            .take((end - start + 1) as usize)
                            .cloned()
                            .map(|v| EngineResponse::BulkString(Some(v)))
                            .collect();
                        (store, EngineResponse::Array(Some(slice)))
                    }
                    _ => (store, wrong_type()),
                },
                None => (store, EngineResponse::Array(Some(Vec::new()))),
            }
        }
        _ => (store, err("ERR wrong number of arguments for 'LRANGE'")),
    }
}

pub fn cmd_lindex(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, index] => {
            let index = match parse_isize(index) {
                Ok(value) => value,
                Err(err) => return (store, err),
            };
            match store.get(key) {
                Some(entry) => match &entry.value {
                    EntryValue::List(list) => {
                        let len = list.len() as isize;
                        let index = if index < 0 { len + index } else { index };
                        if index < 0 || index >= len {
                            return (store, EngineResponse::BulkString(None));
                        }
                        let value = list.get(index as usize).cloned();
                        (
                            store,
                            value.map_or(EngineResponse::BulkString(None), |v| {
                                EngineResponse::BulkString(Some(v))
                            }),
                        )
                    }
                    _ => (store, wrong_type()),
                },
                None => (store, EngineResponse::BulkString(None)),
            }
        }
        _ => (store, err("ERR wrong number of arguments for 'LINDEX'")),
    }
}

pub fn cmd_sadd(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 2 {
        return (store, err("ERR wrong number of arguments for 'SADD'"));
    }
    let key = args[0].clone();
    let expires_at = store.get(&key).and_then(|entry| entry.expires_at);
    let mut set = match store.get(&key) {
        Some(entry) => match &entry.value {
            EntryValue::Set(set) => set.clone(),
            _ => return (store, wrong_type()),
        },
        None => DtHashSet::new(),
    };
    let mut added = 0i64;
    for member in &args[1..] {
        if !set.contains(member) {
            added += 1;
        }
        set = set.add(member.clone());
    }
    let store = store.set(key, Entry::set(set, expires_at));
    (store, integer(added))
}

pub fn cmd_srem(mut store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 2 {
        return (store, err("ERR wrong number of arguments for 'SREM'"));
    }
    let key = &args[0];
    let expires_at = store.get(key).and_then(|entry| entry.expires_at);
    let mut set = match store.get(key) {
        Some(entry) => match &entry.value {
            EntryValue::Set(set) => set.clone(),
            _ => return (store, wrong_type()),
        },
        None => return (store, integer(0)),
    };
    let mut removed = 0i64;
    for member in &args[1..] {
        if set.contains(member) {
            removed += 1;
            set = set.remove(member);
        }
    }
    if set.is_empty() {
        store = store.delete(key);
    } else {
        store = store.set(key.clone(), Entry::set(set, expires_at));
    }
    (store, integer(removed))
}

pub fn cmd_sismember(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, member] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Set(set) => (store, integer(set.contains(member) as i64)),
                _ => (store, wrong_type()),
            },
            None => (store, integer(0)),
        },
        _ => (store, err("ERR wrong number of arguments for 'SISMEMBER'")),
    }
}

pub fn cmd_smembers(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Set(set) => (
                    store,
                    EngineResponse::Array(Some(
                        {
                            let mut members = set.to_list();
                            members.sort();
                            members
                        }
                        .into_iter()
                        .map(|member| EngineResponse::BulkString(Some(member)))
                        .collect(),
                    )),
                ),
                _ => (store, wrong_type()),
            },
            None => (store, EngineResponse::Array(Some(Vec::new()))),
        },
        _ => (store, err("ERR wrong number of arguments for 'SMEMBERS'")),
    }
}

pub fn cmd_scard(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Set(set) => (store, integer(set.len() as i64)),
                _ => (store, wrong_type()),
            },
            None => (store, integer(0)),
        },
        _ => (store, err("ERR wrong number of arguments for 'SCARD'")),
    }
}

pub fn cmd_sunion(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'SUNION'"));
    }
    let mut out = DtHashSet::new();
    for key in args {
        if let Some(entry) = store.get(key) {
            match &entry.value {
                EntryValue::Set(set) => {
                    out = out.union(set.clone());
                }
                _ => return (store, wrong_type()),
            }
        }
    }
    let mut members = out.to_list();
    members.sort();
    (
        store,
        EngineResponse::Array(Some(
            members
                .into_iter()
                .map(|member| EngineResponse::BulkString(Some(member)))
                .collect(),
        )),
    )
}

pub fn cmd_sinter(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'SINTER'"));
    }
    let mut iter = args.iter();
    let first = iter.next().unwrap();
    let mut out = match store.get(first) {
        Some(entry) => match &entry.value {
            EntryValue::Set(set) => set.clone(),
            _ => return (store, wrong_type()),
        },
        None => DtHashSet::new(),
    };
    for key in iter {
        let set = match store.get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Set(set) => set.clone(),
                _ => return (store, wrong_type()),
            },
            None => DtHashSet::new(),
        };
        out = out.intersection(set);
    }
    let mut members = out.to_list();
    members.sort();
    (
        store,
        EngineResponse::Array(Some(
            members
                .into_iter()
                .map(|member| EngineResponse::BulkString(Some(member)))
                .collect(),
        )),
    )
}

pub fn cmd_sdiff(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'SDIFF'"));
    }
    let mut iter = args.iter();
    let first = iter.next().unwrap();
    let mut out = match store.get(first) {
        Some(entry) => match &entry.value {
            EntryValue::Set(set) => set.clone(),
            _ => return (store, wrong_type()),
        },
        None => DtHashSet::new(),
    };
    for key in iter {
        let set = match store.get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Set(set) => set.clone(),
                _ => return (store, wrong_type()),
            },
            None => DtHashSet::new(),
        };
        out = out.difference(set);
    }
    let mut members = out.to_list();
    members.sort();
    (
        store,
        EngineResponse::Array(Some(
            members
                .into_iter()
                .map(|member| EngineResponse::BulkString(Some(member)))
                .collect(),
        )),
    )
}

pub fn cmd_zadd(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 3 || args.len() % 2 == 0 {
        return (store, err("ERR wrong number of arguments for 'ZADD'"));
    }
    let key = args[0].clone();
    let expires_at = store.get(&key).and_then(|entry| entry.expires_at);
    let mut zset = match store.get(&key) {
        Some(entry) => match &entry.value {
            EntryValue::ZSet(zset) => zset.clone(),
            _ => return (store, wrong_type()),
        },
        None => SortedSet::new(),
    };
    let mut added = 0i64;
    for pair in args[1..].chunks(2) {
        let score = match parse_f64(&pair[0]) {
            Ok(value) => value,
            Err(err) => return (store, err),
        };
        if zset.insert(score, pair[1].clone()) {
            added += 1;
        }
    }
    let store = store.set(key, Entry::zset(zset, expires_at));
    (store, integer(added))
}

pub fn cmd_zrange(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, start, end] => {
            let start = match parse_isize(start) {
                Ok(value) => value,
                Err(err) => return (store, err),
            };
            let end = match parse_isize(end) {
                Ok(value) => value,
                Err(err) => return (store, err),
            };
            match store.clone().get(key) {
                Some(entry) => match &entry.value {
                    EntryValue::ZSet(zset) => (
                        store,
                        EngineResponse::Array(Some(
                            zset.range_by_index(start, end)
                                .into_iter()
                                .map(|(member, _)| EngineResponse::BulkString(Some(member)))
                                .collect(),
                        )),
                    ),
                    _ => (store, wrong_type()),
                },
                None => (store, EngineResponse::Array(Some(Vec::new()))),
            }
        }
        _ => (store, err("ERR wrong number of arguments for 'ZRANGE'")),
    }
}

pub fn cmd_zrangebyscore(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, min, max] => {
            let min = match parse_f64(min) {
                Ok(value) => value,
                Err(err) => return (store, err),
            };
            let max = match parse_f64(max) {
                Ok(value) => value,
                Err(err) => return (store, err),
            };
            match store.clone().get(key) {
                Some(entry) => match &entry.value {
                    EntryValue::ZSet(zset) => (
                        store,
                        EngineResponse::Array(Some(
                            zset.range_by_score(min, max)
                                .into_iter()
                                .map(|(member, _)| EngineResponse::BulkString(Some(member)))
                                .collect(),
                        )),
                    ),
                    _ => (store, wrong_type()),
                },
                None => (store, EngineResponse::Array(Some(Vec::new()))),
            }
        }
        _ => (
            store,
            err("ERR wrong number of arguments for 'ZRANGEBYSCORE'"),
        ),
    }
}

pub fn cmd_zrank(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, member] => match store.get(key) {
            Some(entry) => match &entry.value {
                EntryValue::ZSet(zset) => match zset.rank(member) {
                    Some(rank) => (store, integer(rank as i64)),
                    None => (store, EngineResponse::BulkString(None)),
                },
                _ => (store, wrong_type()),
            },
            None => (store, EngineResponse::BulkString(None)),
        },
        _ => (store, err("ERR wrong number of arguments for 'ZRANK'")),
    }
}

pub fn cmd_zscore(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, member] => match store.get(key) {
            Some(entry) => match &entry.value {
                EntryValue::ZSet(zset) => match zset.score(member) {
                    Some(score) => (
                        store,
                        EngineResponse::BulkString(Some(score.to_string().into_bytes())),
                    ),
                    None => (store, EngineResponse::BulkString(None)),
                },
                _ => (store, wrong_type()),
            },
            None => (store, EngineResponse::BulkString(None)),
        },
        _ => (store, err("ERR wrong number of arguments for 'ZSCORE'")),
    }
}

pub fn cmd_zcard(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::ZSet(zset) => (store, integer(zset.len() as i64)),
                _ => (store, wrong_type()),
            },
            None => (store, integer(0)),
        },
        _ => (store, err("ERR wrong number of arguments for 'ZCARD'")),
    }
}

pub fn cmd_zrem(mut store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 2 {
        return (store, err("ERR wrong number of arguments for 'ZREM'"));
    }
    let key = &args[0];
    let expires_at = store.get(key).and_then(|entry| entry.expires_at);
    let mut zset = match store.get(key) {
        Some(entry) => match &entry.value {
            EntryValue::ZSet(zset) => zset.clone(),
            _ => return (store, wrong_type()),
        },
        None => return (store, integer(0)),
    };
    let mut removed = 0i64;
    for member in &args[1..] {
        if zset.remove(member) {
            removed += 1;
        }
    }
    if zset.is_empty() {
        store = store.delete(key);
    } else {
        store = store.set(key.clone(), Entry::zset(zset, expires_at));
    }
    (store, integer(removed))
}

pub fn cmd_pfadd(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 2 {
        return (store, err("ERR wrong number of arguments for 'PFADD'"));
    }
    let key = args[0].clone();
    let expires_at = store.get(&key).and_then(|entry| entry.expires_at);
    let mut hll = match store.get(&key) {
        Some(entry) => match &entry.value {
            EntryValue::Hll(hll) => hll.clone(),
            _ => return (store, wrong_type()),
        },
        None => HyperLogLog::new(),
    };
    let before = hll.clone();
    for element in &args[1..] {
        hll.add_bytes(element);
    }
    let changed = before != hll;
    let store = store.set(key, Entry::hll(hll, expires_at));
    (store, integer(changed as i64))
}

pub fn cmd_pfcount(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'PFCOUNT'"));
    }
    let mut iter = args.iter();
    let first = iter.next().unwrap();
    let mut hll = match store.get(first) {
        Some(entry) => match &entry.value {
            EntryValue::Hll(hll) => hll.clone(),
            _ => return (store, wrong_type()),
        },
        None => HyperLogLog::new(),
    };
    for key in iter {
        match store.get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hll(other) => {
                    hll = hll.merge(other);
                }
                _ => return (store, wrong_type()),
            },
            None => {}
        }
    }
    (store, integer(hll.count() as i64))
}

pub fn cmd_pfmerge(mut store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if args.len() < 2 {
        return (store, err("ERR wrong number of arguments for 'PFMERGE'"));
    }
    let dest = args[0].clone();
    let mut merged = HyperLogLog::new();
    for key in &args[1..] {
        if let Some(entry) = store.get(key) {
            match &entry.value {
                EntryValue::Hll(hll) => {
                    merged = merged.merge(hll);
                }
                _ => return (store, wrong_type()),
            }
        }
    }
    let expires_at = store.get(&dest).and_then(|entry| entry.expires_at);
    store = store.set(dest, Entry::hll(merged, expires_at));
    (store, ok())
}

pub fn cmd_expire(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, seconds] => match parse_i64(seconds) {
            Ok(seconds) => set_expiration(store, key.clone(), expiration_from_seconds(seconds)),
            Err(err) => (store, err),
        },
        _ => (store, err("ERR wrong number of arguments for 'EXPIRE'")),
    }
}

pub fn cmd_expireat(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key, timestamp] => match parse_i64(timestamp) {
            Ok(timestamp) => set_expiration(
                store,
                key.clone(),
                expiration_from_seconds(timestamp - unix_now_s()),
            ),
            Err(err) => (store, err),
        },
        _ => (store, err("ERR wrong number of arguments for 'EXPIREAT'")),
    }
}

pub fn cmd_ttl(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => ttl_like(store, key, false),
        _ => (store, err("ERR wrong number of arguments for 'TTL'")),
    }
}

pub fn cmd_pttl(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => ttl_like(store, key, true),
        _ => (store, err("ERR wrong number of arguments for 'PTTL'")),
    }
}

pub fn cmd_persist(mut store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [key] => {
            let Some(entry) = store.get(key).cloned() else {
                return (store, integer(0));
            };
            if entry.expires_at.is_none() {
                return (store, integer(0));
            }
            let db_index = store.active_db;
            let mut db = store.current_db().clone();
            let mut updated = entry;
            updated.expires_at = None;
            db = db.set(key.clone(), updated);
            store.databases[db_index] = db;
            (store, integer(1))
        }
        _ => (store, err("ERR wrong number of arguments for 'PERSIST'")),
    }
}

pub fn cmd_select(mut store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [index] => match parse_usize(index) {
            Ok(index) if index < store.databases.len() => {
                store.active_db = index;
                (store, ok())
            }
            Ok(_) => (store, err("ERR DB index out of range")),
            Err(err) => (store, err),
        },
        _ => (store, err("ERR wrong number of arguments for 'SELECT'")),
    }
}

pub fn cmd_flushdb(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if !args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'FLUSHDB'"));
    }
    (store.flushdb(), ok())
}

pub fn cmd_flushall(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if !args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'FLUSHALL'"));
    }
    (store.flushall(), ok())
}

pub fn cmd_dbsize(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if !args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'DBSIZE'"));
    }
    let size = store.dbsize();
    (store, integer(size as i64))
}

pub fn cmd_info(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    if !args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'INFO'"));
    }
    let info = format!(
        "# Server\r\nmini_redis_version:0.1.0\r\nactive_db:{}\r\ndbsize:{}\r\n",
        store.active_db,
        store.dbsize()
    );
    (store, EngineResponse::BulkString(Some(info.into_bytes())))
}

pub fn cmd_keys(store: Store, args: &[Vec<u8>]) -> (Store, EngineResponse) {
    match args {
        [pattern] => (
            store.clone(),
            EngineResponse::Array(Some(
                store
                    .keys(pattern)
                    .into_iter()
                    .map(|key| EngineResponse::BulkString(Some(key)))
                    .collect(),
            )),
        ),
        _ => (store, err("ERR wrong number of arguments for 'KEYS'")),
    }
}

fn adjust_integer(store: Store, key: Vec<u8>, delta: i64) -> (Store, EngineResponse) {
    let expires_at = store.get(&key).and_then(|entry| entry.expires_at);
    let current = match store.get(&key) {
        Some(entry) => match &entry.value {
            EntryValue::String(bytes) => match parse_i64(bytes) {
                Ok(value) => value,
                Err(err) => return (store, err),
            },
            _ => return (store, wrong_type()),
        },
        None => 0,
    };
    let new_value = match current.checked_add(delta) {
        Some(value) => value,
        None => return (store, err("ERR increment or decrement would overflow")),
    };
    let store = store.set(
        key,
        Entry::string(new_value.to_string().into_bytes(), expires_at),
    );
    (store, integer(new_value))
}

fn ttl_like(store: Store, key: &[u8], milliseconds: bool) -> (Store, EngineResponse) {
    match store.get(key) {
        None => (store, integer(-2)),
        Some(entry) => match entry.expires_at {
            None => (store, integer(-1)),
            Some(expires_at) => {
                let now = current_time_ms();
                if now >= expires_at {
                    return (store.delete(key), integer(-2));
                }
                let remaining = expires_at - now;
                if milliseconds {
                    (store, integer(remaining as i64))
                } else {
                    (store, integer((remaining / 1000) as i64))
                }
            }
        },
    }
}

fn set_expiration(mut store: Store, key: Vec<u8>, expires_at: u64) -> (Store, EngineResponse) {
    if store.get(&key).is_none() {
        return (store, integer(0));
    }
    if expires_at <= current_time_ms() {
        return (store.delete(&key), integer(1));
    }
    let db_index = store.active_db;
    let mut db = store.current_db().clone();
    if let Some(mut current) = db.entries.get(&key).cloned() {
        current.expires_at = Some(expires_at);
        db = db.set(key.clone(), current);
    }
    db.ttl_heap.push((expires_at, key));
    store.databases[db_index] = db;
    (store, integer(1))
}

fn ok() -> EngineResponse {
    EngineResponse::SimpleString("OK".to_string())
}

fn integer(value: i64) -> EngineResponse {
    EngineResponse::Integer(value)
}

fn err(message: impl Into<String>) -> EngineResponse {
    EngineResponse::error(message)
}

fn wrong_type() -> EngineResponse {
    err("WRONGTYPE Operation against a key holding the wrong kind of value")
}

fn parse_i64(bytes: &[u8]) -> Result<i64, EngineResponse> {
    std::str::from_utf8(bytes)
        .map_err(|_| err("ERR value is not an integer or out of range"))
        .and_then(|text: &str| { text.parse::<i64>()
                .map_err(|_| err("ERR value is not an integer or out of range"))
        })
}

fn parse_isize(bytes: &[u8]) -> Result<isize, EngineResponse> {
    std::str::from_utf8(bytes)
        .map_err(|_| err("ERR value is not an integer or out of range"))
        .and_then(|text: &str| { text.parse::<isize>()
                .map_err(|_| err("ERR value is not an integer or out of range"))
        })
}

fn parse_usize(bytes: &[u8]) -> Result<usize, EngineResponse> {
    std::str::from_utf8(bytes)
        .map_err(|_| err("ERR value is not an integer or out of range"))
        .and_then(|text: &str| { text.parse::<usize>()
                .map_err(|_| err("ERR value is not an integer or out of range"))
        })
}

fn parse_f64(bytes: &[u8]) -> Result<f64, EngineResponse> {
    std::str::from_utf8(bytes)
        .map_err(|_| err("ERR value is not a valid float"))
        .and_then(|text: &str| { text.parse::<f64>()
                .map_err(|_| err("ERR value is not a valid float"))
        })
}

fn unix_now_s() -> i64 {
    (current_time_ms() / 1000) as i64
}

fn expiration_from_seconds(seconds: i64) -> u64 {
    if seconds <= 0 {
        current_time_ms()
    } else {
        current_time_ms().saturating_add((seconds as u64).saturating_mul(1000))
    }
}

fn expiration_from_millis(milliseconds: i64) -> u64 {
    if milliseconds <= 0 {
        current_time_ms()
    } else {
        current_time_ms().saturating_add(milliseconds as u64)
    }
}

fn ascii_upper(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| byte.to_ascii_uppercase() as char)
        .collect()
}

#[allow(dead_code)]
pub fn bulk_string_bytes(value: &EngineResponse) -> Option<Vec<u8>> {
    match value {
        EngineResponse::BulkString(Some(bytes)) => Some(bytes.clone()),
        EngineResponse::SimpleString(text) => Some(text.as_bytes().to_vec()),
        EngineResponse::Integer(n) => Some(n.to_string().into_bytes()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(store: Store, command: &str, args: &[&[u8]]) -> (Store, EngineResponse) {
        let mut parts = vec![command.as_bytes().to_vec()];
        parts.extend(args.iter().map(|arg| arg.to_vec()));
        dispatch(store, &parts)
    }

    fn bulk(value: &str) -> EngineResponse {
        EngineResponse::BulkString(Some(value.as_bytes().to_vec()))
    }

    fn array(values: &[&str]) -> EngineResponse {
        EngineResponse::Array(Some(values.iter().map(|value| bulk(value)).collect()))
    }

    fn assert_error(resp: EngineResponse, expected: &str) {
        match resp {
            EngineResponse::Error(err) => assert_eq!(err, expected),
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn string_commands_cover_basic_lifecycle() {
        let store = Store::empty();

        let (store, resp) = run(store, "SET", &[b"greeting", b"hello"]);
        assert_eq!(resp, ok());

        let (store, resp) = run(store, "GET", &[b"greeting"]);
        assert_eq!(resp, bulk("hello"));

        let (store, resp) = run(store, "SET", &[b"greeting", b"world", b"NX"]);
        assert_eq!(resp, EngineResponse::BulkString(None));

        let (store, resp) = run(store, "SET", &[b"greeting", b"world", b"XX"]);
        assert_eq!(resp, ok());

        let (store, resp) = run(store, "GET", &[b"greeting"]);
        assert_eq!(resp, bulk("world"));

        let (store, resp) = run(store, "SET", &[b"user:2", b"two"]);
        assert_eq!(resp, ok());

        let (store, resp) = run(store, "SET", &[b"user:1", b"one"]);
        assert_eq!(resp, ok());

        let (store, resp) = run(store, "KEYS", &[b"user:*"]);
        assert_eq!(resp, array(&["user:1", "user:2"]));

        let (store, resp) = run(store, "EXISTS", &[b"greeting", b"missing"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "DEL", &[b"greeting"]);
        assert_eq!(resp, integer(1));

        let (_, resp) = run(store, "GET", &[b"greeting"]);
        assert_eq!(resp, EngineResponse::BulkString(None));
    }

    #[test]
    fn hash_commands_are_backed_by_internal_map() {
        let store = Store::empty();

        let (store, resp) = run(
            store,
            "HSET",
            &[b"hash", b"b", b"2", b"a", b"1", b"c", b"3"],
        );
        assert_eq!(resp, integer(3));

        let (store, resp) = run(store, "HGET", &[b"hash", b"a"]);
        assert_eq!(resp, bulk("1"));

        let (store, resp) = run(store, "HLEN", &[b"hash"]);
        assert_eq!(resp, integer(3));

        let (store, resp) = run(store, "HEXISTS", &[b"hash", b"z"]);
        assert_eq!(resp, integer(0));

        let (store, resp) = run(store, "HKEYS", &[b"hash"]);
        assert_eq!(resp, array(&["a", "b", "c"]));

        let (store, resp) = run(store, "HVALS", &[b"hash"]);
        assert_eq!(resp, array(&["1", "2", "3"]));

        let (store, resp) = run(store, "HGETALL", &[b"hash"]);
        assert_eq!(
            resp,
            EngineResponse::Array(Some(vec![
                bulk("a"),
                bulk("1"),
                bulk("b"),
                bulk("2"),
                bulk("c"),
                bulk("3"),
            ]))
        );

        let (store, resp) = run(store, "HDEL", &[b"hash", b"b", b"c"]);
        assert_eq!(resp, integer(2));

        let (_, resp) = run(store, "HGETALL", &[b"hash"]);
        assert_eq!(resp, EngineResponse::Array(Some(vec![bulk("a"), bulk("1")])));
    }

    #[test]
    fn set_commands_are_backed_by_internal_set() {
        let store = Store::empty();

        let (store, resp) = run(store, "SADD", &[b"alpha", b"a", b"b", b"c"]);
        assert_eq!(resp, integer(3));

        let (store, resp) = run(store, "SADD", &[b"beta", b"b", b"c", b"d"]);
        assert_eq!(resp, integer(3));

        let (store, resp) = run(store, "SMEMBERS", &[b"alpha"]);
        assert_eq!(resp, array(&["a", "b", "c"]));

        let (store, resp) = run(store, "SISMEMBER", &[b"alpha", b"a"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "SREM", &[b"alpha", b"b", b"c"]);
        assert_eq!(resp, integer(2));

        let (store, resp) = run(store, "SMEMBERS", &[b"alpha"]);
        assert_eq!(resp, array(&["a"]));

        let (_, resp) = run(store.clone(), "SUNION", &[b"alpha", b"beta"]);
        assert_eq!(resp, array(&["a", "b", "c", "d"]));

        let (_, resp) = run(store.clone(), "SINTER", &[b"alpha", b"beta"]);
        assert_eq!(resp, EngineResponse::Array(Some(Vec::new())));

        let (_, resp) = run(store, "SDIFF", &[b"alpha", b"beta"]);
        assert_eq!(resp, array(&["a"]));
    }

    #[test]
    fn sorted_set_commands_cover_rank_and_range() {
        let store = Store::empty();

        let (store, resp) = run(
            store,
            "ZADD",
            &[b"scores", b"2", b"b", b"1", b"a", b"3", b"c"],
        );
        assert_eq!(resp, integer(3));

        let (store, resp) = run(store, "ZRANGE", &[b"scores", b"0", b"-1"]);
        assert_eq!(resp, array(&["a", "b", "c"]));

        let (store, resp) = run(store, "ZRANGEBYSCORE", &[b"scores", b"1", b"2"]);
        assert_eq!(resp, array(&["a", "b"]));

        let (store, resp) = run(store, "ZRANK", &[b"scores", b"b"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "ZSCORE", &[b"scores", b"c"]);
        assert_eq!(resp, bulk("3"));

        let (store, resp) = run(store, "ZREM", &[b"scores", b"b", b"c"]);
        assert_eq!(resp, integer(2));

        let (_, resp) = run(store, "ZCARD", &[b"scores"]);
        assert_eq!(resp, integer(1));
    }

    #[test]
    fn ttl_commands_manage_expiration_state() {
        let store = Store::empty();

        let (store, resp) = run(store, "SET", &[b"session", b"value"]);
        assert_eq!(resp, ok());

        let (store, resp) = run(store, "EXPIRE", &[b"session", b"10"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "TTL", &[b"session"]);
        match resp {
            EngineResponse::Integer(seconds) => assert!((0..=10).contains(&seconds)),
            other => panic!("unexpected TTL response: {other:?}"),
        }

        let (store, resp) = run(store, "PTTL", &[b"session"]);
        match resp {
            EngineResponse::Integer(millis) => assert!((1..=10_000).contains(&millis)),
            other => panic!("unexpected PTTL response: {other:?}"),
        }

        let (store, resp) = run(store, "PERSIST", &[b"session"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "TTL", &[b"session"]);
        assert_eq!(resp, integer(-1));

        let (store, resp) = run(store, "EXPIRE", &[b"session", b"0"]);
        assert_eq!(resp, integer(1));

        let (_, resp) = run(store, "GET", &[b"session"]);
        assert_eq!(resp, EngineResponse::BulkString(None));
    }

    #[test]
    fn hyperloglog_commands_approximate_cardinality_and_merge() {
        let store = Store::empty();

        let (store, resp) = run(store, "PFADD", &[b"visitors", b"a", b"b", b"c"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "PFADD", &[b"visitors", b"a", b"b"]);
        assert_eq!(resp, integer(0));

        let (store, resp) = run(store, "PFCOUNT", &[b"visitors"]);
        assert_eq!(resp, integer(3));

        let (store, resp) = run(store, "PFADD", &[b"other", b"c", b"d"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "PFMERGE", &[b"merged", b"visitors", b"other"]);
        assert_eq!(resp, ok());

        let (_, resp) = run(store, "PFCOUNT", &[b"merged"]);
        assert_eq!(resp, integer(4));
    }

    #[test]
    fn helper_paths_cover_dispatch_and_parser_edges() {
        let store = Store::empty();

        assert_error(dispatch(store.clone(), &[]).1, "ERR empty command");
        assert_error(
            dispatch(store.clone(), &[b"NOPE".to_vec()]).1,
            "ERR unknown command 'NOPE'",
        );

        let (_, resp) = run(store.clone(), "PING", &[]);
        assert_eq!(resp, EngineResponse::SimpleString("PONG".to_string()));

        let (_, resp) = run(store.clone(), "PING", &[b"hello"]);
        assert_eq!(resp, bulk("hello"));

        assert_error(
            run(store.clone(), "PING", &[b"hello", b"world"]).1,
            "ERR wrong number of arguments for 'PING'",
        );

        let (_, resp) = run(store.clone(), "ECHO", &[b"hi"]);
        assert_eq!(resp, bulk("hi"));

        assert_error(
            run(store.clone(), "ECHO", &[]).1,
            "ERR wrong number of arguments for 'ECHO'",
        );

        assert_eq!(ascii_upper(b"ping"), "PING");
        assert_eq!(
            bulk_string_bytes(&EngineResponse::BulkString(Some(b"abc".to_vec()))),
            Some(b"abc".to_vec())
        );
        assert_eq!(
            bulk_string_bytes(&EngineResponse::SimpleString("OK".to_string())),
            Some(b"OK".to_vec())
        );
        assert_eq!(
            bulk_string_bytes(&EngineResponse::Integer(7)),
            Some(b"7".to_vec())
        );
        assert_eq!(bulk_string_bytes(&EngineResponse::Array(None)), None);

        assert_eq!(parse_i64(b"42").unwrap(), 42);
        assert!(parse_i64(b"nope").is_err());
        assert_eq!(parse_isize(b"-2").unwrap(), -2);
        assert!(parse_isize(b"nope").is_err());
        assert_eq!(parse_usize(b"2").unwrap(), 2);
        assert!(parse_usize(b"-1").is_err());
        assert_eq!(parse_f64(b"2.5").unwrap(), 2.5);
        assert!(parse_f64(b"nope").is_err());

        let now_ms = current_time_ms();
        let expires_now = expiration_from_seconds(0);
        assert!(expires_now >= now_ms);
        let expires_later = expiration_from_seconds(1);
        assert!(expires_later >= now_ms + 1000);
        let expires_now_ms = expiration_from_millis(0);
        assert!(expires_now_ms >= now_ms);
        let expires_later_ms = expiration_from_millis(5);
        assert!(expires_later_ms >= now_ms + 5);
        assert!(unix_now_s() <= (current_time_ms() / 1000) as i64 + 1);

        assert!(!is_mutating(&[b"GET".to_vec()]));
        assert!(is_mutating(&[b"SET".to_vec()]));
    }

    #[test]
    fn string_commands_cover_numeric_and_metadata_edges() {
        let store = Store::empty();

        assert_error(
            run(store.clone(), "SET", &[b"k", b"v", b"BAD"]).1,
            "ERR syntax error",
        );
        assert_error(
            run(store.clone(), "SET", &[b"k", b"v", b"NX", b"XX"]).1,
            "ERR syntax error",
        );

        let (store, resp) = run(store, "SET", &[b"counter", b"9"]);
        assert_eq!(resp, ok());

        let (store, resp) = run(store, "INCR", &[b"counter"]);
        assert_eq!(resp, integer(10));

        let (store, resp) = run(store, "DECR", &[b"counter"]);
        assert_eq!(resp, integer(9));

        assert_error(
            run(store.clone(), "INCRBY", &[b"counter", b"foo"]).1,
            "ERR value is not an integer or out of range",
        );

        let mut overflow_store = Store::empty();
        overflow_store = overflow_store.set(
            b"huge",
            Entry::string(i64::MAX.to_string().into_bytes(), None),
        );
        assert_error(
            run(overflow_store, "INCRBY", &[b"huge", b"1"]).1,
            "ERR increment or decrement would overflow",
        );

        let (store, resp) = run(store, "APPEND", &[b"counter", b" world"]);
        assert_eq!(resp, integer(7));

        let (store, resp) = run(store, "RENAME", &[b"counter", b"renamed"]);
        assert_eq!(resp, ok());

        let (_, resp) = run(store.clone(), "TYPE", &[b"renamed"]);
        assert_eq!(resp, EngineResponse::SimpleString("string".to_string()));

        let (_, resp) = run(store.clone(), "TYPE", &[b"missing"]);
        assert_eq!(resp, EngineResponse::SimpleString("none".to_string()));

        let (hash_store, _) = run(Store::empty(), "HSET", &[b"hash", b"field", b"value"]);
        assert_error(
            run(hash_store, "APPEND", &[b"hash", b"x"]).1,
            "WRONGTYPE Operation against a key holding the wrong kind of value",
        );

        assert_error(
            run(Store::empty(), "RENAME", &[b"missing", b"dst"]).1,
            "ERR no such key",
        );
        assert_error(
            run(Store::empty(), "GET", &[]).1,
            "ERR wrong number of arguments for 'GET'",
        );
        assert_error(
            run(Store::empty(), "DEL", &[]).1,
            "ERR wrong number of arguments for 'DEL'",
        );
        assert_error(
            run(Store::empty(), "EXISTS", &[]).1,
            "ERR wrong number of arguments for 'EXISTS'",
        );
        assert_error(
            run(Store::empty(), "INCR", &[]).1,
            "ERR wrong number of arguments for 'INCR'",
        );
        assert_error(
            run(Store::empty(), "DECR", &[]).1,
            "ERR wrong number of arguments for 'DECR'",
        );
        assert_error(
            run(Store::empty(), "INCRBY", &[b"counter"]).1,
            "ERR wrong number of arguments for 'INCRBY'",
        );
        assert_error(
            run(Store::empty(), "DECRBY", &[b"counter"]).1,
            "ERR wrong number of arguments for 'DECRBY'",
        );
        assert_error(
            run(Store::empty(), "APPEND", &[b"k"]).1,
            "ERR wrong number of arguments for 'APPEND'",
        );
        assert_error(
            run(Store::empty(), "SET", &[b"k"]).1,
            "ERR wrong number of arguments for 'SET'",
        );
    }

    #[test]
    fn collection_commands_cover_wrong_types_and_bounds() {
        let store = Store::empty();

        assert_error(
            run(store.clone(), "HSET", &[b"hash", b"field"]).1,
            "ERR wrong number of arguments for 'HSET'",
        );

        let (store, resp) = run(store, "HSET", &[b"hash", b"b", b"2", b"a", b"1"]);
        assert_eq!(resp, integer(2));

        let (store, resp) = run(store, "HGET", &[b"hash", b"a"]);
        assert_eq!(resp, bulk("1"));

        let (_, resp) = run(store.clone(), "HGET", &[b"hash", b"missing"]);
        assert_eq!(resp, EngineResponse::BulkString(None));

        let (_, resp) = run(store.clone(), "HLEN", &[b"hash"]);
        assert_eq!(resp, integer(2));

        let (_, resp) = run(store.clone(), "HEXISTS", &[b"hash", b"a"]);
        assert_eq!(resp, integer(1));

        let (_, resp) = run(store.clone(), "HKEYS", &[b"hash"]);
        assert_eq!(resp, array(&["a", "b"]));

        let (_, resp) = run(store.clone(), "HVALS", &[b"hash"]);
        assert_eq!(resp, array(&["1", "2"]));

        let (_, resp) = run(store.clone(), "HGETALL", &[b"hash"]);
        assert_eq!(resp, EngineResponse::Array(Some(vec![bulk("a"), bulk("1"), bulk("b"), bulk("2")])));

        assert_error(
            run(store.clone(), "HDEL", &[b"hash"]).1,
            "ERR wrong number of arguments for 'HDEL'",
        );

        let (store, resp) = run(store, "HDEL", &[b"hash", b"a", b"b"]);
        assert_eq!(resp, integer(2));

        let (_, resp) = run(store, "HGETALL", &[b"hash"]);
        assert_eq!(resp, EngineResponse::Array(Some(Vec::new())));

        let (store, _) = run(Store::empty(), "SET", &[b"string", b"value"]);
        assert_error(
            run(store.clone(), "HSET", &[b"string", b"field", b"value"]).1,
            "WRONGTYPE Operation against a key holding the wrong kind of value",
        );
        assert_error(
            run(store.clone(), "HGET", &[b"string", b"field"]).1,
            "WRONGTYPE Operation against a key holding the wrong kind of value",
        );

        assert_error(
            run(Store::empty(), "LPUSH", &[b"list"]).1,
            "ERR wrong number of arguments for 'LPUSH'",
        );
        let (store, resp) = run(Store::empty(), "LPUSH", &[b"list", b"c", b"b", b"a"]);
        assert_eq!(resp, integer(3));

        let (store, resp) = run(store, "RPUSH", &[b"list", b"d"]);
        assert_eq!(resp, integer(4));

        let (_, resp) = run(store.clone(), "LLEN", &[b"list"]);
        assert_eq!(resp, integer(4));

        let (_, resp) = run(store.clone(), "LRANGE", &[b"list", b"0", b"-1"]);
        assert_eq!(resp, array(&["a", "b", "c", "d"]));

        let (_, resp) = run(store.clone(), "LINDEX", &[b"list", b"-1"]);
        assert_eq!(resp, bulk("d"));

        let (_, resp) = run(store.clone(), "LPOP", &[b"list"]);
        assert_eq!(resp, bulk("a"));

        let (_, resp) = run(store.clone(), "RPOP", &[b"list"]);
        assert_eq!(resp, bulk("d"));

        assert_error(
            run(store.clone(), "LRANGE", &[b"list", b"not", b"1"]).1,
            "ERR value is not an integer or out of range",
        );
        assert_error(
            run(store.clone(), "LINDEX", &[b"list", b"not"]).1,
            "ERR value is not an integer or out of range",
        );
        let (_, resp) = run(Store::empty(), "LPOP", &[b"missing"]);
        assert_eq!(resp, EngineResponse::BulkString(None));
        let (_, resp) = run(Store::empty(), "RPOP", &[b"missing"]);
        assert_eq!(resp, EngineResponse::BulkString(None));

        let (store, _) = run(Store::empty(), "SET", &[b"not-a-list", b"value"]);
        assert_error(
            run(store.clone(), "LPUSH", &[b"not-a-list", b"x"]).1,
            "WRONGTYPE Operation against a key holding the wrong kind of value",
        );

        assert_error(
            run(Store::empty(), "SADD", &[b"set"]).1,
            "ERR wrong number of arguments for 'SADD'",
        );
        let (store, resp) = run(Store::empty(), "SADD", &[b"set", b"a", b"b", b"a"]);
        assert_eq!(resp, integer(2));

        let (store, resp) = run(store.clone(), "SREM", &[b"set", b"a", b"missing"]);
        assert_eq!(resp, integer(1));

        let (_, resp) = run(store.clone(), "SISMEMBER", &[b"set", b"a"]);
        assert_eq!(resp, integer(0));

        let (_, resp) = run(store.clone(), "SMEMBERS", &[b"set"]);
        assert_eq!(resp, array(&["b"]));

        let (_, resp) = run(store.clone(), "SCARD", &[b"set"]);
        assert_eq!(resp, integer(1));

        assert_error(run(store.clone(), "SUNION", &[]).1, "ERR wrong number of arguments for 'SUNION'");
        assert_error(run(store.clone(), "SINTER", &[]).1, "ERR wrong number of arguments for 'SINTER'");
        assert_error(run(store.clone(), "SDIFF", &[]).1, "ERR wrong number of arguments for 'SDIFF'");

        let (_, resp) = run(Store::empty(), "SREM", &[b"missing", b"a"]);
        assert_eq!(resp, integer(0));

        let (store, _) = run(Store::empty(), "SET", &[b"not-a-set", b"value"]);
        assert_error(
            run(store, "SADD", &[b"not-a-set", b"x"]).1,
            "WRONGTYPE Operation against a key holding the wrong kind of value",
        );
    }

    #[test]
    fn sorted_set_and_hyperloglog_commands_cover_edges() {
        let store = Store::empty();

        assert_error(
            run(store.clone(), "ZADD", &[b"z"]).1,
            "ERR wrong number of arguments for 'ZADD'",
        );

        let (store, resp) = run(store, "ZADD", &[b"z", b"2", b"b", b"1", b"a", b"3", b"c"]);
        assert_eq!(resp, integer(3));

        let (store, resp) = run(store, "ZADD", &[b"z", b"4", b"b"]);
        assert_eq!(resp, integer(0));

        let (_, resp) = run(store.clone(), "ZRANGE", &[b"z", b"0", b"-1"]);
        assert_eq!(resp, array(&["a", "c", "b"]));

        let (_, resp) = run(store.clone(), "ZRANGEBYSCORE", &[b"z", b"1", b"3"]);
        assert_eq!(resp, array(&["a", "c"]));

        let (_, resp) = run(store.clone(), "ZRANK", &[b"z", b"b"]);
        assert_eq!(resp, integer(2));

        let (_, resp) = run(store.clone(), "ZSCORE", &[b"z", b"b"]);
        assert_eq!(resp, bulk("4"));

        let (_, resp) = run(store.clone(), "ZCARD", &[b"z"]);
        assert_eq!(resp, integer(3));

        let (_, resp) = run(store.clone(), "ZREM", &[b"z", b"b", b"missing"]);
        assert_eq!(resp, integer(1));

        let (_, resp) = run(store.clone(), "ZRANK", &[b"z", b"missing"]);
        assert_eq!(resp, EngineResponse::BulkString(None));

        let (_, resp) = run(store.clone(), "ZSCORE", &[b"z", b"missing"]);
        assert_eq!(resp, EngineResponse::BulkString(None));

        assert_error(
            run(store.clone(), "ZADD", &[b"z", b"bad", b"x"]).1,
            "ERR value is not a valid float",
        );
        assert_error(
            run(store.clone(), "ZRANGE", &[b"z", b"bad", b"1"]).1,
            "ERR value is not an integer or out of range",
        );
        assert_error(
            run(store.clone(), "ZRANGEBYSCORE", &[b"z", b"bad", b"1"]).1,
            "ERR value is not a valid float",
        );
        assert_error(
            run(Store::empty(), "ZRANGE", &[b"z"]).1,
            "ERR wrong number of arguments for 'ZRANGE'",
        );

        let (store, _) = run(Store::empty(), "SET", &[b"zstring", b"v"]);
        assert_error(
            run(store.clone(), "ZADD", &[b"zstring", b"1", b"x"]).1,
            "WRONGTYPE Operation against a key holding the wrong kind of value",
        );

        assert_error(
            run(store.clone(), "PFADD", &[b"hll"]).1,
            "ERR wrong number of arguments for 'PFADD'",
        );

        let (store, resp) = run(store, "PFADD", &[b"hll", b"a", b"b", b"a"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "PFADD", &[b"hll", b"a", b"b"]);
        assert_eq!(resp, integer(0));

        let (_, resp) = run(store.clone(), "PFCOUNT", &[b"hll"]);
        assert_eq!(resp, integer(2));

        let (_, resp) = run(store.clone(), "PFCOUNT", &[b"hll", b"missing"]);
        assert_eq!(resp, integer(2));

        assert_error(run(store.clone(), "PFCOUNT", &[]).1, "ERR wrong number of arguments for 'PFCOUNT'");
        assert_error(run(store.clone(), "PFMERGE", &[b"merged"]).1, "ERR wrong number of arguments for 'PFMERGE'");

        let (store, resp) = run(store.clone(), "PFADD", &[b"other", b"c"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "PFMERGE", &[b"merged", b"hll", b"other"]);
        assert_eq!(resp, ok());

        let (_, resp) = run(store.clone(), "PFCOUNT", &[b"merged"]);
        assert_eq!(resp, integer(3));

        let (store, _) = run(Store::empty(), "SET", &[b"not-hll", b"v"]);
        assert_error(
            run(store.clone(), "PFADD", &[b"not-hll", b"x"]).1,
            "WRONGTYPE Operation against a key holding the wrong kind of value",
        );
        assert_error(
            run(store.clone(), "PFCOUNT", &[b"hll", b"not-hll"]).1,
            "WRONGTYPE Operation against a key holding the wrong kind of value",
        );
        assert_error(
            run(store, "PFMERGE", &[b"merged", b"not-hll"]).1,
            "WRONGTYPE Operation against a key holding the wrong kind of value",
        );
    }

    #[test]
    fn ttl_and_database_commands_cover_selection_and_flush_paths() {
        let store = Store::empty();

        let (store, resp) = run(store, "SET", &[b"session", b"value"]);
        assert_eq!(resp, ok());

        let (store, resp) = run(store, "EXPIRE", &[b"session", b"1"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "TTL", &[b"session"]);
        match resp {
            EngineResponse::Integer(seconds) => assert!((0..=1).contains(&seconds)),
            other => panic!("unexpected TTL response: {other:?}"),
        }

        let (store, resp) = run(store, "PTTL", &[b"session"]);
        match resp {
            EngineResponse::Integer(millis) => assert!((0..=1_000).contains(&millis)),
            other => panic!("unexpected PTTL response: {other:?}"),
        }

        let (store, resp) = run(store, "PERSIST", &[b"session"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "TTL", &[b"session"]);
        assert_eq!(resp, integer(-1));

        let (_, resp) = run(store.clone(), "PERSIST", &[b"session"]);
        assert_eq!(resp, integer(0));

        let (_, resp) = run(store.clone(), "EXPIRE", &[b"missing", b"1"]);
        assert_eq!(resp, integer(0));

        let (_, resp) = run(store.clone(), "EXPIREAT", &[b"missing", b"1"]);
        assert_eq!(resp, integer(0));

        let past = (unix_now_s() - 1).to_string();
        let (store, resp) = run(store, "EXPIREAT", &[b"session", past.as_bytes()]);
        assert_eq!(resp, integer(1));

        let (_, resp) = run(store, "GET", &[b"session"]);
        assert_eq!(resp, EngineResponse::BulkString(None));

        let (store, _) = run(Store::empty(), "SET", &[b"user:1", b"one"]);
        let (store, _) = run(store, "SET", &[b"user:2", b"two"]);

        let (_, resp) = run(store.clone(), "DBSIZE", &[]);
        assert_eq!(resp, integer(2));

        let (_, resp) = run(store.clone(), "INFO", &[]);
        match resp {
            EngineResponse::BulkString(Some(bytes)) => {
                let text = String::from_utf8(bytes).unwrap();
                assert!(text.contains("mini_redis_version:0.1.0"));
                assert!(text.contains("active_db:0"));
                assert!(text.contains("dbsize:2"));
            }
            other => panic!("unexpected INFO response: {other:?}"),
        }

        let (_, resp) = run(store.clone(), "KEYS", &[b"user:?"]);
        assert_eq!(resp, array(&["user:1", "user:2"]));

        assert_error(
            run(store.clone(), "SELECT", &[b"bad"]).1,
            "ERR value is not an integer or out of range",
        );
        assert_error(
            run(store.clone(), "SELECT", &[b"999"]).1,
            "ERR DB index out of range",
        );

        let (store, resp) = run(store, "SELECT", &[b"1"]);
        assert_eq!(resp, ok());
        assert_eq!(store.active_db, 1);

        let (store, resp) = run(store, "SET", &[b"db1", b"one"]);
        assert_eq!(resp, ok());

        assert_error(
            run(store.clone(), "DBSIZE", &[b"extra"]).1,
            "ERR wrong number of arguments for 'DBSIZE'",
        );
        assert_error(
            run(store.clone(), "INFO", &[b"extra"]).1,
            "ERR wrong number of arguments for 'INFO'",
        );
        assert_error(
            run(store.clone(), "FLUSHDB", &[b"extra"]).1,
            "ERR wrong number of arguments for 'FLUSHDB'",
        );
        assert_error(
            run(store.clone(), "FLUSHALL", &[b"extra"]).1,
            "ERR wrong number of arguments for 'FLUSHALL'",
        );
        assert_error(
            run(store.clone(), "KEYS", &[b"a", b"b"]).1,
            "ERR wrong number of arguments for 'KEYS'",
        );

        let (store, resp) = run(store, "FLUSHDB", &[]);
        assert_eq!(resp, ok());
        let (_, resp) = run(store.clone(), "DBSIZE", &[]);
        assert_eq!(resp, integer(0));

        let (store, _) = run(store, "SELECT", &[b"0"]);
        let (store, _) = run(store, "SET", &[b"db0", b"zero"]);
        let (store, _) = run(store, "SELECT", &[b"1"]);
        let (store, _) = run(store, "SET", &[b"db1", b"one"]);
        let (store, resp) = run(store, "FLUSHALL", &[]);
        assert_eq!(resp, ok());
        let (_, resp) = run(store, "DBSIZE", &[]);
        assert_eq!(resp, integer(0));
    }
}
