package datastoreengine

import (
	"fmt"
	"strconv"
	"strings"

	hashmap "github.com/adhithyan15/coding-adventures/code/packages/go/hash-map"
	hashset "github.com/adhithyan15/coding-adventures/code/packages/go/hash-set"
	hyperloglog "github.com/adhithyan15/coding-adventures/code/packages/go/hyperloglog"
	resp "github.com/adhithyan15/coding-adventures/code/packages/go/resp-protocol"
)

func registerBuiltinCommands(engine *DataStoreEngine) {
	register := func(name string, mutating bool, skipLazyExpire bool, handler CommandHandler) {
		engine.RegisterCommand(name, mutating, skipLazyExpire, handler)
	}

	register("PING", false, true, cmdPing)
	register("ECHO", false, true, cmdEcho)
	register("SET", true, false, cmdSet)
	register("GET", false, false, cmdGet)
	register("DEL", true, false, cmdDel)
	register("EXISTS", false, false, cmdExists)
	register("TYPE", false, false, cmdType)
	register("RENAME", true, false, cmdRename)
	register("INCR", true, false, cmdIncr)
	register("DECR", true, false, cmdDecr)
	register("INCRBY", true, false, cmdIncrBy)
	register("DECRBY", true, false, cmdDecrBy)
	register("APPEND", true, false, cmdAppend)
	register("HSET", true, false, cmdHSet)
	register("HGET", false, false, cmdHGet)
	register("HDEL", true, false, cmdHDel)
	register("HGETALL", false, false, cmdHGetAll)
	register("HLEN", false, false, cmdHLen)
	register("HEXISTS", false, false, cmdHExists)
	register("HKEYS", false, false, cmdHKeys)
	register("HVALS", false, false, cmdHVals)
	register("SADD", true, false, cmdSAdd)
	register("SREM", true, false, cmdSRem)
	register("SISMEMBER", false, false, cmdSIsMember)
	register("SMEMBERS", false, false, cmdSMembers)
	register("SCARD", false, false, cmdSCard)
	register("PFADD", true, false, cmdPFAdd)
	register("PFCOUNT", false, false, cmdPFCount)
	register("PFMERGE", true, false, cmdPFMerge)
	register("EXPIRE", true, false, cmdExpire)
	register("EXPIREAT", true, false, cmdExpireAt)
	register("TTL", false, false, cmdTTL)
	register("PTTL", false, false, cmdPTTL)
	register("PERSIST", true, false, cmdPersist)
	register("SELECT", true, true, cmdSelect)
	register("FLUSHDB", true, true, cmdFlushDB)
	register("FLUSHALL", true, true, cmdFlushAll)
	register("DBSIZE", false, true, cmdDBSize)
	register("INFO", false, true, cmdInfo)
	register("KEYS", false, false, cmdKeys)
}

func dispatch(store *Store, parts [][]byte) (*Store, resp.Value) {
	if len(parts) == 0 {
		return store, errValue("ERR empty command")
	}
	engine := FromStore(store)
	return engine.Store(), engine.ExecuteParts(parts)
}

func isMutating(parts [][]byte) bool {
	if len(parts) == 0 {
		return false
	}
	switch strings.ToUpper(string(parts[0])) {
	case "SET", "DEL", "RENAME", "INCR", "DECR", "INCRBY", "DECRBY", "APPEND",
		"HSET", "HDEL", "SADD", "SREM", "PFADD", "PFMERGE", "EXPIRE", "EXPIREAT",
		"PERSIST", "SELECT", "FLUSHDB", "FLUSHALL":
		return true
	default:
		return false
	}
}

func cmdPing(store *Store, args [][]byte) (*Store, resp.Value) {
	switch len(args) {
	case 0:
		return store, resp.SimpleString("PONG")
	case 1:
		return store, resp.BulkString(cloneBytes(args[0]))
	default:
		return store, errValue("ERR wrong number of arguments for 'PING'")
	}
}

func cmdEcho(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'ECHO'")
	}
	return store, resp.BulkString(cloneBytes(args[0]))
}

func cmdSet(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) < 2 {
		return store, errValue("ERR wrong number of arguments for 'SET'")
	}
	key := cloneBytes(args[0])
	value := cloneBytes(args[1])
	var expiresAt *uint64
	nx := false
	xx := false
	for i := 2; i < len(args); {
		switch strings.ToUpper(string(args[i])) {
		case "EX":
			if i+1 >= len(args) {
				return store, errValue("ERR syntax error")
			}
			seconds, err := parseInt64(args[i+1])
			if err != nil {
				return store, errValue(err.Error())
			}
			expiresAt = expirationFromSeconds(seconds)
			i += 2
		case "PX":
			if i+1 >= len(args) {
				return store, errValue("ERR syntax error")
			}
			millis, err := parseInt64(args[i+1])
			if err != nil {
				return store, errValue(err.Error())
			}
			expiresAt = expirationFromMillis(millis)
			i += 2
		case "NX":
			nx = true
			i++
		case "XX":
			xx = true
			i++
		default:
			return store, errValue("ERR syntax error")
		}
	}
	if nx && xx {
		return store, errValue("ERR syntax error")
	}
	exists := store.Exists(key)
	if nx && exists {
		return store, resp.NullBulkString()
	}
	if xx && !exists {
		return store, resp.NullBulkString()
	}
	return store.Set(key, NewStringEntry(value, expiresAt)), okValue()
}

func cmdGet(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'GET'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.NullBulkString()
	}
	if entry.Type != EntryTypeString {
		return store, wrongTypeValue()
	}
	return store, resp.BulkString(entry.Value.([]byte))
}

func cmdDel(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) == 0 {
		return store, errValue("ERR wrong number of arguments for 'DEL'")
	}
	removed := int64(0)
	for _, key := range args {
		if store.Exists(key) {
			removed++
			store = store.Delete(key)
		}
	}
	return store, resp.Integer(removed)
}

func cmdExists(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) == 0 {
		return store, errValue("ERR wrong number of arguments for 'EXISTS'")
	}
	count := int64(0)
	for _, key := range args {
		if store.Exists(key) {
			count++
		}
	}
	return store, resp.Integer(count)
}

func cmdType(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'TYPE'")
	}
	if entryType, ok := store.TypeOf(args[0]); ok {
		return store, resp.SimpleString(entryType.String())
	}
	return store, resp.SimpleString("none")
}

func cmdRename(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 2 {
		return store, errValue("ERR wrong number of arguments for 'RENAME'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, errValue("ERR no such key")
	}
	store = store.Delete(args[0])
	store = store.Set(cloneBytes(args[1]), entry)
	return store, okValue()
}

func cmdIncr(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'INCR'")
	}
	return cmdIncrBy(store, [][]byte{args[0], []byte("1")})
}

func cmdDecr(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'DECR'")
	}
	return cmdIncrBy(store, [][]byte{args[0], []byte("-1")})
}

func cmdIncrBy(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 2 {
		return store, errValue("ERR wrong number of arguments for 'INCRBY'")
	}
	delta, err := parseInt64(args[1])
	if err != nil {
		return store, errValue(err.Error())
	}
	return adjustInteger(store, args[0], delta)
}

func cmdDecrBy(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 2 {
		return store, errValue("ERR wrong number of arguments for 'DECRBY'")
	}
	delta, err := parseInt64(args[1])
	if err != nil {
		return store, errValue(err.Error())
	}
	return adjustInteger(store, args[0], -delta)
}

func cmdAppend(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 2 {
		return store, errValue("ERR wrong number of arguments for 'APPEND'")
	}
	key := cloneBytes(args[0])
	suffix := args[1]
	value := []byte{}
	var expiresAt *uint64
	if entry, ok := store.Get(args[0]); ok {
		if entry.Type != EntryTypeString {
			return store, wrongTypeValue()
		}
		value = append([]byte(nil), entry.Value.([]byte)...)
		expiresAt = entry.ExpiresAt
	}
	value = append(value, suffix...)
	return store.Set(key, NewStringEntry(value, expiresAt)), resp.Integer(int64(len(value)))
}

func cmdHSet(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) < 3 || len(args)%2 == 0 {
		return store, errValue("ERR wrong number of arguments for 'HSET'")
	}
	key := cloneBytes(args[0])
	expiresAt := currentExpiry(store, key)
	m, _, _ := getOrCreateHash(store, key, expiresAt)
	added := int64(0)
	for i := 1; i < len(args); i += 2 {
		field := cloneBytes(args[i])
		value := cloneBytes(args[i+1])
		if !m.Has(field) {
			added++
		}
		m.Set(field, value)
	}
	return store.Set(key, NewHashEntry(m, expiresAt)), resp.Integer(added)
}

func cmdHGet(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 2 {
		return store, errValue("ERR wrong number of arguments for 'HGET'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.NullBulkString()
	}
	if entry.Type != EntryTypeHash {
		return store, wrongTypeValue()
	}
	m := entry.Value.(*hashmap.HashMap[[]byte])
	value, ok := m.Get(args[1])
	if !ok {
		return store, resp.NullBulkString()
	}
	return store, resp.BulkString(value)
}

func cmdHDel(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) < 2 {
		return store, errValue("ERR wrong number of arguments for 'HDEL'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Integer(0)
	}
	if entry.Type != EntryTypeHash {
		return store, wrongTypeValue()
	}
	m := entry.Value.(*hashmap.HashMap[[]byte]).Clone()
	removed := int64(0)
	for _, field := range args[1:] {
		if m.Delete(field) {
			removed++
		}
	}
	return store.Set(args[0], NewHashEntry(m, entry.ExpiresAt)), resp.Integer(removed)
}

func cmdHGetAll(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'HGETALL'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Array(nil)
	}
	if entry.Type != EntryTypeHash {
		return store, wrongTypeValue()
	}
	m := entry.Value.(*hashmap.HashMap[[]byte])
	values := make([]resp.Value, 0, m.Size()*2)
	for _, kv := range m.Entries() {
		values = append(values, resp.BulkString(kv.Key), resp.BulkString(kv.Value))
	}
	return store, resp.Array(values)
}

func cmdHLen(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'HLEN'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Integer(0)
	}
	if entry.Type != EntryTypeHash {
		return store, wrongTypeValue()
	}
	return store, resp.Integer(int64(entry.Value.(*hashmap.HashMap[[]byte]).Size()))
}

func cmdHExists(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 2 {
		return store, errValue("ERR wrong number of arguments for 'HEXISTS'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Integer(0)
	}
	if entry.Type != EntryTypeHash {
		return store, wrongTypeValue()
	}
	if entry.Value.(*hashmap.HashMap[[]byte]).Has(args[1]) {
		return store, resp.Integer(1)
	}
	return store, resp.Integer(0)
}

func cmdHKeys(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'HKEYS'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Array(nil)
	}
	if entry.Type != EntryTypeHash {
		return store, wrongTypeValue()
	}
	values := make([]resp.Value, 0)
	for _, key := range entry.Value.(*hashmap.HashMap[[]byte]).Keys() {
		values = append(values, resp.BulkString(key))
	}
	return store, resp.Array(values)
}

func cmdHVals(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'HVALS'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Array(nil)
	}
	if entry.Type != EntryTypeHash {
		return store, wrongTypeValue()
	}
	values := make([]resp.Value, 0)
	for _, kv := range entry.Value.(*hashmap.HashMap[[]byte]).Entries() {
		values = append(values, resp.BulkString(kv.Value))
	}
	return store, resp.Array(values)
}

func cmdSAdd(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) < 2 {
		return store, errValue("ERR wrong number of arguments for 'SADD'")
	}
	key := cloneBytes(args[0])
	expiresAt := currentExpiry(store, key)
	set, _, _ := getOrCreateSet(store, key, expiresAt)
	added := int64(0)
	for _, member := range args[1:] {
		if set.Add(member) {
			added++
		}
	}
	return store.Set(key, NewSetEntry(set, expiresAt)), resp.Integer(added)
}

func cmdSRem(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) < 2 {
		return store, errValue("ERR wrong number of arguments for 'SREM'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Integer(0)
	}
	if entry.Type != EntryTypeSet {
		return store, wrongTypeValue()
	}
	set := entry.Value.(*hashset.HashSet).Clone()
	removed := int64(0)
	for _, member := range args[1:] {
		if set.Remove(member) {
			removed++
		}
	}
	return store.Set(args[0], NewSetEntry(set, entry.ExpiresAt)), resp.Integer(removed)
}

func cmdSIsMember(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 2 {
		return store, errValue("ERR wrong number of arguments for 'SISMEMBER'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Integer(0)
	}
	if entry.Type != EntryTypeSet {
		return store, wrongTypeValue()
	}
	if entry.Value.(*hashset.HashSet).Contains(args[1]) {
		return store, resp.Integer(1)
	}
	return store, resp.Integer(0)
}

func cmdSMembers(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'SMEMBERS'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Array(nil)
	}
	if entry.Type != EntryTypeSet {
		return store, wrongTypeValue()
	}
	values := make([]resp.Value, 0)
	for _, member := range entry.Value.(*hashset.HashSet).ToSlice() {
		values = append(values, resp.BulkString(member))
	}
	return store, resp.Array(values)
}

func cmdSCard(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'SCARD'")
	}
	entry, ok := store.Get(args[0])
	if !ok {
		return store, resp.Integer(0)
	}
	if entry.Type != EntryTypeSet {
		return store, wrongTypeValue()
	}
	return store, resp.Integer(int64(entry.Value.(*hashset.HashSet).Size()))
}

func cmdPFAdd(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) < 2 {
		return store, errValue("ERR wrong number of arguments for 'PFADD'")
	}
	key := cloneBytes(args[0])
	expiresAt := currentExpiry(store, key)
	hll, _, _ := getOrCreateHLL(store, key, expiresAt)
	changed := false
	for _, value := range args[1:] {
		changed = hll.Add(value) || changed
	}
	return store.Set(key, NewHLLEntry(hll, expiresAt)), resp.Integer(boolToInt64(changed))
}

func cmdPFCount(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) == 0 {
		return store, errValue("ERR wrong number of arguments for 'PFCOUNT'")
	}
	if len(args) == 1 {
		entry, ok := store.Get(args[0])
		if !ok {
			return store, resp.Integer(0)
		}
		if entry.Type != EntryTypeHLL {
			return store, wrongTypeValue()
		}
		return store, resp.Integer(int64(entry.Value.(*hyperloglog.HyperLogLog).Count()))
	}
	merged := hyperloglog.New()
	for _, key := range args {
		entry, ok := store.Get(key)
		if !ok {
			continue
		}
		if entry.Type != EntryTypeHLL {
			return store, wrongTypeValue()
		}
		merged.Merge(entry.Value.(*hyperloglog.HyperLogLog))
	}
	return store, resp.Integer(int64(merged.Count()))
}

func cmdPFMerge(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) < 2 {
		return store, errValue("ERR wrong number of arguments for 'PFMERGE'")
	}
	dst := cloneBytes(args[0])
	merged := hyperloglog.New()
	var expiresAt *uint64
	for _, key := range args[1:] {
		entry, ok := store.Get(key)
		if !ok {
			continue
		}
		if entry.Type != EntryTypeHLL {
			return store, wrongTypeValue()
		}
		merged.Merge(entry.Value.(*hyperloglog.HyperLogLog))
		if expiresAt == nil {
			expiresAt = entry.ExpiresAt
		}
	}
	return store.Set(dst, NewHLLEntry(merged, expiresAt)), okValue()
}

func cmdExpire(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 2 {
		return store, errValue("ERR wrong number of arguments for 'EXPIRE'")
	}
	seconds, err := parseInt64(args[1])
	if err != nil {
		return store, errValue(err.Error())
	}
	return setExpiry(store, args[0], expirationFromSeconds(seconds))
}

func cmdExpireAt(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 2 {
		return store, errValue("ERR wrong number of arguments for 'EXPIREAT'")
	}
	ts, err := parseInt64(args[1])
	if err != nil {
		return store, errValue(err.Error())
	}
	return setExpiry(store, args[0], expirationFromUnixSeconds(ts))
}

func cmdTTL(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'TTL'")
	}
	return store, resp.Integer(ttlSeconds(store, args[0]))
}

func cmdPTTL(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'PTTL'")
	}
	return store, resp.Integer(ttlMillis(store, args[0]))
}

func cmdPersist(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'PERSIST'")
	}
	entry, ok := store.Get(args[0])
	if !ok || entry.ExpiresAt == nil {
		return store, resp.Integer(0)
	}
	entry.ExpiresAt = nil
	return store.Set(args[0], entry), resp.Integer(1)
}

func cmdSelect(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'SELECT'")
	}
	dbIndex, err := parseInt64(args[0])
	if err != nil {
		return store, errValue(err.Error())
	}
	return store.Select(int(dbIndex)), okValue()
}

func cmdFlushDB(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 0 {
		return store, errValue("ERR wrong number of arguments for 'FLUSHDB'")
	}
	return store.FlushDB(), okValue()
}

func cmdFlushAll(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 0 {
		return store, errValue("ERR wrong number of arguments for 'FLUSHALL'")
	}
	return store.FlushAll(), okValue()
}

func cmdDBSize(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 0 {
		return store, errValue("ERR wrong number of arguments for 'DBSIZE'")
	}
	return store, resp.Integer(int64(store.DBSize()))
}

func cmdInfo(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) > 1 {
		return store, errValue("ERR wrong number of arguments for 'INFO'")
	}
	info := fmt.Sprintf(
		"# Keyspace\nactive_db:%d\nkeys:%d\n",
		store.ActiveDB,
		store.DBSize(),
	)
	return store, resp.BulkString([]byte(info))
}

func cmdKeys(store *Store, args [][]byte) (*Store, resp.Value) {
	if len(args) != 1 {
		return store, errValue("ERR wrong number of arguments for 'KEYS'")
	}
	keys := store.Keys(args[0])
	values := make([]resp.Value, 0, len(keys))
	for _, key := range keys {
		values = append(values, resp.BulkString(key))
	}
	return store, resp.Array(values)
}

func okValue() resp.Value {
	return resp.SimpleString("OK")
}

func errValue(message string) resp.Value {
	return resp.ErrorValue(message)
}

func wrongTypeValue() resp.Value {
	return resp.ErrorValue("WRONGTYPE Operation against a key holding the wrong kind of value")
}

func parseInt64(data []byte) (int64, error) {
	value, err := strconv.ParseInt(string(data), 10, 64)
	if err != nil {
		return 0, fmt.Errorf("ERR value is not an integer or out of range")
	}
	return value, nil
}

func parseFloat64(data []byte) (float64, error) {
	value, err := strconv.ParseFloat(string(data), 64)
	if err != nil {
		return 0, fmt.Errorf("ERR value is not a valid float")
	}
	return value, nil
}

func boolToInt64(value bool) int64 {
	if value {
		return 1
	}
	return 0
}

func currentExpiry(store *Store, key []byte) *uint64 {
	if entry, ok := store.Get(key); ok {
		return entry.ExpiresAt
	}
	return nil
}

func setExpiry(store *Store, key []byte, expiresAt *uint64) (*Store, resp.Value) {
	entry, ok := store.Get(key)
	if !ok {
		return store, resp.Integer(0)
	}
	entry.ExpiresAt = expiresAt
	return store.Set(key, entry), resp.Integer(1)
}

func expirationFromSeconds(seconds int64) *uint64 {
	return expirationFromMillis(seconds * 1000)
}

func expirationFromUnixSeconds(seconds int64) *uint64 {
	return expirationFromMillis(seconds * 1000)
}

func expirationFromMillis(millis int64) *uint64 {
	next := int64(currentTimeMs()) + millis
	if next < 0 {
		next = 0
	}
	value := uint64(next)
	return &value
}

func ttlMillis(store *Store, key []byte) int64 {
	entry, ok := store.Get(key)
	if !ok {
		return -2
	}
	if entry.ExpiresAt == nil {
		return -1
	}
	delta := int64(*entry.ExpiresAt) - int64(currentTimeMs())
	if delta < 0 {
		return -2
	}
	return delta
}

func ttlSeconds(store *Store, key []byte) int64 {
	millis := ttlMillis(store, key)
	if millis < 0 {
		return millis
	}
	return millis / 1000
}

func adjustInteger(store *Store, key []byte, delta int64) (*Store, resp.Value) {
	entry, ok := store.Get(key)
	var current int64
	var expiresAt *uint64
	if ok {
		if entry.Type != EntryTypeString {
			return store, wrongTypeValue()
		}
		value, err := strconv.ParseInt(string(entry.Value.([]byte)), 10, 64)
		if err != nil {
			return store, errValue("ERR value is not an integer or out of range")
		}
		current = value
		expiresAt = entry.ExpiresAt
	}
	current += delta
	return store.Set(key, NewStringEntry([]byte(strconv.FormatInt(current, 10)), expiresAt)), resp.Integer(current)
}

func getOrCreateHash(store *Store, key []byte, expiresAt *uint64) (*hashmap.HashMap[[]byte], Entry, bool) {
	entry, ok := store.Get(key)
	if !ok {
		return hashmap.New[[]byte](), Entry{}, false
	}
	if entry.Type != EntryTypeHash {
		panic("WRONGTYPE")
	}
	return entry.Value.(*hashmap.HashMap[[]byte]).Clone(), entry, true
}

func getOrCreateSet(store *Store, key []byte, expiresAt *uint64) (*hashset.HashSet, Entry, bool) {
	entry, ok := store.Get(key)
	if !ok {
		return hashset.New(), Entry{}, false
	}
	if entry.Type != EntryTypeSet {
		panic("WRONGTYPE")
	}
	return entry.Value.(*hashset.HashSet).Clone(), entry, true
}

func getOrCreateHLL(store *Store, key []byte, expiresAt *uint64) (*hyperloglog.HyperLogLog, Entry, bool) {
	entry, ok := store.Get(key)
	if !ok {
		return hyperloglog.New(), Entry{}, false
	}
	if entry.Type != EntryTypeHLL {
		panic("WRONGTYPE")
	}
	return entry.Value.(*hyperloglog.HyperLogLog).Clone(), entry, true
}
