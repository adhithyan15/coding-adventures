package com.codingadventures.inmemorydatastoreengine

import com.codingadventures.hashmap.HashMap
import com.codingadventures.hashset.HashSet
import com.codingadventures.heap.MinHeap
import com.codingadventures.hyperloglog.HyperLogLog
import com.codingadventures.inmemorydatastoreprotocol.CommandFrame
import com.codingadventures.inmemorydatastoreprotocol.EngineResponse
import com.codingadventures.inmemorydatastoreprotocol.bulkString
import com.codingadventures.inmemorydatastoreprotocol.error
import com.codingadventures.inmemorydatastoreprotocol.integer
import com.codingadventures.inmemorydatastoreprotocol.ok
import com.codingadventures.radixtree.RadixTree
import com.codingadventures.skiplist.SkipList
import java.math.BigDecimal
import java.nio.charset.StandardCharsets
import java.time.Instant
import java.util.Locale

class DataStoreEngine {
    private val store = Store(16)

    fun executeFrame(frame: CommandFrame?): EngineResponse {
        if (frame == null) return error("ERR protocol error: expected array of bulk strings")
        store.activeDatabase().activeExpire()
        return when (frame.command) {
            "PING" -> ping(frame.args)
            "ECHO" -> echo(frame.args)
            "SET" -> set(frame.args)
            "GET" -> get(frame.args)
            "DEL" -> del(frame.args)
            "EXISTS" -> exists(frame.args)
            "KEYS" -> keys(frame.args)
            "TYPE" -> type(frame.args)
            "RENAME" -> rename(frame.args)
            "APPEND" -> append(frame.args)
            "INCR" -> incrBy(frame.args, 1)
            "DECR" -> incrBy(frame.args, -1)
            "INCRBY" -> incrBy(frame.args, null)
            "DECRBY" -> decrBy(frame.args)
            "HSET" -> hset(frame.args)
            "HGET" -> hget(frame.args)
            "HDEL" -> hdel(frame.args)
            "HGETALL" -> hgetall(frame.args)
            "HLEN" -> hlen(frame.args)
            "HEXISTS" -> hexists(frame.args)
            "HKEYS" -> hkeys(frame.args)
            "HVALS" -> hvals(frame.args)
            "LPUSH" -> pushList(frame.args, true)
            "RPUSH" -> pushList(frame.args, false)
            "LPOP" -> popList(frame.args, true)
            "RPOP" -> popList(frame.args, false)
            "LLEN" -> llen(frame.args)
            "LINDEX" -> lindex(frame.args)
            "LRANGE" -> lrange(frame.args)
            "SADD" -> sadd(frame.args)
            "SREM" -> srem(frame.args)
            "SISMEMBER" -> sismember(frame.args)
            "SMEMBERS" -> smembers(frame.args)
            "SCARD" -> scard(frame.args)
            "SUNION" -> setOperation(frame.args, "sunion", SetOperation.UNION)
            "SINTER" -> setOperation(frame.args, "sinter", SetOperation.INTERSECTION)
            "SDIFF" -> setOperation(frame.args, "sdiff", SetOperation.DIFFERENCE)
            "ZADD" -> zadd(frame.args)
            "ZRANGE" -> zrange(frame.args)
            "ZRANGEBYSCORE" -> zrangeByScore(frame.args)
            "ZRANK" -> zrank(frame.args)
            "ZSCORE" -> zscore(frame.args)
            "ZCARD" -> zcard(frame.args)
            "ZREM" -> zrem(frame.args)
            "PFADD" -> pfadd(frame.args)
            "PFCOUNT" -> pfcount(frame.args)
            "PFMERGE" -> pfmerge(frame.args)
            "EXPIRE" -> expire(frame.args, false)
            "EXPIREAT" -> expire(frame.args, true)
            "TTL" -> ttl(frame.args)
            "PTTL" -> pttl(frame.args)
            "PERSIST" -> persist(frame.args)
            "SELECT" -> select(frame.args)
            "FLUSHDB" -> flushdb(frame.args)
            "FLUSHALL" -> flushall(frame.args)
            "DBSIZE" -> dbsize(frame.args)
            "INFO" -> info(frame.args)
            else -> error("ERR unknown command '${frame.command.lowercase(Locale.ROOT)}'")
        }
    }

    companion object {
        fun currentTimeMs(): Long = Instant.now().toEpochMilli()
    }

    private fun ping(args: List<ByteArray>): EngineResponse = when (args.size) {
        0 -> EngineResponse.SimpleString("PONG")
        1 -> bulkString(args.first())
        else -> wrongArity("ping")
    }

    private fun echo(args: List<ByteArray>): EngineResponse = if (args.size == 1) bulkString(args.first()) else wrongArity("echo")

    private fun set(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("set")
        store.activeDatabase().set(ByteSequence(args[0]), Entry.string(args[1], null))
        return ok()
    }

    private fun get(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("get")
        val entry = keyEntry(args.first()) ?: return bulkString(null)
        if (entry.type != EntryType.STRING) return wrongType()
        return bulkString(entry.value as ByteArray)
    }

    private fun del(args: List<ByteArray>): EngineResponse {
        if (args.isEmpty()) return wrongArity("del")
        return integer(args.count { store.activeDatabase().delete(ByteSequence(it)) }.toLong())
    }

    private fun exists(args: List<ByteArray>): EngineResponse {
        if (args.isEmpty()) return wrongArity("exists")
        return integer(args.count { keyEntry(it) != null }.toLong())
    }

    private fun keys(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("keys")
        return EngineResponse.ArrayValue(store.activeDatabase().keys(args.first()).map { bulkString(it.bytes) })
    }

    private fun type(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("type")
        return EngineResponse.SimpleString(keyEntry(args.first())?.type?.wireName ?: "none")
    }

    private fun rename(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("rename")
        val source = ByteSequence(args[0])
        store.activeDatabase().expireLazy(source)
        val entry = store.activeDatabase().get(source) ?: return error("ERR no such key")
        val destination = ByteSequence(args[1])
        if (source != destination) {
            store.activeDatabase().delete(source)
            store.activeDatabase().set(destination, entry)
        }
        return ok()
    }

    private fun append(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("append")
        val key = ByteSequence(args[0])
        val suffix = args[1]
        val entry = keyEntry(args[0])
        if (entry == null) {
            store.activeDatabase().set(key, Entry.string(suffix, null))
            return integer(suffix.size.toLong())
        }
        if (entry.type != EntryType.STRING) return wrongType()
        val combined = (entry.value as ByteArray) + suffix
        entry.value = combined
        return integer(combined.size.toLong())
    }

    private fun incrBy(args: List<ByteArray>, fixedDelta: Long?): EngineResponse {
        if ((fixedDelta == null && args.size != 2) || (fixedDelta != null && args.size != 1)) {
            return wrongArity(if (fixedDelta == null) "incrby" else if (fixedDelta > 0) "incr" else "decr")
        }
        val delta = fixedDelta ?: parseLong(args[1]) ?: return integerParseError()
        val key = ByteSequence(args.first())
        val entry = keyEntry(args.first())
        val current = when {
            entry == null -> 0L
            entry.type != EntryType.STRING -> return wrongType()
            else -> parseLong(entry.value as ByteArray) ?: return integerParseError()
        }
        val next = current + delta
        store.activeDatabase().set(key, Entry.string(next.toString().encodeToByteArray(), entry?.expiresAtMs))
        return integer(next)
    }

    private fun decrBy(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("decrby")
        val delta = parseLong(args[1]) ?: return integerParseError()
        return incrBy(listOf(args[0], (-delta).toString().encodeToByteArray()), null)
    }

    private fun hset(args: List<ByteArray>): EngineResponse {
        if (args.size < 3 || args.size % 2 == 0) return wrongArity("hset")
        val key = ByteSequence(args.first())
        val entry = keyEntry(args.first()) ?: Entry.hash(HashMap(), null).also { store.activeDatabase().set(key, it) }
        if (entry.type != EntryType.HASH) return wrongType()
        val hash = entry.value as HashMap<ByteSequence, ByteArray>
        var added = 0L
        for (index in 1 until args.size step 2) {
            val field = ByteSequence(args[index])
            if (!hash.has(field)) added += 1
            hash.set(field, args[index + 1])
        }
        return integer(added)
    }

    private fun hget(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("hget")
        val entry = keyEntry(args.first()) ?: return bulkString(null)
        if (entry.type != EntryType.HASH) return wrongType()
        return bulkString((entry.value as HashMap<ByteSequence, ByteArray>)[ByteSequence(args[1])])
    }

    private fun hdel(args: List<ByteArray>): EngineResponse {
        if (args.size < 2) return wrongArity("hdel")
        val key = ByteSequence(args.first())
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.type != EntryType.HASH) return wrongType()
        val hash = entry.value as HashMap<ByteSequence, ByteArray>
        var removed = 0L
        for (field in args.drop(1)) if (hash.delete(ByteSequence(field))) removed += 1
        if (hash.isEmpty()) store.activeDatabase().delete(key)
        return integer(removed)
    }

    private fun hgetall(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("hgetall")
        val entry = keyEntry(args.first()) ?: return EngineResponse.ArrayValue(emptyList())
        if (entry.type != EntryType.HASH) return wrongType()
        val responses = mutableListOf<EngineResponse>()
        for ((field, value) in (entry.value as HashMap<ByteSequence, ByteArray>).entries()) {
            responses += bulkString(field.bytes)
            responses += bulkString(value)
        }
        return EngineResponse.ArrayValue(responses)
    }

    private fun hlen(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("hlen")
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.type != EntryType.HASH) return wrongType()
        return integer((entry.value as HashMap<ByteSequence, ByteArray>).size.toLong())
    }

    private fun hexists(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("hexists")
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.type != EntryType.HASH) return wrongType()
        return integer(if ((entry.value as HashMap<ByteSequence, ByteArray>).has(ByteSequence(args[1]))) 1 else 0)
    }

    private fun hkeys(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("hkeys")
        val entry = keyEntry(args.first()) ?: return EngineResponse.ArrayValue(emptyList())
        if (entry.type != EntryType.HASH) return wrongType()
        return EngineResponse.ArrayValue((entry.value as HashMap<ByteSequence, ByteArray>).keys().map { bulkString(it.bytes) })
    }

    private fun hvals(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("hvals")
        val entry = keyEntry(args.first()) ?: return EngineResponse.ArrayValue(emptyList())
        if (entry.type != EntryType.HASH) return wrongType()
        return EngineResponse.ArrayValue((entry.value as HashMap<ByteSequence, ByteArray>).values().map(::bulkString))
    }

    private fun pushList(args: List<ByteArray>, left: Boolean): EngineResponse {
        if (args.size < 2) return wrongArity(if (left) "lpush" else "rpush")
        val entry = ensureList(args.first()) ?: return wrongType()
        val list = entry.value as MutableList<ByteArray>
        for (value in args.drop(1)) if (left) list.add(0, value) else list.add(value)
        return integer(list.size.toLong())
    }

    private fun popList(args: List<ByteArray>, left: Boolean): EngineResponse {
        if (args.size != 1) return wrongArity(if (left) "lpop" else "rpop")
        val key = ByteSequence(args.first())
        val entry = keyEntry(args.first()) ?: return bulkString(null)
        if (entry.type != EntryType.LIST) return wrongType()
        val list = entry.value as MutableList<ByteArray>
        val value = list.removeAt(if (left) 0 else list.lastIndex)
        if (list.isEmpty()) store.activeDatabase().delete(key)
        return bulkString(value)
    }

    private fun llen(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("llen")
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.type != EntryType.LIST) return wrongType()
        return integer((entry.value as MutableList<ByteArray>).size.toLong())
    }

    private fun lindex(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("lindex")
        val entry = keyEntry(args.first()) ?: return bulkString(null)
        if (entry.type != EntryType.LIST) return wrongType()
        val list = entry.value as MutableList<ByteArray>
        val index = parseInt(args[1]) ?: return error("ERR value is not an integer or out of range")
        val resolved = if (index < 0) list.size + index else index
        return if (resolved in list.indices) bulkString(list[resolved]) else bulkString(null)
    }

    private fun lrange(args: List<ByteArray>): EngineResponse {
        if (args.size != 3) return wrongArity("lrange")
        val entry = keyEntry(args.first()) ?: return EngineResponse.ArrayValue(emptyList())
        if (entry.type != EntryType.LIST) return wrongType()
        val list = entry.value as MutableList<ByteArray>
        val startValue = parseInt(args[1]) ?: return error("ERR value is not an integer or out of range")
        val stopValue = parseInt(args[2]) ?: return error("ERR value is not an integer or out of range")
        val start = (if (startValue < 0) list.size + startValue else startValue).coerceAtLeast(0)
        val stop = (if (stopValue < 0) list.size + stopValue else stopValue).coerceAtMost(list.lastIndex)
        if (list.isEmpty() || start > stop || start >= list.size) return EngineResponse.ArrayValue(emptyList())
        return EngineResponse.ArrayValue((start..stop).map { bulkString(list[it]) })
    }

    private fun sadd(args: List<ByteArray>): EngineResponse {
        if (args.size < 2) return wrongArity("sadd")
        val entry = ensureSet(args.first()) ?: return wrongType()
        val set = entry.value as HashSet<ByteSequence>
        var added = 0L
        for (value in args.drop(1).map(::ByteSequence)) {
            if (!set.contains(value)) {
                set.add(value)
                added += 1
            }
        }
        return integer(added)
    }

    private fun srem(args: List<ByteArray>): EngineResponse {
        if (args.size < 2) return wrongArity("srem")
        val key = ByteSequence(args.first())
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.type != EntryType.SET) return wrongType()
        val set = entry.value as HashSet<ByteSequence>
        val removed = args.drop(1).count { set.remove(ByteSequence(it)) }
        if (set.isEmpty()) store.activeDatabase().delete(key)
        return integer(removed.toLong())
    }

    private fun sismember(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("sismember")
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.type != EntryType.SET) return wrongType()
        return integer(if ((entry.value as HashSet<ByteSequence>).contains(ByteSequence(args[1]))) 1 else 0)
    }

    private fun smembers(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("smembers")
        val entry = keyEntry(args.first()) ?: return EngineResponse.ArrayValue(emptyList())
        if (entry.type != EntryType.SET) return wrongType()
        return EngineResponse.ArrayValue((entry.value as HashSet<ByteSequence>).items().sorted().map { bulkString(it.bytes) })
    }

    private fun scard(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("scard")
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.type != EntryType.SET) return wrongType()
        return integer((entry.value as HashSet<ByteSequence>).size.toLong())
    }

    private fun setOperation(args: List<ByteArray>, command: String, operation: SetOperation): EngineResponse {
        if (args.isEmpty()) return wrongArity(command)
        var result = HashSet<ByteSequence>()
        var first = true
        for (rawKey in args) {
            val entry = keyEntry(rawKey)
            val next = if (entry == null) {
                HashSet()
            } else {
                castSet(entry.value, entry.type) ?: return wrongType()
            }
            result = when (operation) {
                SetOperation.UNION -> if (first) next.copy() else result.union(next)
                SetOperation.INTERSECTION -> if (first) next.copy() else result.intersection(next)
                SetOperation.DIFFERENCE -> if (first) next.copy() else result.difference(next)
            }
            first = false
        }
        return EngineResponse.ArrayValue(result.items().sorted().map { bulkString(it.bytes) })
    }

    private fun zadd(args: List<ByteArray>): EngineResponse {
        if (args.size < 3 || args.size % 2 == 0) return wrongArity("zadd")
        val key = ByteSequence(args.first())
        val entry = keyEntry(args.first())
        val zset = when {
            entry == null -> SortedSet().also { store.activeDatabase().set(key, Entry.zset(it, null)) }
            entry.type != EntryType.ZSET -> return wrongType()
            else -> entry.value as SortedSet
        }
        var added = 0L
        for (index in 1 until args.size step 2) {
            val score = parseDouble(args[index]) ?: return floatParseError()
            if (zset.insert(score, ByteSequence(args[index + 1]))) added += 1
        }
        return integer(added)
    }

    private fun zrange(args: List<ByteArray>): EngineResponse {
        if (args.size !in 3..4) return wrongArity("zrange")
        val start = parseInt(args[1]) ?: return integerParseError()
        val end = parseInt(args[2]) ?: return integerParseError()
        val entry = keyEntry(args.first()) ?: return EngineResponse.ArrayValue(emptyList())
        if (entry.type != EntryType.ZSET) return wrongType()
        val withScores = args.size == 4 && args[3].decodeToString().equals("WITHSCORES", ignoreCase = true)
        return EngineResponse.ArrayValue(flattenZset((entry.value as SortedSet).rangeByIndex(start, end), withScores))
    }

    private fun zrangeByScore(args: List<ByteArray>): EngineResponse {
        if (args.size !in 3..4) return wrongArity("zrangebyscore")
        val min = parseDouble(args[1]) ?: return floatParseError()
        val max = parseDouble(args[2]) ?: return floatParseError()
        val entry = keyEntry(args.first()) ?: return EngineResponse.ArrayValue(emptyList())
        if (entry.type != EntryType.ZSET) return wrongType()
        val withScores = args.size == 4 && args[3].decodeToString().equals("WITHSCORES", ignoreCase = true)
        return EngineResponse.ArrayValue(flattenZset((entry.value as SortedSet).rangeByScore(min, max), withScores))
    }

    private fun zrank(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("zrank")
        val entry = keyEntry(args.first()) ?: return bulkString(null)
        if (entry.type != EntryType.ZSET) return wrongType()
        return (entry.value as SortedSet).rank(ByteSequence(args[1]))?.let { integer(it.toLong()) } ?: bulkString(null)
    }

    private fun zscore(args: List<ByteArray>): EngineResponse {
        if (args.size != 2) return wrongArity("zscore")
        val entry = keyEntry(args.first()) ?: return bulkString(null)
        if (entry.type != EntryType.ZSET) return wrongType()
        return (entry.value as SortedSet).score(ByteSequence(args[1]))
            ?.let { bulkString(formatScore(it).encodeToByteArray()) }
            ?: bulkString(null)
    }

    private fun zcard(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("zcard")
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.type != EntryType.ZSET) return wrongType()
        return integer((entry.value as SortedSet).size().toLong())
    }

    private fun zrem(args: List<ByteArray>): EngineResponse {
        if (args.size < 2) return wrongArity("zrem")
        val key = ByteSequence(args.first())
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.type != EntryType.ZSET) return wrongType()
        val zset = entry.value as SortedSet
        val removed = args.drop(1).count { zset.remove(ByteSequence(it)) }
        if (zset.isEmpty()) store.activeDatabase().delete(key)
        return integer(removed.toLong())
    }

    private fun pfadd(args: List<ByteArray>): EngineResponse {
        if (args.size < 2) return wrongArity("pfadd")
        val key = ByteSequence(args.first())
        val entry = keyEntry(args.first())
        val sketch = when {
            entry == null -> HyperLogLog().also { store.activeDatabase().set(key, Entry.hll(it, null)) }
            entry.type != EntryType.HLL -> return wrongType()
            else -> entry.value as HyperLogLog
        }
        val before = sketch.copy()
        args.drop(1).forEach(sketch::add)
        return integer(if (before == sketch) 0 else 1)
    }

    private fun pfcount(args: List<ByteArray>): EngineResponse {
        if (args.isEmpty()) return wrongArity("pfcount")
        var aggregate: HyperLogLog? = null
        for (rawKey in args) {
            val entry = keyEntry(rawKey) ?: continue
            if (entry.type != EntryType.HLL) return wrongType()
            aggregate = if (aggregate == null) (entry.value as HyperLogLog).copy() else aggregate.merge(entry.value as HyperLogLog)
        }
        return integer((aggregate ?: HyperLogLog()).count().toLong())
    }

    private fun pfmerge(args: List<ByteArray>): EngineResponse {
        if (args.size < 2) return wrongArity("pfmerge")
        var merged: HyperLogLog? = null
        for (rawKey in args.drop(1)) {
            val entry = keyEntry(rawKey) ?: continue
            if (entry.type != EntryType.HLL) return wrongType()
            merged = if (merged == null) (entry.value as HyperLogLog).copy() else merged.merge(entry.value as HyperLogLog)
        }
        val expiresAt = keyEntry(args.first())?.expiresAtMs
        store.activeDatabase().set(ByteSequence(args.first()), Entry.hll(merged ?: HyperLogLog(), expiresAt))
        return ok()
    }

    private fun expire(args: List<ByteArray>, absoluteSeconds: Boolean): EngineResponse {
        if (args.size != 2) return wrongArity(if (absoluteSeconds) "expireat" else "expire")
        val key = ByteSequence(args.first())
        val entry = keyEntry(args.first()) ?: return integer(0)
        val parsed = parseLong(args[1]) ?: return integerParseError()
        val expiresAt = if (absoluteSeconds) parsed * 1000L else currentTimeMs() + parsed * 1000L
        entry.expiresAtMs = expiresAt
        store.activeDatabase().ttlHeap.push(ExpiryRecord(expiresAt, key))
        return integer(1)
    }

    private fun ttl(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("ttl")
        val key = ByteSequence(args.first())
        store.activeDatabase().expireLazy(key)
        val entry = store.activeDatabase().get(key) ?: return integer(-2)
        if (entry.expiresAtMs == null) return integer(-1)
        val remaining = entry.expiresAtMs!! - currentTimeMs()
        if (remaining < 0) {
            store.activeDatabase().delete(key)
            return integer(-2)
        }
        return integer(remaining / 1000L)
    }

    private fun pttl(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("pttl")
        val entry = keyEntry(args.first()) ?: return integer(-2)
        if (entry.expiresAtMs == null) return integer(-1)
        return integer(maxOf(-1, entry.expiresAtMs!! - currentTimeMs()))
    }

    private fun persist(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("persist")
        val entry = keyEntry(args.first()) ?: return integer(0)
        if (entry.expiresAtMs == null) return integer(0)
        entry.expiresAtMs = null
        return integer(1)
    }

    private fun select(args: List<ByteArray>): EngineResponse {
        if (args.size != 1) return wrongArity("select")
        val index = parseInt(args.first()) ?: return error("ERR DB index is out of range")
        if (index !in 0 until store.databaseCount()) return error("ERR DB index is out of range")
        store.select(index)
        return ok()
    }

    private fun flushdb(args: List<ByteArray>): EngineResponse = if (args.isEmpty()) {
        store.flushdb(); ok()
    } else wrongArity("flushdb")

    private fun flushall(args: List<ByteArray>): EngineResponse = if (args.isEmpty()) {
        store.flushall(); ok()
    } else wrongArity("flushall")

    private fun dbsize(args: List<ByteArray>): EngineResponse = if (args.isEmpty()) integer(store.activeDatabase().dbsize().toLong()) else wrongArity("dbsize")

    private fun info(args: List<ByteArray>): EngineResponse {
        if (args.isNotEmpty()) return wrongArity("info")
        val text = "# Server\r\nmini_redis_kotlin:0.1.0\r\nactive_db:${store.activeDb}\r\ndbsize:${store.activeDatabase().dbsize()}\r\n"
        return bulkString(text.encodeToByteArray())
    }

    private fun keyEntry(rawKey: ByteArray): Entry? {
        val key = ByteSequence(rawKey)
        store.activeDatabase().expireLazy(key)
        return store.activeDatabase().get(key)
    }

    private fun ensureList(rawKey: ByteArray): Entry? {
        val key = ByteSequence(rawKey)
        store.activeDatabase().expireLazy(key)
        val current = store.activeDatabase().get(key)
        if (current == null) {
            return Entry.list(mutableListOf(), null).also { store.activeDatabase().set(key, it) }
        }
        return current.takeIf { it.type == EntryType.LIST }
    }

    private fun ensureSet(rawKey: ByteArray): Entry? {
        val key = ByteSequence(rawKey)
        store.activeDatabase().expireLazy(key)
        val current = store.activeDatabase().get(key)
        if (current == null) {
            return Entry.set(HashSet(), null).also { store.activeDatabase().set(key, it) }
        }
        return current.takeIf { it.type == EntryType.SET }
    }

    private fun castSet(value: Any, type: EntryType): HashSet<ByteSequence>? = if (type == EntryType.SET) value as HashSet<ByteSequence> else null

    private fun wrongArity(command: String): EngineResponse = error("ERR wrong number of arguments for '$command' command")
    private fun wrongType(): EngineResponse = error("WRONGTYPE Operation against a key holding the wrong kind of value")
    private fun integerParseError(): EngineResponse = error("ERR value is not an integer or out of range")
    private fun floatParseError(): EngineResponse = error("ERR value is not a valid float")
    private fun parseLong(bytes: ByteArray): Long? = bytes.decodeToString().toLongOrNull()
    private fun parseInt(bytes: ByteArray): Int? = bytes.decodeToString().toIntOrNull()
    private fun parseDouble(bytes: ByteArray): Double? = bytes.decodeToString().toDoubleOrNull()?.takeIf(Double::isFinite)
    private fun formatScore(score: Double): String = BigDecimal.valueOf(score).stripTrailingZeros().toPlainString()
    private fun flattenZset(values: List<Pair<ByteSequence, Double>>, withScores: Boolean): List<EngineResponse> = buildList {
        values.forEach { (member, score) ->
            add(bulkString(member.bytes))
            if (withScores) add(bulkString(formatScore(score).encodeToByteArray()))
        }
    }
}

enum class SetOperation { UNION, INTERSECTION, DIFFERENCE }

enum class EntryType(val wireName: String) { STRING("string"), HASH("hash"), LIST("list"), SET("set"), ZSET("zset"), HLL("hll") }

data class Entry(val type: EntryType, var value: Any, var expiresAtMs: Long?) {
    companion object {
        fun string(value: ByteArray, expiresAtMs: Long?) = Entry(EntryType.STRING, value, expiresAtMs)
        fun hash(value: HashMap<ByteSequence, ByteArray>, expiresAtMs: Long?) = Entry(EntryType.HASH, value, expiresAtMs)
        fun list(value: MutableList<ByteArray>, expiresAtMs: Long?) = Entry(EntryType.LIST, value, expiresAtMs)
        fun set(value: HashSet<ByteSequence>, expiresAtMs: Long?) = Entry(EntryType.SET, value, expiresAtMs)
        fun zset(value: SortedSet, expiresAtMs: Long?) = Entry(EntryType.ZSET, value, expiresAtMs)
        fun hll(value: HyperLogLog, expiresAtMs: Long?) = Entry(EntryType.HLL, value, expiresAtMs)
    }
}

data class SortedEntry(val score: Double, val member: ByteSequence)

class SortedSet {
    private val members = HashMap<ByteSequence, Double>()
    private val ordering = SkipList<SortedEntry, Byte>(comparator = { left, right ->
        compareValuesBy(left, right, SortedEntry::score, SortedEntry::member)
    })

    fun insert(score: Double, member: ByteSequence): Boolean {
        require(!score.isNaN()) { "sorted set score cannot be NaN" }
        val isNew = !members.has(member)
        members[member]?.let { ordering.delete(SortedEntry(it, member)) }
        members.set(member, score)
        ordering.insert(SortedEntry(score, member), 0)
        return isNew
    }

    fun remove(member: ByteSequence): Boolean {
        val score = members[member] ?: return false
        ordering.delete(SortedEntry(score, member))
        members.delete(member)
        return true
    }

    fun rank(member: ByteSequence): Int? = members[member]?.let { ordering.rank(SortedEntry(it, member)) }
    fun score(member: ByteSequence): Double? = members[member]
    fun size(): Int = members.size
    fun isEmpty(): Boolean = members.isEmpty()
    fun orderedEntries(): List<Pair<ByteSequence, Double>> = ordering.entries().map { it.first.member to it.first.score }

    fun rangeByIndex(start: Int, end: Int): List<Pair<ByteSequence, Double>> {
        val entries = orderedEntries()
        if (entries.isEmpty()) return emptyList()
        val length = entries.size
        val normalizedStart = if (start < 0) length + start else start
        val normalizedEnd = if (end < 0) length + end else end
        if (normalizedStart < 0 || normalizedEnd < 0 || normalizedStart >= length || normalizedStart > normalizedEnd) return emptyList()
        return entries.subList(normalizedStart, minOf(length, normalizedEnd + 1))
    }

    fun rangeByScore(min: Double, max: Double): List<Pair<ByteSequence, Double>> {
        require(!min.isNaN() && !max.isNaN()) { "sorted set score cannot be NaN" }
        return orderedEntries().filter { (_, score) -> score in min..max }
    }
}

class Store(count: Int) {
    private val databases = List(count) { Database() }
    var activeDb: Int = 0
        private set

    fun activeDatabase(): Database = databases[activeDb]
    fun select(index: Int) { activeDb = index }
    fun databaseCount(): Int = databases.size
    fun flushdb() = activeDatabase().clear()
    fun flushall() = databases.forEach { it.clear() }
}

class Database {
    private val entries = HashMap<ByteSequence, Entry>()
    val ttlHeap = MinHeap<ExpiryRecord>()
    private var keyIndex = RadixTree<ByteSequence>()

    fun get(key: ByteSequence): Entry? {
        val entry = entries[key] ?: return null
        return if (entry.expiresAtMs != null && entry.expiresAtMs!! <= DataStoreEngine.currentTimeMs()) null else entry
    }

    fun set(key: ByteSequence, entry: Entry) {
        entries.set(key, entry)
        keyIndex.insert(indexKey(key), key)
        entry.expiresAtMs?.let { ttlHeap.push(ExpiryRecord(it, key)) }
    }

    fun delete(key: ByteSequence): Boolean = entries.delete(key).also { deleted ->
        if (deleted) keyIndex.delete(indexKey(key))
    }

    fun expireLazy(key: ByteSequence) {
        val entry = entries[key] ?: return
        if (entry.expiresAtMs != null && entry.expiresAtMs!! <= DataStoreEngine.currentTimeMs()) delete(key)
    }

    fun activeExpire() {
        val now = DataStoreEngine.currentTimeMs()
        while (!ttlHeap.isEmpty()) {
            val record = ttlHeap.peek() ?: break
            if (record.expiresAtMs > now) break
            ttlHeap.pop()
            val entry = entries[record.key]
            if (entry != null && entry.expiresAtMs == record.expiresAtMs) delete(record.key)
        }
    }

    fun keys(pattern: ByteArray): List<ByteSequence> {
        val prefix = literalPrefix(pattern)
        val candidates = if (prefix.isEmpty()) keyIndex.keys() else keyIndex.wordsWithPrefix(indexKey(prefix))
        return candidates.map(::fromIndexKey).filter {
            expireLazy(it)
            entries[it] != null && globMatch(pattern, it.bytes)
        }.sorted()
    }

    fun dbsize(): Int {
        activeExpire()
        return entries.size
    }

    fun clear() {
        entries.clear()
        keyIndex = RadixTree()
    }
}

private fun literalPrefix(pattern: ByteArray): ByteArray =
    pattern.copyOf(pattern.indexOfFirst { it == '*'.code.toByte() || it == '?'.code.toByte() }.let { if (it == -1) pattern.size else it })

private fun indexKey(key: ByteSequence): String = indexKey(key.bytes)

private fun indexKey(bytes: ByteArray): String = String(bytes, StandardCharsets.ISO_8859_1)

private fun fromIndexKey(key: String): ByteSequence = ByteSequence(key.toByteArray(StandardCharsets.ISO_8859_1))

data class ExpiryRecord(val expiresAtMs: Long, val key: ByteSequence) : Comparable<ExpiryRecord> {
    override fun compareTo(other: ExpiryRecord): Int = compareValuesBy(this, other, ExpiryRecord::expiresAtMs, ExpiryRecord::key)
}

class ByteSequence(val bytes: ByteArray) : Comparable<ByteSequence> {
    override fun equals(other: Any?): Boolean = other is ByteSequence && bytes.contentEquals(other.bytes)
    override fun hashCode(): Int = bytes.contentHashCode()
    override fun compareTo(other: ByteSequence): Int {
        val limit = minOf(bytes.size, other.bytes.size)
        for (index in 0 until limit) {
            val diff = bytes[index].toUByte().compareTo(other.bytes[index].toUByte())
            if (diff != 0) return diff
        }
        return bytes.size.compareTo(other.bytes.size)
    }
}

private fun globMatch(pattern: ByteArray, text: ByteArray, p: Int = 0, t: Int = 0): Boolean {
    if (p == pattern.size) return t == text.size
    if (pattern[p] == '*'.code.toByte()) return (t..text.size).any { globMatch(pattern, text, p + 1, it) }
    if (t == text.size) return false
    return if (pattern[p] == '?'.code.toByte() || pattern[p] == text[t]) globMatch(pattern, text, p + 1, t + 1) else false
}
