use std::collections::VecDeque;

use hash_map::HashMap as DtHashMap;
use hash_set::HashSet as DtHashSet;
use hyperloglog::HyperLogLog;
use resp_protocol::{RespError, RespValue};

use crate::store::{current_time_ms, Store};
use crate::types::{Entry, EntryValue, SortedSet};

pub fn dispatch(store: Store, parts: &[Vec<u8>]) -> (Store, RespValue) {
    if parts.is_empty() {
        return (store, err("ERR empty command"));
    }

    let command = ascii_upper(&parts[0]);
    let store = match command.as_str() {
        "SELECT" | "INFO" | "PING" | "ECHO" | "DBSIZE" | "FLUSHDB" | "FLUSHALL" => store,
        _ => store.expire_lazy(parts.get(1).map(|bytes| bytes.as_slice())),
    };

    match command.as_str() {
        "PING" => cmd_ping(store, &parts[1..]),
        "ECHO" => cmd_echo(store, &parts[1..]),
        "SET" => cmd_set(store, &parts[1..]),
        "GET" => cmd_get(store, &parts[1..]),
        "DEL" => cmd_del(store, &parts[1..]),
        "EXISTS" => cmd_exists(store, &parts[1..]),
        "TYPE" => cmd_type(store, &parts[1..]),
        "RENAME" => cmd_rename(store, &parts[1..]),
        "INCR" => cmd_incr(store, &parts[1..]),
        "DECR" => cmd_decr(store, &parts[1..]),
        "INCRBY" => cmd_incrby(store, &parts[1..]),
        "DECRBY" => cmd_decrby(store, &parts[1..]),
        "APPEND" => cmd_append(store, &parts[1..]),
        "HSET" => cmd_hset(store, &parts[1..]),
        "HGET" => cmd_hget(store, &parts[1..]),
        "HDEL" => cmd_hdel(store, &parts[1..]),
        "HGETALL" => cmd_hgetall(store, &parts[1..]),
        "HLEN" => cmd_hlen(store, &parts[1..]),
        "HEXISTS" => cmd_hexists(store, &parts[1..]),
        "HKEYS" => cmd_hkeys(store, &parts[1..]),
        "HVALS" => cmd_hvals(store, &parts[1..]),
        "LPUSH" => cmd_lpush(store, &parts[1..]),
        "RPUSH" => cmd_rpush(store, &parts[1..]),
        "LPOP" => cmd_lpop(store, &parts[1..]),
        "RPOP" => cmd_rpop(store, &parts[1..]),
        "LLEN" => cmd_llen(store, &parts[1..]),
        "LRANGE" => cmd_lrange(store, &parts[1..]),
        "LINDEX" => cmd_lindex(store, &parts[1..]),
        "SADD" => cmd_sadd(store, &parts[1..]),
        "SREM" => cmd_srem(store, &parts[1..]),
        "SISMEMBER" => cmd_sismember(store, &parts[1..]),
        "SMEMBERS" => cmd_smembers(store, &parts[1..]),
        "SCARD" => cmd_scard(store, &parts[1..]),
        "SUNION" => cmd_sunion(store, &parts[1..]),
        "SINTER" => cmd_sinter(store, &parts[1..]),
        "SDIFF" => cmd_sdiff(store, &parts[1..]),
        "ZADD" => cmd_zadd(store, &parts[1..]),
        "ZRANGE" => cmd_zrange(store, &parts[1..]),
        "ZRANGEBYSCORE" => cmd_zrangebyscore(store, &parts[1..]),
        "ZRANK" => cmd_zrank(store, &parts[1..]),
        "ZSCORE" => cmd_zscore(store, &parts[1..]),
        "ZCARD" => cmd_zcard(store, &parts[1..]),
        "ZREM" => cmd_zrem(store, &parts[1..]),
        "PFADD" => cmd_pfadd(store, &parts[1..]),
        "PFCOUNT" => cmd_pfcount(store, &parts[1..]),
        "PFMERGE" => cmd_pfmerge(store, &parts[1..]),
        "EXPIRE" => cmd_expire(store, &parts[1..]),
        "EXPIREAT" => cmd_expireat(store, &parts[1..]),
        "TTL" => cmd_ttl(store, &parts[1..]),
        "PTTL" => cmd_pttl(store, &parts[1..]),
        "PERSIST" => cmd_persist(store, &parts[1..]),
        "SELECT" => cmd_select(store, &parts[1..]),
        "FLUSHDB" => cmd_flushdb(store, &parts[1..]),
        "FLUSHALL" => cmd_flushall(store, &parts[1..]),
        "DBSIZE" => cmd_dbsize(store, &parts[1..]),
        "INFO" => cmd_info(store, &parts[1..]),
        "KEYS" => cmd_keys(store, &parts[1..]),
        _ => (store, err(format!("ERR unknown command '{}'", command))),
    }
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

pub fn cmd_ping(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [] => (store, RespValue::SimpleString("PONG".to_string())),
        [message] => (store, RespValue::BulkString(Some(message.clone()))),
        _ => (store, err("ERR wrong number of arguments for 'PING'")),
    }
}

pub fn cmd_echo(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [message] => (store, RespValue::BulkString(Some(message.clone()))),
        _ => (store, err("ERR wrong number of arguments for 'ECHO'")),
    }
}

pub fn cmd_set(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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
        return (store, RespValue::BulkString(None));
    }
    if xx && !exists {
        return (store, RespValue::BulkString(None));
    }

    let entry = Entry::string(value, expires_at);
    (store.set(key, entry), ok())
}

pub fn cmd_get(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::String(bytes) => (store, RespValue::BulkString(Some(bytes.clone()))),
                _ => (store, wrong_type()),
            },
            None => (store, RespValue::BulkString(None)),
        },
        _ => (store, err("ERR wrong number of arguments for 'GET'")),
    }
}

pub fn cmd_del(mut store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_exists(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    if args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'EXISTS'"));
    }
    let count = args.iter().filter(|key| store.get(key).is_some()).count() as i64;
    (store, integer(count))
}

pub fn cmd_type(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => match store.type_of(key) {
            Some(entry_type) => (store, RespValue::SimpleString(entry_type.to_string())),
            None => (store, RespValue::SimpleString("none".to_string())),
        },
        _ => (store, err("ERR wrong number of arguments for 'TYPE'")),
    }
}

pub fn cmd_rename(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_incr(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => cmd_incrby(store, &[key.clone(), b"1".to_vec()]),
        _ => (store, err("ERR wrong number of arguments for 'INCR'")),
    }
}

pub fn cmd_decr(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => cmd_incrby(store, &[key.clone(), b"-1".to_vec()]),
        _ => (store, err("ERR wrong number of arguments for 'DECR'")),
    }
}

pub fn cmd_incrby(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key, delta_bytes] => match parse_i64(delta_bytes) {
            Ok(delta) => adjust_integer(store, key.clone(), delta),
            Err(err) => (store, err),
        },
        _ => (store, err("ERR wrong number of arguments for 'INCRBY'")),
    }
}

pub fn cmd_decrby(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key, delta_bytes] => match parse_i64(delta_bytes) {
            Ok(delta) => adjust_integer(store, key.clone(), -delta),
            Err(err) => (store, err),
        },
        _ => (store, err("ERR wrong number of arguments for 'DECRBY'")),
    }
}

pub fn cmd_append(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_hset(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_hget(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key, field] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => match map.get(field) {
                    Some(value) => (store, RespValue::BulkString(Some(value.clone()))),
                    None => (store, RespValue::BulkString(None)),
                },
                _ => (store, wrong_type()),
            },
            None => (store, RespValue::BulkString(None)),
        },
        _ => (store, err("ERR wrong number of arguments for 'HGET'")),
    }
}

pub fn cmd_hdel(mut store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_hgetall(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => match store.get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => {
                    let mut entries = map.entries();
                    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                    let mut out = Vec::with_capacity(entries.len() * 2);
                    for (field, value) in entries {
                        out.push(RespValue::BulkString(Some(field.clone())));
                        out.push(RespValue::BulkString(Some(value.clone())));
                    }
                    (store, RespValue::Array(Some(out)))
                }
                _ => (store, wrong_type()),
            },
            None => (store, RespValue::Array(Some(Vec::new()))),
        },
        _ => (store, err("ERR wrong number of arguments for 'HGETALL'")),
    }
}

pub fn cmd_hlen(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_hexists(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_hkeys(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => (
                    store,
                    RespValue::Array(Some(
                        {
                            let mut keys = map.keys();
                            keys.sort();
                            keys
                        }
                        .into_iter()
                        .map(|field| RespValue::BulkString(Some(field)))
                        .collect(),
                    )),
                ),
                _ => (store, wrong_type()),
            },
            None => (store, RespValue::Array(Some(Vec::new()))),
        },
        _ => (store, err("ERR wrong number of arguments for 'HKEYS'")),
    }
}

pub fn cmd_hvals(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Hash(map) => (
                    store,
                    RespValue::Array(Some(
                        {
                            let mut entries = map.entries();
                            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                            entries
                        }
                        .into_iter()
                        .map(|(_, value)| RespValue::BulkString(Some(value)))
                        .collect(),
                    )),
                ),
                _ => (store, wrong_type()),
            },
            None => (store, RespValue::Array(Some(Vec::new()))),
        },
        _ => (store, err("ERR wrong number of arguments for 'HVALS'")),
    }
}

pub fn cmd_lpush(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_rpush(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_lpop(mut store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => {
            let expires_at = store.get(key).and_then(|entry| entry.expires_at);
            let mut list = match store.get(key) {
                Some(entry) => match &entry.value {
                    EntryValue::List(list) => list.clone(),
                    _ => return (store, wrong_type()),
                },
                None => return (store, RespValue::BulkString(None)),
            };
            let value = list.pop_front();
            if list.is_empty() {
                store = store.delete(key);
            } else {
                store = store.set(key.clone(), Entry::list(list, expires_at));
            }
            (
                store,
                value.map_or(RespValue::BulkString(None), |v| {
                    RespValue::BulkString(Some(v))
                }),
            )
        }
        _ => (store, err("ERR wrong number of arguments for 'LPOP'")),
    }
}

pub fn cmd_rpop(mut store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => {
            let expires_at = store.get(key).and_then(|entry| entry.expires_at);
            let mut list = match store.get(key) {
                Some(entry) => match &entry.value {
                    EntryValue::List(list) => list.clone(),
                    _ => return (store, wrong_type()),
                },
                None => return (store, RespValue::BulkString(None)),
            };
            let value = list.pop_back();
            if list.is_empty() {
                store = store.delete(key);
            } else {
                store = store.set(key.clone(), Entry::list(list, expires_at));
            }
            (
                store,
                value.map_or(RespValue::BulkString(None), |v| {
                    RespValue::BulkString(Some(v))
                }),
            )
        }
        _ => (store, err("ERR wrong number of arguments for 'RPOP'")),
    }
}

pub fn cmd_llen(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_lrange(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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
                            return (store, RespValue::Array(Some(Vec::new())));
                        }
                        let slice = list
                            .iter()
                            .skip(start as usize)
                            .take((end - start + 1) as usize)
                            .cloned()
                            .map(|v| RespValue::BulkString(Some(v)))
                            .collect();
                        (store, RespValue::Array(Some(slice)))
                    }
                    _ => (store, wrong_type()),
                },
                None => (store, RespValue::Array(Some(Vec::new()))),
            }
        }
        _ => (store, err("ERR wrong number of arguments for 'LRANGE'")),
    }
}

pub fn cmd_lindex(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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
                            return (store, RespValue::BulkString(None));
                        }
                        let value = list.get(index as usize).cloned();
                        (
                            store,
                            value.map_or(RespValue::BulkString(None), |v| {
                                RespValue::BulkString(Some(v))
                            }),
                        )
                    }
                    _ => (store, wrong_type()),
                },
                None => (store, RespValue::BulkString(None)),
            }
        }
        _ => (store, err("ERR wrong number of arguments for 'LINDEX'")),
    }
}

pub fn cmd_sadd(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_srem(mut store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_sismember(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_smembers(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => match store.clone().get(key) {
            Some(entry) => match &entry.value {
                EntryValue::Set(set) => (
                    store,
                    RespValue::Array(Some(
                        {
                            let mut members = set.to_list();
                            members.sort();
                            members
                        }
                        .into_iter()
                        .map(|member| RespValue::BulkString(Some(member)))
                        .collect(),
                    )),
                ),
                _ => (store, wrong_type()),
            },
            None => (store, RespValue::Array(Some(Vec::new()))),
        },
        _ => (store, err("ERR wrong number of arguments for 'SMEMBERS'")),
    }
}

pub fn cmd_scard(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_sunion(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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
        RespValue::Array(Some(
            members
                .into_iter()
                .map(|member| RespValue::BulkString(Some(member)))
                .collect(),
        )),
    )
}

pub fn cmd_sinter(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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
        RespValue::Array(Some(
            members
                .into_iter()
                .map(|member| RespValue::BulkString(Some(member)))
                .collect(),
        )),
    )
}

pub fn cmd_sdiff(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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
        RespValue::Array(Some(
            members
                .into_iter()
                .map(|member| RespValue::BulkString(Some(member)))
                .collect(),
        )),
    )
}

pub fn cmd_zadd(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_zrange(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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
                        RespValue::Array(Some(
                            zset.range_by_index(start, end)
                                .into_iter()
                                .map(|(member, _)| RespValue::BulkString(Some(member)))
                                .collect(),
                        )),
                    ),
                    _ => (store, wrong_type()),
                },
                None => (store, RespValue::Array(Some(Vec::new()))),
            }
        }
        _ => (store, err("ERR wrong number of arguments for 'ZRANGE'")),
    }
}

pub fn cmd_zrangebyscore(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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
                        RespValue::Array(Some(
                            zset.range_by_score(min, max)
                                .into_iter()
                                .map(|(member, _)| RespValue::BulkString(Some(member)))
                                .collect(),
                        )),
                    ),
                    _ => (store, wrong_type()),
                },
                None => (store, RespValue::Array(Some(Vec::new()))),
            }
        }
        _ => (
            store,
            err("ERR wrong number of arguments for 'ZRANGEBYSCORE'"),
        ),
    }
}

pub fn cmd_zrank(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key, member] => match store.get(key) {
            Some(entry) => match &entry.value {
                EntryValue::ZSet(zset) => match zset.rank(member) {
                    Some(rank) => (store, integer(rank as i64)),
                    None => (store, RespValue::BulkString(None)),
                },
                _ => (store, wrong_type()),
            },
            None => (store, RespValue::BulkString(None)),
        },
        _ => (store, err("ERR wrong number of arguments for 'ZRANK'")),
    }
}

pub fn cmd_zscore(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key, member] => match store.get(key) {
            Some(entry) => match &entry.value {
                EntryValue::ZSet(zset) => match zset.score(member) {
                    Some(score) => (
                        store,
                        RespValue::BulkString(Some(score.to_string().into_bytes())),
                    ),
                    None => (store, RespValue::BulkString(None)),
                },
                _ => (store, wrong_type()),
            },
            None => (store, RespValue::BulkString(None)),
        },
        _ => (store, err("ERR wrong number of arguments for 'ZSCORE'")),
    }
}

pub fn cmd_zcard(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_zrem(mut store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_pfadd(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_pfcount(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_pfmerge(mut store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_expire(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key, seconds] => match parse_i64(seconds) {
            Ok(seconds) => set_expiration(store, key.clone(), expiration_from_seconds(seconds)),
            Err(err) => (store, err),
        },
        _ => (store, err("ERR wrong number of arguments for 'EXPIRE'")),
    }
}

pub fn cmd_expireat(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_ttl(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => ttl_like(store, key, false),
        _ => (store, err("ERR wrong number of arguments for 'TTL'")),
    }
}

pub fn cmd_pttl(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [key] => ttl_like(store, key, true),
        _ => (store, err("ERR wrong number of arguments for 'PTTL'")),
    }
}

pub fn cmd_persist(mut store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_select(mut store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
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

pub fn cmd_flushdb(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    if !args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'FLUSHDB'"));
    }
    (store.flushdb(), ok())
}

pub fn cmd_flushall(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    if !args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'FLUSHALL'"));
    }
    (store.flushall(), ok())
}

pub fn cmd_dbsize(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    if !args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'DBSIZE'"));
    }
    let size = store.dbsize();
    (store, integer(size as i64))
}

pub fn cmd_info(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    if !args.is_empty() {
        return (store, err("ERR wrong number of arguments for 'INFO'"));
    }
    let info = format!(
        "# Server\r\nmini_redis_version:0.1.0\r\nactive_db:{}\r\ndbsize:{}\r\n",
        store.active_db,
        store.dbsize()
    );
    (store, RespValue::BulkString(Some(info.into_bytes())))
}

pub fn cmd_keys(store: Store, args: &[Vec<u8>]) -> (Store, RespValue) {
    match args {
        [pattern] => (
            store.clone(),
            RespValue::Array(Some(
                store
                    .keys(pattern)
                    .into_iter()
                    .map(|key| RespValue::BulkString(Some(key)))
                    .collect(),
            )),
        ),
        _ => (store, err("ERR wrong number of arguments for 'KEYS'")),
    }
}

fn adjust_integer(store: Store, key: Vec<u8>, delta: i64) -> (Store, RespValue) {
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

fn ttl_like(store: Store, key: &[u8], milliseconds: bool) -> (Store, RespValue) {
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

fn set_expiration(mut store: Store, key: Vec<u8>, expires_at: u64) -> (Store, RespValue) {
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

fn ok() -> RespValue {
    RespValue::SimpleString("OK".to_string())
}

fn integer(value: i64) -> RespValue {
    RespValue::Integer(value)
}

fn err(message: impl Into<String>) -> RespValue {
    RespValue::Error(RespError::new(message))
}

fn wrong_type() -> RespValue {
    err("WRONGTYPE Operation against a key holding the wrong kind of value")
}

fn parse_i64(bytes: &[u8]) -> Result<i64, RespValue> {
    std::str::from_utf8(bytes)
        .map_err(|_| err("ERR value is not an integer or out of range"))
        .and_then(|text| {
            text.parse::<i64>()
                .map_err(|_| err("ERR value is not an integer or out of range"))
        })
}

fn parse_isize(bytes: &[u8]) -> Result<isize, RespValue> {
    std::str::from_utf8(bytes)
        .map_err(|_| err("ERR value is not an integer or out of range"))
        .and_then(|text| {
            text.parse::<isize>()
                .map_err(|_| err("ERR value is not an integer or out of range"))
        })
}

fn parse_usize(bytes: &[u8]) -> Result<usize, RespValue> {
    std::str::from_utf8(bytes)
        .map_err(|_| err("ERR value is not an integer or out of range"))
        .and_then(|text| {
            text.parse::<usize>()
                .map_err(|_| err("ERR value is not an integer or out of range"))
        })
}

fn parse_f64(bytes: &[u8]) -> Result<f64, RespValue> {
    std::str::from_utf8(bytes)
        .map_err(|_| err("ERR value is not a valid float"))
        .and_then(|text| {
            text.parse::<f64>()
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
pub fn bulk_string_bytes(value: &RespValue) -> Option<Vec<u8>> {
    match value {
        RespValue::BulkString(Some(bytes)) => Some(bytes.clone()),
        RespValue::SimpleString(text) => Some(text.as_bytes().to_vec()),
        RespValue::Integer(n) => Some(n.to_string().into_bytes()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(store: Store, command: &str, args: &[&[u8]]) -> (Store, RespValue) {
        let mut parts = vec![command.as_bytes().to_vec()];
        parts.extend(args.iter().map(|arg| arg.to_vec()));
        dispatch(store, &parts)
    }

    fn bulk(value: &str) -> RespValue {
        RespValue::BulkString(Some(value.as_bytes().to_vec()))
    }

    fn array(values: &[&str]) -> RespValue {
        RespValue::Array(Some(values.iter().map(|value| bulk(value)).collect()))
    }

    #[test]
    fn string_commands_cover_basic_lifecycle() {
        let store = Store::empty();

        let (store, resp) = run(store, "SET", &[b"greeting", b"hello"]);
        assert_eq!(resp, ok());

        let (store, resp) = run(store, "GET", &[b"greeting"]);
        assert_eq!(resp, bulk("hello"));

        let (store, resp) = run(store, "SET", &[b"greeting", b"world", b"NX"]);
        assert_eq!(resp, RespValue::BulkString(None));

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
        assert_eq!(resp, RespValue::BulkString(None));
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
            RespValue::Array(Some(vec![
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
        assert_eq!(resp, RespValue::Array(Some(vec![bulk("a"), bulk("1")])));
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
        assert_eq!(resp, RespValue::Array(Some(Vec::new())));

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
            RespValue::Integer(seconds) => assert!((0..=10).contains(&seconds)),
            other => panic!("unexpected TTL response: {other:?}"),
        }

        let (store, resp) = run(store, "PTTL", &[b"session"]);
        match resp {
            RespValue::Integer(millis) => assert!((1..=10_000).contains(&millis)),
            other => panic!("unexpected PTTL response: {other:?}"),
        }

        let (store, resp) = run(store, "PERSIST", &[b"session"]);
        assert_eq!(resp, integer(1));

        let (store, resp) = run(store, "TTL", &[b"session"]);
        assert_eq!(resp, integer(-1));

        let (store, resp) = run(store, "EXPIRE", &[b"session", b"0"]);
        assert_eq!(resp, integer(1));

        let (_, resp) = run(store, "GET", &[b"session"]);
        assert_eq!(resp, RespValue::BulkString(None));
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
}
