package com.codingadventures.inmemorydatastoreengine;

import com.codingadventures.hashmap.HashMap;
import com.codingadventures.hashset.HashSet;
import com.codingadventures.heap.MinHeap;
import com.codingadventures.hyperloglog.HyperLogLog;
import com.codingadventures.inmemorydatastoreprotocol.CommandFrame;
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse;
import com.codingadventures.radixtree.RadixTree;
import com.codingadventures.skiplist.SkipList;

import java.math.BigDecimal;
import java.nio.charset.StandardCharsets;
import java.time.Instant;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.Locale;
import java.util.Map;

public final class DataStoreEngine {
    private final Store store = new Store(16);

    public synchronized EngineResponse executeFrame(CommandFrame frame) {
        if (frame == null) {
            return EngineResponse.error("ERR protocol error: expected array of bulk strings");
        }
        store.activeDatabase().activeExpire();
        return switch (frame.command()) {
            case "PING" -> ping(frame.args());
            case "ECHO" -> echo(frame.args());
            case "SET" -> set(frame.args());
            case "GET" -> get(frame.args());
            case "DEL" -> del(frame.args());
            case "EXISTS" -> exists(frame.args());
            case "KEYS" -> keys(frame.args());
            case "TYPE" -> type(frame.args());
            case "RENAME" -> rename(frame.args());
            case "APPEND" -> append(frame.args());
            case "INCR" -> incrBy(frame.args(), 1);
            case "DECR" -> incrBy(frame.args(), -1);
            case "INCRBY" -> incrBy(frame.args(), null);
            case "DECRBY" -> decrBy(frame.args());
            case "HSET" -> hset(frame.args());
            case "HGET" -> hget(frame.args());
            case "HDEL" -> hdel(frame.args());
            case "HGETALL" -> hgetall(frame.args());
            case "HLEN" -> hlen(frame.args());
            case "HEXISTS" -> hexists(frame.args());
            case "HKEYS" -> hkeys(frame.args());
            case "HVALS" -> hvals(frame.args());
            case "LPUSH" -> lpush(frame.args(), true);
            case "RPUSH" -> lpush(frame.args(), false);
            case "LPOP" -> lpop(frame.args(), true);
            case "RPOP" -> lpop(frame.args(), false);
            case "LLEN" -> llen(frame.args());
            case "LINDEX" -> lindex(frame.args());
            case "LRANGE" -> lrange(frame.args());
            case "SADD" -> sadd(frame.args());
            case "SREM" -> srem(frame.args());
            case "SISMEMBER" -> sismember(frame.args());
            case "SMEMBERS" -> smembers(frame.args());
            case "SCARD" -> scard(frame.args());
            case "SUNION" -> setOperation(frame.args(), "sunion", SetOperation.UNION);
            case "SINTER" -> setOperation(frame.args(), "sinter", SetOperation.INTERSECTION);
            case "SDIFF" -> setOperation(frame.args(), "sdiff", SetOperation.DIFFERENCE);
            case "ZADD" -> zadd(frame.args());
            case "ZRANGE" -> zrange(frame.args());
            case "ZRANGEBYSCORE" -> zrangeByScore(frame.args());
            case "ZRANK" -> zrank(frame.args());
            case "ZSCORE" -> zscore(frame.args());
            case "ZCARD" -> zcard(frame.args());
            case "ZREM" -> zrem(frame.args());
            case "PFADD" -> pfadd(frame.args());
            case "PFCOUNT" -> pfcount(frame.args());
            case "PFMERGE" -> pfmerge(frame.args());
            case "EXPIRE" -> expire(frame.args(), false);
            case "EXPIREAT" -> expire(frame.args(), true);
            case "TTL" -> ttl(frame.args());
            case "PTTL" -> pttl(frame.args());
            case "PERSIST" -> persist(frame.args());
            case "SELECT" -> select(frame.args());
            case "FLUSHDB" -> flushdb(frame.args());
            case "FLUSHALL" -> flushall(frame.args());
            case "DBSIZE" -> dbsize(frame.args());
            case "INFO" -> info(frame.args());
            default -> EngineResponse.error("ERR unknown command '" + frame.command().toLowerCase(Locale.ROOT) + "'");
        };
    }

    public static long currentTimeMs() {
        return Instant.now().toEpochMilli();
    }

    private EngineResponse ping(List<byte[]> args) {
        if (args.isEmpty()) {
            return EngineResponse.simpleString("PONG");
        }
        if (args.size() == 1) {
            return EngineResponse.bulkString(args.getFirst());
        }
        return wrongArity("ping");
    }

    private EngineResponse echo(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("echo");
        return EngineResponse.bulkString(args.getFirst());
    }

    private EngineResponse set(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("set");
        ByteSequence key = wrap(args.get(0));
        store.activeDatabase().set(key, Entry.string(args.get(1), null));
        return EngineResponse.ok();
    }

    private EngineResponse get(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("get");
        ByteSequence key = wrap(args.getFirst());
        store.activeDatabase().expireLazy(key);
        Entry entry = store.activeDatabase().get(key);
        if (entry == null) return EngineResponse.nullBulkString();
        if (entry.type != EntryType.STRING) return wrongType();
        return EngineResponse.bulkString((byte[]) entry.value);
    }

    private EngineResponse del(List<byte[]> args) {
        if (args.isEmpty()) return wrongArity("del");
        long removed = 0;
        for (byte[] arg : args) {
            if (store.activeDatabase().delete(wrap(arg))) removed++;
        }
        return EngineResponse.integer(removed);
    }

    private EngineResponse exists(List<byte[]> args) {
        if (args.isEmpty()) return wrongArity("exists");
        long count = 0;
        for (byte[] arg : args) {
            ByteSequence key = wrap(arg);
            store.activeDatabase().expireLazy(key);
            if (store.activeDatabase().get(key) != null) count++;
        }
        return EngineResponse.integer(count);
    }

    private EngineResponse keys(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("keys");
        List<EngineResponse> values = new ArrayList<>();
        for (ByteSequence key : store.activeDatabase().keys(args.getFirst())) {
            values.add(EngineResponse.bulkString(key.bytes()));
        }
        return EngineResponse.array(values);
    }

    private EngineResponse type(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("type");
        ByteSequence key = wrap(args.getFirst());
        store.activeDatabase().expireLazy(key);
        Entry entry = store.activeDatabase().get(key);
        return EngineResponse.simpleString(entry == null ? "none" : entry.type.wireName);
    }

    private EngineResponse rename(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("rename");
        ByteSequence source = wrap(args.get(0));
        store.activeDatabase().expireLazy(source);
        Entry entry = store.activeDatabase().get(source);
        if (entry == null) return EngineResponse.error("ERR no such key");
        ByteSequence destination = wrap(args.get(1));
        if (!source.equals(destination)) {
            store.activeDatabase().delete(source);
            store.activeDatabase().set(destination, entry);
        }
        return EngineResponse.ok();
    }

    private EngineResponse append(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("append");
        ByteSequence key = wrap(args.get(0));
        byte[] suffix = args.get(1);
        store.activeDatabase().expireLazy(key);
        Entry entry = store.activeDatabase().get(key);
        if (entry == null) {
            store.activeDatabase().set(key, Entry.string(suffix, null));
            return EngineResponse.integer(suffix.length);
        }
        if (entry.type != EntryType.STRING) return wrongType();
        byte[] prefix = (byte[]) entry.value;
        byte[] combined = Arrays.copyOf(prefix, prefix.length + suffix.length);
        System.arraycopy(suffix, 0, combined, prefix.length, suffix.length);
        entry.value = combined;
        return EngineResponse.integer(combined.length);
    }

    private EngineResponse incrBy(List<byte[]> args, Integer fixedDelta) {
        if ((fixedDelta == null && args.size() != 2) || (fixedDelta != null && args.size() != 1)) {
            return wrongArity(fixedDelta == null ? "incrby" : (fixedDelta > 0 ? "incr" : "decr"));
        }
        long delta = fixedDelta == null ? parseLong(args.get(1)) : fixedDelta;
        if (fixedDelta == null && args.size() == 2 && delta == Long.MIN_VALUE) return integerParseError();
        ByteSequence key = wrap(args.getFirst());
        store.activeDatabase().expireLazy(key);
        Entry entry = store.activeDatabase().get(key);
        long value = 0;
        if (entry != null) {
            if (entry.type != EntryType.STRING) return wrongType();
            value = parseLong((byte[]) entry.value);
            if (value == Long.MIN_VALUE) return integerParseError();
        }
        long next = value + delta;
        store.activeDatabase().set(key, Entry.string(Long.toString(next).getBytes(StandardCharsets.UTF_8), entry == null ? null : entry.expiresAtMs));
        return EngineResponse.integer(next);
    }

    private EngineResponse decrBy(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("decrby");
        long delta = parseLong(args.get(1));
        if (delta == Long.MIN_VALUE) return integerParseError();
        return incrBy(List.of(args.get(0), Long.toString(-delta).getBytes(StandardCharsets.UTF_8)), null);
    }

    private EngineResponse hset(List<byte[]> args) {
        if (args.size() < 3 || args.size() % 2 == 0) return wrongArity("hset");
        ByteSequence key = wrap(args.getFirst());
        store.activeDatabase().expireLazy(key);
        Entry entry = store.activeDatabase().get(key);
        HashMap<ByteSequence, byte[]> hash;
        Long expiresAt = null;
        if (entry == null) {
            hash = new HashMap<>();
            entry = Entry.hash(hash, null);
            store.activeDatabase().set(key, entry);
        } else {
            if (entry.type != EntryType.HASH) return wrongType();
            hash = castHash(entry.value);
            expiresAt = entry.expiresAtMs;
        }
        long added = 0;
        for (int i = 1; i < args.size(); i += 2) {
            ByteSequence field = wrap(args.get(i));
            if (!hash.has(field)) added++;
            hash.set(field, args.get(i + 1));
        }
        entry.expiresAtMs = expiresAt;
        return EngineResponse.integer(added);
    }

    private EngineResponse hget(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("hget");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.nullBulkString();
        if (entry.type != EntryType.HASH) return wrongType();
        return EngineResponse.bulkString(castHash(entry.value).get(wrap(args.get(1))));
    }

    private EngineResponse hdel(List<byte[]> args) {
        if (args.size() < 2) return wrongArity("hdel");
        ByteSequence key = wrap(args.getFirst());
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        if (entry.type != EntryType.HASH) return wrongType();
        HashMap<ByteSequence, byte[]> hash = castHash(entry.value);
        long removed = 0;
        for (int i = 1; i < args.size(); i++) {
            if (hash.delete(wrap(args.get(i)))) removed++;
        }
        if (hash.isEmpty()) store.activeDatabase().delete(key);
        return EngineResponse.integer(removed);
    }

    private EngineResponse hgetall(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("hgetall");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.array(List.of());
        if (entry.type != EntryType.HASH) return wrongType();
        List<EngineResponse> values = new ArrayList<>();
        for (var pair : castHash(entry.value).entries()) {
            values.add(EngineResponse.bulkString(pair.getKey().bytes()));
            values.add(EngineResponse.bulkString(pair.getValue()));
        }
        return EngineResponse.array(values);
    }

    private EngineResponse hlen(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("hlen");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        if (entry.type != EntryType.HASH) return wrongType();
        return EngineResponse.integer(castHash(entry.value).size());
    }

    private EngineResponse hexists(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("hexists");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        if (entry.type != EntryType.HASH) return wrongType();
        return EngineResponse.integer(castHash(entry.value).has(wrap(args.get(1))) ? 1 : 0);
    }

    private EngineResponse hkeys(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("hkeys");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.array(List.of());
        if (entry.type != EntryType.HASH) return wrongType();
        List<EngineResponse> values = new ArrayList<>();
        for (ByteSequence field : castHash(entry.value).keys()) {
            values.add(EngineResponse.bulkString(field.bytes()));
        }
        return EngineResponse.array(values);
    }

    private EngineResponse hvals(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("hvals");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.array(List.of());
        if (entry.type != EntryType.HASH) return wrongType();
        List<EngineResponse> values = new ArrayList<>();
        for (byte[] value : castHash(entry.value).values()) {
            values.add(EngineResponse.bulkString(value));
        }
        return EngineResponse.array(values);
    }

    private EngineResponse lpush(List<byte[]> args, boolean left) {
        if (args.size() < 2) return wrongArity(left ? "lpush" : "rpush");
        Entry entry = ensureList(args.getFirst());
        if (entry == null) return wrongType();
        @SuppressWarnings("unchecked")
        ArrayList<byte[]> list = (ArrayList<byte[]>) entry.value;
        for (int i = 1; i < args.size(); i++) {
            if (left) list.add(0, args.get(i)); else list.add(args.get(i));
        }
        return EngineResponse.integer(list.size());
    }

    private EngineResponse lpop(List<byte[]> args, boolean left) {
        if (args.size() != 1) return wrongArity(left ? "lpop" : "rpop");
        ByteSequence key = wrap(args.getFirst());
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.nullBulkString();
        if (entry.type != EntryType.LIST) return wrongType();
        @SuppressWarnings("unchecked")
        ArrayList<byte[]> list = (ArrayList<byte[]>) entry.value;
        byte[] value = list.remove(left ? 0 : list.size() - 1);
        if (list.isEmpty()) store.activeDatabase().delete(key);
        return EngineResponse.bulkString(value);
    }

    private EngineResponse llen(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("llen");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        if (entry.type != EntryType.LIST) return wrongType();
        @SuppressWarnings("unchecked")
        ArrayList<byte[]> list = (ArrayList<byte[]>) entry.value;
        return EngineResponse.integer(list.size());
    }

    private EngineResponse lindex(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("lindex");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.nullBulkString();
        if (entry.type != EntryType.LIST) return wrongType();
        @SuppressWarnings("unchecked")
        ArrayList<byte[]> list = (ArrayList<byte[]>) entry.value;
        Integer index = parseInt(args.get(1));
        if (index == null) return EngineResponse.error("ERR value is not an integer or out of range");
        int resolved = index < 0 ? list.size() + index : index;
        if (resolved < 0 || resolved >= list.size()) return EngineResponse.nullBulkString();
        return EngineResponse.bulkString(list.get(resolved));
    }

    private EngineResponse lrange(List<byte[]> args) {
        if (args.size() != 3) return wrongArity("lrange");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.array(List.of());
        if (entry.type != EntryType.LIST) return wrongType();
        Integer startValue = parseInt(args.get(1));
        Integer stopValue = parseInt(args.get(2));
        if (startValue == null || stopValue == null) return EngineResponse.error("ERR value is not an integer or out of range");
        @SuppressWarnings("unchecked")
        ArrayList<byte[]> list = (ArrayList<byte[]>) entry.value;
        int start = startValue < 0 ? list.size() + startValue : startValue;
        int stop = stopValue < 0 ? list.size() + stopValue : stopValue;
        start = Math.max(0, start);
        stop = Math.min(list.size() - 1, stop);
        if (list.isEmpty() || start > stop || start >= list.size()) return EngineResponse.array(List.of());
        List<EngineResponse> values = new ArrayList<>();
        for (int i = start; i <= stop; i++) values.add(EngineResponse.bulkString(list.get(i)));
        return EngineResponse.array(values);
    }

    private EngineResponse sadd(List<byte[]> args) {
        if (args.size() < 2) return wrongArity("sadd");
        Entry entry = ensureSet(args.getFirst());
        if (entry == null) return wrongType();
        @SuppressWarnings("unchecked")
        HashSet<ByteSequence> set = (HashSet<ByteSequence>) entry.value;
        long added = 0;
        for (int i = 1; i < args.size(); i++) {
            ByteSequence value = wrap(args.get(i));
            if (!set.contains(value)) {
                set.add(value);
                added++;
            }
        }
        return EngineResponse.integer(added);
    }

    private EngineResponse srem(List<byte[]> args) {
        if (args.size() < 2) return wrongArity("srem");
        ByteSequence key = wrap(args.getFirst());
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        if (entry.type != EntryType.SET) return wrongType();
        @SuppressWarnings("unchecked")
        HashSet<ByteSequence> set = (HashSet<ByteSequence>) entry.value;
        long removed = 0;
        for (int i = 1; i < args.size(); i++) {
            if (set.remove(wrap(args.get(i)))) removed++;
        }
        if (set.isEmpty()) store.activeDatabase().delete(key);
        return EngineResponse.integer(removed);
    }

    private EngineResponse sismember(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("sismember");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        if (entry.type != EntryType.SET) return wrongType();
        @SuppressWarnings("unchecked")
        HashSet<ByteSequence> set = (HashSet<ByteSequence>) entry.value;
        return EngineResponse.integer(set.contains(wrap(args.get(1))) ? 1 : 0);
    }

    private EngineResponse smembers(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("smembers");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.array(List.of());
        if (entry.type != EntryType.SET) return wrongType();
        @SuppressWarnings("unchecked")
        HashSet<ByteSequence> set = (HashSet<ByteSequence>) entry.value;
        List<ByteSequence> items = new ArrayList<>(set.items());
        items.sort(ByteSequence::compareTo);
        List<EngineResponse> values = new ArrayList<>();
        for (ByteSequence item : items) values.add(EngineResponse.bulkString(item.bytes()));
        return EngineResponse.array(values);
    }

    private EngineResponse scard(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("scard");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        if (entry.type != EntryType.SET) return wrongType();
        @SuppressWarnings("unchecked")
        HashSet<ByteSequence> set = (HashSet<ByteSequence>) entry.value;
        return EngineResponse.integer(set.size());
    }

    private EngineResponse setOperation(List<byte[]> args, String command, SetOperation operation) {
        if (args.isEmpty()) return wrongArity(command);
        HashSet<ByteSequence> result = new HashSet<>();
        boolean first = true;
        for (byte[] rawKey : args) {
            Entry entry = keyEntry(rawKey);
            HashSet<ByteSequence> next = entry == null ? new HashSet<>() : castSet(entry.value, entry.type);
            if (next == null) return wrongType();
            result = switch (operation) {
                case UNION -> first ? next.copy() : result.union(next);
                case INTERSECTION -> first ? next.copy() : result.intersection(next);
                case DIFFERENCE -> first ? next.copy() : result.difference(next);
            };
            first = false;
        }
        List<ByteSequence> values = new ArrayList<>(result.items());
        values.sort(ByteSequence::compareTo);
        ArrayList<EngineResponse> responses = new ArrayList<>();
        for (ByteSequence value : values) {
            responses.add(EngineResponse.bulkString(value.bytes()));
        }
        return EngineResponse.array(responses);
    }

    private EngineResponse zadd(List<byte[]> args) {
        if (args.size() < 3 || args.size() % 2 == 0) return wrongArity("zadd");
        ByteSequence key = wrap(args.getFirst());
        Entry entry = keyEntry(args.getFirst());
        SortedSet zset;
        Long expiresAt = null;
        if (entry == null) {
            zset = new SortedSet();
            store.activeDatabase().set(key, Entry.zset(zset, null));
        } else {
            if (entry.type != EntryType.ZSET) return wrongType();
            zset = (SortedSet) entry.value;
            expiresAt = entry.expiresAtMs;
        }
        long added = 0;
        for (int index = 1; index < args.size(); index += 2) {
            Double score = parseDouble(args.get(index));
            if (score == null) return floatParseError();
            if (zset.insert(score, wrap(args.get(index + 1)))) {
                added++;
            }
        }
        if (entry != null) entry.expiresAtMs = expiresAt;
        return EngineResponse.integer(added);
    }

    private EngineResponse zrange(List<byte[]> args) {
        if (args.size() < 3 || args.size() > 4) return wrongArity("zrange");
        Integer start = parseInt(args.get(1));
        Integer end = parseInt(args.get(2));
        if (start == null || end == null) return integerParseError();
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.array(List.of());
        if (entry.type != EntryType.ZSET) return wrongType();
        boolean withScores = args.size() == 4 && equalsIgnoreCase(args.get(3), "WITHSCORES");
        return EngineResponse.array(flattenZset(((SortedSet) entry.value).rangeByIndex(start, end), withScores));
    }

    private EngineResponse zrangeByScore(List<byte[]> args) {
        if (args.size() < 3 || args.size() > 4) return wrongArity("zrangebyscore");
        Double min = parseDouble(args.get(1));
        Double max = parseDouble(args.get(2));
        if (min == null || max == null) return floatParseError();
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.array(List.of());
        if (entry.type != EntryType.ZSET) return wrongType();
        boolean withScores = args.size() == 4 && equalsIgnoreCase(args.get(3), "WITHSCORES");
        return EngineResponse.array(flattenZset(((SortedSet) entry.value).rangeByScore(min, max), withScores));
    }

    private EngineResponse zrank(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("zrank");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.nullBulkString();
        if (entry.type != EntryType.ZSET) return wrongType();
        Integer rank = ((SortedSet) entry.value).rank(wrap(args.get(1)));
        return rank == null ? EngineResponse.nullBulkString() : EngineResponse.integer(rank);
    }

    private EngineResponse zscore(List<byte[]> args) {
        if (args.size() != 2) return wrongArity("zscore");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.nullBulkString();
        if (entry.type != EntryType.ZSET) return wrongType();
        Double score = ((SortedSet) entry.value).score(wrap(args.get(1)));
        return score == null ? EngineResponse.nullBulkString() : EngineResponse.bulkString(formatScore(score).getBytes(StandardCharsets.UTF_8));
    }

    private EngineResponse zcard(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("zcard");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        if (entry.type != EntryType.ZSET) return wrongType();
        return EngineResponse.integer(((SortedSet) entry.value).size());
    }

    private EngineResponse zrem(List<byte[]> args) {
        if (args.size() < 2) return wrongArity("zrem");
        ByteSequence key = wrap(args.getFirst());
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        if (entry.type != EntryType.ZSET) return wrongType();
        SortedSet zset = (SortedSet) entry.value;
        long removed = 0;
        for (int index = 1; index < args.size(); index++) {
            if (zset.remove(wrap(args.get(index)))) removed++;
        }
        if (zset.isEmpty()) store.activeDatabase().delete(key);
        return EngineResponse.integer(removed);
    }

    private EngineResponse pfadd(List<byte[]> args) {
        if (args.size() < 2) return wrongArity("pfadd");
        ByteSequence key = wrap(args.getFirst());
        Entry entry = keyEntry(args.getFirst());
        HyperLogLog sketch;
        Long expiresAt = null;
        if (entry == null) {
            sketch = new HyperLogLog();
            store.activeDatabase().set(key, Entry.hll(sketch, null));
        } else {
            if (entry.type != EntryType.HLL) return wrongType();
            sketch = (HyperLogLog) entry.value;
            expiresAt = entry.expiresAtMs;
        }
        HyperLogLog before = sketch.copy();
        for (int index = 1; index < args.size(); index++) {
            sketch.add(args.get(index));
        }
        if (entry != null) entry.expiresAtMs = expiresAt;
        return EngineResponse.integer(before.equals(sketch) ? 0 : 1);
    }

    private EngineResponse pfcount(List<byte[]> args) {
        if (args.isEmpty()) return wrongArity("pfcount");
        HyperLogLog aggregate = null;
        for (byte[] rawKey : args) {
            Entry entry = keyEntry(rawKey);
            if (entry == null) continue;
            if (entry.type != EntryType.HLL) return wrongType();
            aggregate = aggregate == null ? ((HyperLogLog) entry.value).copy() : aggregate.merge((HyperLogLog) entry.value);
        }
        return EngineResponse.integer((aggregate == null ? new HyperLogLog() : aggregate).count());
    }

    private EngineResponse pfmerge(List<byte[]> args) {
        if (args.size() < 2) return wrongArity("pfmerge");
        HyperLogLog merged = null;
        for (int index = 1; index < args.size(); index++) {
            Entry entry = keyEntry(args.get(index));
            if (entry == null) continue;
            if (entry.type != EntryType.HLL) return wrongType();
            merged = merged == null ? ((HyperLogLog) entry.value).copy() : merged.merge((HyperLogLog) entry.value);
        }
        Entry destination = keyEntry(args.getFirst());
        store.activeDatabase().set(wrap(args.getFirst()), Entry.hll(merged == null ? new HyperLogLog() : merged, destination == null ? null : destination.expiresAtMs));
        return EngineResponse.ok();
    }

    private EngineResponse expire(List<byte[]> args, boolean absoluteSeconds) {
        if (args.size() != 2) return wrongArity(absoluteSeconds ? "expireat" : "expire");
        ByteSequence key = wrap(args.getFirst());
        Entry entry = keyEntry(args.getFirst());
        if (entry == null) return EngineResponse.integer(0);
        long parsed = parseLong(args.get(1));
        if (parsed == Long.MIN_VALUE) return integerParseError();
        long expiresAt = absoluteSeconds ? parsed * 1000L : currentTimeMs() + (parsed * 1000L);
        entry.expiresAtMs = expiresAt;
        store.activeDatabase().ttlHeap.push(new ExpiryRecord(expiresAt, key));
        return EngineResponse.integer(1);
    }

    private EngineResponse ttl(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("ttl");
        ByteSequence key = wrap(args.getFirst());
        store.activeDatabase().expireLazy(key);
        Entry entry = store.activeDatabase().get(key);
        if (entry == null) return EngineResponse.integer(-2);
        if (entry.expiresAtMs == null) return EngineResponse.integer(-1);
        long remaining = entry.expiresAtMs - currentTimeMs();
        if (remaining < 0) {
            store.activeDatabase().delete(key);
            return EngineResponse.integer(-2);
        }
        return EngineResponse.integer(remaining / 1000L);
    }

    private EngineResponse pttl(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("pttl");
        ByteSequence key = wrap(args.getFirst());
        store.activeDatabase().expireLazy(key);
        Entry entry = store.activeDatabase().get(key);
        if (entry == null) return EngineResponse.integer(-2);
        if (entry.expiresAtMs == null) return EngineResponse.integer(-1);
        return EngineResponse.integer(Math.max(-1, entry.expiresAtMs - currentTimeMs()));
    }

    private EngineResponse persist(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("persist");
        Entry entry = keyEntry(args.getFirst());
        if (entry == null || entry.expiresAtMs == null) return EngineResponse.integer(0);
        entry.expiresAtMs = null;
        return EngineResponse.integer(1);
    }

    private EngineResponse select(List<byte[]> args) {
        if (args.size() != 1) return wrongArity("select");
        Integer index = parseInt(args.getFirst());
        if (index == null || index < 0 || index >= store.databaseCount()) {
            return EngineResponse.error("ERR DB index is out of range");
        }
        store.select(index);
        return EngineResponse.ok();
    }

    private EngineResponse flushdb(List<byte[]> args) {
        if (!args.isEmpty()) return wrongArity("flushdb");
        store.flushdb();
        return EngineResponse.ok();
    }

    private EngineResponse flushall(List<byte[]> args) {
        if (!args.isEmpty()) return wrongArity("flushall");
        store.flushall();
        return EngineResponse.ok();
    }

    private EngineResponse dbsize(List<byte[]> args) {
        if (!args.isEmpty()) return wrongArity("dbsize");
        return EngineResponse.integer(store.activeDatabase().dbsize());
    }

    private EngineResponse info(List<byte[]> args) {
        if (!args.isEmpty()) return wrongArity("info");
        String text = "# Server\r\nmini_redis_jvm:0.1.0\r\nactive_db:" + store.activeDb + "\r\ndbsize:" + store.activeDatabase().dbsize() + "\r\n";
        return EngineResponse.bulkString(text.getBytes(StandardCharsets.UTF_8));
    }

    private Entry keyEntry(byte[] rawKey) {
        ByteSequence key = wrap(rawKey);
        store.activeDatabase().expireLazy(key);
        return store.activeDatabase().get(key);
    }

    private Entry ensureList(byte[] rawKey) {
        ByteSequence key = wrap(rawKey);
        store.activeDatabase().expireLazy(key);
        Entry entry = store.activeDatabase().get(key);
        if (entry == null) {
            entry = Entry.list(new ArrayList<>(), null);
            store.activeDatabase().set(key, entry);
            return entry;
        }
        return entry.type == EntryType.LIST ? entry : null;
    }

    private Entry ensureSet(byte[] rawKey) {
        ByteSequence key = wrap(rawKey);
        store.activeDatabase().expireLazy(key);
        Entry entry = store.activeDatabase().get(key);
        if (entry == null) {
            entry = Entry.set(new HashSet<>(), null);
            store.activeDatabase().set(key, entry);
            return entry;
        }
        return entry.type == EntryType.SET ? entry : null;
    }

    @SuppressWarnings("unchecked")
    private HashMap<ByteSequence, byte[]> castHash(Object value) {
        return (HashMap<ByteSequence, byte[]>) value;
    }

    @SuppressWarnings("unchecked")
    private HashSet<ByteSequence> castSet(Object value, EntryType type) {
        return type == EntryType.SET ? (HashSet<ByteSequence>) value : null;
    }

    private static ByteSequence wrap(byte[] bytes) {
        return new ByteSequence(bytes);
    }

    private static EngineResponse wrongArity(String command) {
        return EngineResponse.error("ERR wrong number of arguments for '" + command + "' command");
    }

    private static EngineResponse wrongType() {
        return EngineResponse.error("WRONGTYPE Operation against a key holding the wrong kind of value");
    }

    private static EngineResponse integerParseError() {
        return EngineResponse.error("ERR value is not an integer or out of range");
    }

    private static EngineResponse floatParseError() {
        return EngineResponse.error("ERR value is not a valid float");
    }

    private static long parseLong(byte[] bytes) {
        try {
            return Long.parseLong(new String(bytes, StandardCharsets.UTF_8));
        } catch (NumberFormatException error) {
            return Long.MIN_VALUE;
        }
    }

    private static Integer parseInt(byte[] bytes) {
        try {
            return Integer.parseInt(new String(bytes, StandardCharsets.UTF_8));
        } catch (NumberFormatException error) {
            return null;
        }
    }

    private static byte[] literalPrefix(byte[] pattern) {
        int length = 0;
        while (length < pattern.length && pattern[length] != '*' && pattern[length] != '?') {
            length++;
        }
        return Arrays.copyOf(pattern, length);
    }

    private static String indexKey(ByteSequence key) {
        return indexKey(key.bytes());
    }

    private static String indexKey(byte[] bytes) {
        return new String(bytes, StandardCharsets.ISO_8859_1);
    }

    private static ByteSequence fromIndexKey(String key) {
        return new ByteSequence(key.getBytes(StandardCharsets.ISO_8859_1));
    }

    private enum SetOperation {
        UNION,
        INTERSECTION,
        DIFFERENCE
    }

    private static Double parseDouble(byte[] bytes) {
        try {
            double value = Double.parseDouble(new String(bytes, StandardCharsets.UTF_8));
            return Double.isFinite(value) ? value : null;
        } catch (NumberFormatException error) {
            return null;
        }
    }

    private static boolean equalsIgnoreCase(byte[] bytes, String text) {
        return new String(bytes, StandardCharsets.UTF_8).equalsIgnoreCase(text);
    }

    private static String formatScore(double score) {
        return BigDecimal.valueOf(score).stripTrailingZeros().toPlainString();
    }

    private static List<EngineResponse> flattenZset(List<Map.Entry<ByteSequence, Double>> values, boolean withScores) {
        ArrayList<EngineResponse> responses = new ArrayList<>();
        for (Map.Entry<ByteSequence, Double> entry : values) {
            responses.add(EngineResponse.bulkString(entry.getKey().bytes()));
            if (withScores) {
                responses.add(EngineResponse.bulkString(formatScore(entry.getValue()).getBytes(StandardCharsets.UTF_8)));
            }
        }
        return responses;
    }

    private enum EntryType {
        STRING("string"), HASH("hash"), LIST("list"), SET("set"), ZSET("zset"), HLL("hll");

        private final String wireName;

        EntryType(String wireName) {
            this.wireName = wireName;
        }
    }

    private static final class Entry {
        private final EntryType type;
        private Object value;
        private Long expiresAtMs;

        private Entry(EntryType type, Object value, Long expiresAtMs) {
            this.type = type;
            this.value = value;
            this.expiresAtMs = expiresAtMs;
        }

        private static Entry string(byte[] value, Long expiresAtMs) {
            return new Entry(EntryType.STRING, value, expiresAtMs);
        }

        private static Entry hash(HashMap<ByteSequence, byte[]> value, Long expiresAtMs) {
            return new Entry(EntryType.HASH, value, expiresAtMs);
        }

        private static Entry list(ArrayList<byte[]> value, Long expiresAtMs) {
            return new Entry(EntryType.LIST, value, expiresAtMs);
        }

        private static Entry set(HashSet<ByteSequence> value, Long expiresAtMs) {
            return new Entry(EntryType.SET, value, expiresAtMs);
        }

        private static Entry zset(SortedSet value, Long expiresAtMs) {
            return new Entry(EntryType.ZSET, value, expiresAtMs);
        }

        private static Entry hll(HyperLogLog value, Long expiresAtMs) {
            return new Entry(EntryType.HLL, value, expiresAtMs);
        }
    }

    private record SortedEntry(double score, ByteSequence member) {
    }

    private static final class SortedSet {
        private final HashMap<ByteSequence, Double> members = new HashMap<>();
        private final SkipList<SortedEntry, Byte> ordering = new SkipList<>((left, right) -> {
            int byScore = Double.compare(left.score(), right.score());
            return byScore != 0 ? byScore : left.member().compareTo(right.member());
        });

        private boolean insert(double score, ByteSequence member) {
            if (Double.isNaN(score)) {
                throw new IllegalArgumentException("sorted set score cannot be NaN");
            }
            boolean isNew = !members.has(member);
            if (!isNew) {
                ordering.delete(new SortedEntry(members.get(member), member));
            }
            members.set(member, score);
            ordering.insert(new SortedEntry(score, member), (byte) 0);
            return isNew;
        }

        private boolean remove(ByteSequence member) {
            if (!members.has(member)) {
                return false;
            }
            double score = members.get(member);
            ordering.delete(new SortedEntry(score, member));
            members.delete(member);
            return true;
        }

        private Integer rank(ByteSequence member) {
            if (!members.has(member)) {
                return null;
            }
            return ordering.rank(new SortedEntry(members.get(member), member));
        }

        private Double score(ByteSequence member) {
            return members.get(member);
        }

        private long size() {
            return members.size();
        }

        private boolean isEmpty() {
            return members.isEmpty();
        }

        private List<Map.Entry<ByteSequence, Double>> orderedEntries() {
            ArrayList<Map.Entry<ByteSequence, Double>> result = new ArrayList<>();
            for (Map.Entry<SortedEntry, Byte> entry : ordering.entries()) {
                result.add(Map.entry(entry.getKey().member(), entry.getKey().score()));
            }
            return result;
        }

        private List<Map.Entry<ByteSequence, Double>> rangeByIndex(int start, int end) {
            List<Map.Entry<ByteSequence, Double>> entries = orderedEntries();
            if (entries.isEmpty()) return List.of();
            int length = entries.size();
            int normalizedStart = start < 0 ? length + start : start;
            int normalizedEnd = end < 0 ? length + end : end;
            if (normalizedStart < 0 || normalizedEnd < 0 || normalizedStart >= length || normalizedStart > normalizedEnd) {
                return List.of();
            }
            return new ArrayList<>(entries.subList(normalizedStart, Math.min(length, normalizedEnd + 1)));
        }

        private List<Map.Entry<ByteSequence, Double>> rangeByScore(double min, double max) {
            if (Double.isNaN(min) || Double.isNaN(max)) {
                throw new IllegalArgumentException("sorted set score cannot be NaN");
            }
            ArrayList<Map.Entry<ByteSequence, Double>> result = new ArrayList<>();
            for (Map.Entry<ByteSequence, Double> entry : orderedEntries()) {
                if (entry.getValue() >= min && entry.getValue() <= max) {
                    result.add(entry);
                }
            }
            return result;
        }
    }

    private static final class Database {
        private final HashMap<ByteSequence, Entry> entries = new HashMap<>();
        private final MinHeap<ExpiryRecord> ttlHeap = new MinHeap<>();
        private RadixTree<ByteSequence> keyIndex = new RadixTree<>();

        private Entry get(ByteSequence key) {
            Entry entry = entries.get(key);
            if (entry == null) return null;
            if (entry.expiresAtMs != null && entry.expiresAtMs <= currentTimeMs()) return null;
            return entry;
        }

        private void set(ByteSequence key, Entry entry) {
            entries.set(key, entry);
            keyIndex.insert(indexKey(key), key);
            if (entry.expiresAtMs != null) ttlHeap.push(new ExpiryRecord(entry.expiresAtMs, key));
        }

        private boolean delete(ByteSequence key) {
            boolean deleted = entries.delete(key);
            if (deleted) {
                keyIndex.delete(indexKey(key));
            }
            return deleted;
        }

        private void expireLazy(ByteSequence key) {
            Entry entry = entries.get(key);
            if (entry != null && entry.expiresAtMs != null && entry.expiresAtMs <= currentTimeMs()) {
                delete(key);
            }
        }

        private void activeExpire() {
            long now = currentTimeMs();
            while (!ttlHeap.isEmpty()) {
                ExpiryRecord record = ttlHeap.peek();
                if (record == null || record.expiresAtMs > now) break;
                ttlHeap.pop();
                Entry entry = entries.get(record.key);
                if (entry != null && entry.expiresAtMs != null && entry.expiresAtMs.equals(record.expiresAtMs)) {
                    delete(record.key);
                }
            }
        }

        private List<ByteSequence> keys(byte[] pattern) {
            List<ByteSequence> keys = new ArrayList<>();
            byte[] prefix = literalPrefix(pattern);
            List<String> candidates = prefix.length == 0 ? keyIndex.keys() : keyIndex.wordsWithPrefix(indexKey(prefix));
            for (String candidate : candidates) {
                ByteSequence key = fromIndexKey(candidate);
                expireLazy(key);
                if (entries.get(key) != null && globMatch(pattern, key.bytes())) keys.add(key);
            }
            keys.sort(ByteSequence::compareTo);
            return keys;
        }

        private int dbsize() {
            activeExpire();
            return entries.size();
        }

        private void clear() {
            entries.clear();
            keyIndex = new RadixTree<>();
        }
    }

    private static final class Store {
        private final List<Database> databases;
        private int activeDb;

        private Store(int count) {
            databases = new ArrayList<>();
            for (int i = 0; i < count; i++) databases.add(new Database());
        }

        private Database activeDatabase() {
            return databases.get(activeDb);
        }

        private void select(int nextDb) {
            activeDb = nextDb;
        }

        private int databaseCount() {
            return databases.size();
        }

        private void flushdb() {
            activeDatabase().clear();
        }

        private void flushall() {
            for (Database database : databases) database.clear();
        }
    }

    private record ExpiryRecord(long expiresAtMs, ByteSequence key) implements Comparable<ExpiryRecord> {
        @Override
        public int compareTo(ExpiryRecord other) {
            int byTime = Long.compare(expiresAtMs, other.expiresAtMs);
            return byTime != 0 ? byTime : key.compareTo(other.key);
        }
    }

    private record ByteSequence(byte[] bytes) implements Comparable<ByteSequence> {
        @Override
        public boolean equals(Object other) {
            return other instanceof ByteSequence that && Arrays.equals(bytes, that.bytes);
        }

        @Override
        public int hashCode() {
            return Arrays.hashCode(bytes);
        }

        @Override
        public int compareTo(ByteSequence other) {
            int limit = Math.min(bytes.length, other.bytes.length);
            for (int i = 0; i < limit; i++) {
                int diff = Byte.compareUnsigned(bytes[i], other.bytes[i]);
                if (diff != 0) return diff;
            }
            return Integer.compare(bytes.length, other.bytes.length);
        }
    }

    private static boolean globMatch(byte[] pattern, byte[] text) {
        return globMatch(pattern, 0, text, 0);
    }

    private static boolean globMatch(byte[] pattern, int p, byte[] text, int t) {
        if (p == pattern.length) return t == text.length;
        if (pattern[p] == '*') {
            for (int next = t; next <= text.length; next++) {
                if (globMatch(pattern, p + 1, text, next)) return true;
            }
            return false;
        }
        if (t == text.length) return false;
        if (pattern[p] == '?' || pattern[p] == text[t]) return globMatch(pattern, p + 1, text, t + 1);
        return false;
    }
}
