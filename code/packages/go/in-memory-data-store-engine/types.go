package datastoreengine

import (
	"bytes"
	"fmt"
	"math"
	"sort"
	"time"

	"github.com/adhithyan15/coding-adventures/code/packages/go/hash-map"
	"github.com/adhithyan15/coding-adventures/code/packages/go/hash-set"
	"github.com/adhithyan15/coding-adventures/code/packages/go/heap"
	"github.com/adhithyan15/coding-adventures/code/packages/go/hyperloglog"
	"github.com/adhithyan15/coding-adventures/code/packages/go/skip-list"
)

type EntryType string

const (
	EntryTypeString EntryType = "string"
	EntryTypeHash   EntryType = "hash"
	EntryTypeList   EntryType = "list"
	EntryTypeSet    EntryType = "set"
	EntryTypeZSet   EntryType = "zset"
	EntryTypeHLL    EntryType = "hll"
)

func (t EntryType) String() string {
	return string(t)
}

type SortedEntry struct {
	Score  float64
	Member []byte
}

type SortedSet struct {
	members  *hashmap.HashMap[float64]
	ordering *skiplist.SkipList[SortedEntry]
}

func NewSortedSet() *SortedSet {
	return &SortedSet{
		members: hashmap.New[float64](),
		ordering: skiplist.New(func(a, b SortedEntry) bool {
			if a.Score != b.Score {
				return a.Score < b.Score
			}
			return bytes.Compare(a.Member, b.Member) < 0
		}),
	}
}

func (s *SortedSet) Clone() *SortedSet {
	if s == nil {
		return NewSortedSet()
	}
	clone := NewSortedSet()
	for _, entry := range s.OrderedEntries() {
		clone.Insert(entry.Score, entry.Member)
	}
	return clone
}

func (s *SortedSet) Len() int {
	if s == nil {
		return 0
	}
	return s.members.Size()
}

func (s *SortedSet) IsEmpty() bool {
	return s.Len() == 0
}

func (s *SortedSet) Contains(member []byte) bool {
	return s != nil && s.members.Has(member)
}

func (s *SortedSet) Score(member []byte) (float64, bool) {
	if s == nil {
		return 0, false
	}
	return s.members.Get(member)
}

func (s *SortedSet) Insert(score float64, member []byte) bool {
	if math.IsNaN(score) {
		panic("sorted set score cannot be NaN")
	}
	if s == nil {
		return false
	}
	isNew := !s.members.Has(member)
	if oldScore, ok := s.members.Get(member); ok {
		_ = s.ordering.Delete(SortedEntry{Score: oldScore, Member: cloneBytes(member)})
	}
	s.members.Set(member, score)
	s.ordering.Insert(SortedEntry{Score: score, Member: cloneBytes(member)})
	return isNew
}

func (s *SortedSet) Remove(member []byte) bool {
	if s == nil {
		return false
	}
	oldScore, ok := s.members.Get(member)
	if !ok {
		return false
	}
	_ = s.ordering.Delete(SortedEntry{Score: oldScore, Member: cloneBytes(member)})
	return s.members.Delete(member)
}

func (s *SortedSet) Rank(member []byte) (int, bool) {
	if s == nil {
		return 0, false
	}
	target := cloneBytes(member)
	for i, entry := range s.ordering.ToSlice() {
		if bytes.Equal(entry.Member, target) {
			return i, true
		}
	}
	return 0, false
}

func (s *SortedSet) OrderedEntries() []SortedEntry {
	if s == nil {
		return nil
	}
	return cloneSortedEntries(s.ordering.ToSlice())
}

func (s *SortedSet) RangeByIndex(start, end int) []SortedEntry {
	entries := s.OrderedEntries()
	if len(entries) == 0 {
		return nil
	}
	n := len(entries)
	if start < 0 {
		start = n + start
	}
	if end < 0 {
		end = n + end
	}
	if start < 0 || end < 0 || start >= n || start > end || end >= n {
		return nil
	}
	return cloneSortedEntries(entries[start : end+1])
}

func (s *SortedSet) RangeByScore(min, max float64) []SortedEntry {
	if math.IsNaN(min) || math.IsNaN(max) {
		panic("sorted set score cannot be NaN")
	}
	result := make([]SortedEntry, 0)
	for _, entry := range s.OrderedEntries() {
		if entry.Score >= min && entry.Score <= max {
			result = append(result, entry)
		}
	}
	return result
}

func (s *SortedSet) Equal(other *SortedSet) bool {
	left := s.OrderedEntries()
	right := other.OrderedEntries()
	if len(left) != len(right) {
		return false
	}
	for i := range left {
		if left[i].Score != right[i].Score || !bytes.Equal(left[i].Member, right[i].Member) {
			return false
		}
	}
	return true
}

type Entry struct {
	Type      EntryType
	Value     any
	ExpiresAt *uint64
}

func NewStringEntry(value []byte, expiresAt *uint64) Entry {
	return Entry{Type: EntryTypeString, Value: cloneBytes(value), ExpiresAt: cloneExpiry(expiresAt)}
}

func NewHashEntry(value *hashmap.HashMap[[]byte], expiresAt *uint64) Entry {
	return Entry{Type: EntryTypeHash, Value: value.Clone(), ExpiresAt: cloneExpiry(expiresAt)}
}

func NewListEntry(value [][]byte, expiresAt *uint64) Entry {
	return Entry{Type: EntryTypeList, Value: cloneByteSlices(value), ExpiresAt: cloneExpiry(expiresAt)}
}

func NewSetEntry(value *hashset.HashSet, expiresAt *uint64) Entry {
	return Entry{Type: EntryTypeSet, Value: value.Clone(), ExpiresAt: cloneExpiry(expiresAt)}
}

func NewZSetEntry(value *SortedSet, expiresAt *uint64) Entry {
	return Entry{Type: EntryTypeZSet, Value: value.Clone(), ExpiresAt: cloneExpiry(expiresAt)}
}

func NewHLLEntry(value *hyperloglog.HyperLogLog, expiresAt *uint64) Entry {
	return Entry{Type: EntryTypeHLL, Value: value.Clone(), ExpiresAt: cloneExpiry(expiresAt)}
}

func (e Entry) Clone() Entry {
	switch e.Type {
	case EntryTypeString:
		return NewStringEntry(e.Value.([]byte), e.ExpiresAt)
	case EntryTypeHash:
		return NewHashEntry(e.Value.(*hashmap.HashMap[[]byte]), e.ExpiresAt)
	case EntryTypeList:
		return NewListEntry(e.Value.([][]byte), e.ExpiresAt)
	case EntryTypeSet:
		return NewSetEntry(e.Value.(*hashset.HashSet), e.ExpiresAt)
	case EntryTypeZSet:
		return NewZSetEntry(e.Value.(*SortedSet), e.ExpiresAt)
	case EntryTypeHLL:
		return NewHLLEntry(e.Value.(*hyperloglog.HyperLogLog), e.ExpiresAt)
	default:
		return e
	}
}

func (e Entry) String() string {
	return fmt.Sprintf("Entry{%s}", e.Type)
}

func cloneExpiry(expiresAt *uint64) *uint64 {
	if expiresAt == nil {
		return nil
	}
	value := *expiresAt
	return &value
}

func cloneByteSlices(values [][]byte) [][]byte {
	if values == nil {
		return nil
	}
	result := make([][]byte, len(values))
	for i, value := range values {
		result[i] = cloneBytes(value)
	}
	return result
}

func cloneSortedEntries(values []SortedEntry) []SortedEntry {
	if values == nil {
		return nil
	}
	result := make([]SortedEntry, len(values))
	for i, value := range values {
		result[i] = SortedEntry{Score: value.Score, Member: cloneBytes(value.Member)}
	}
	return result
}

func cloneBytes(value []byte) []byte {
	if value == nil {
		return nil
	}
	return append([]byte(nil), value...)
}

type ttlEntry struct {
	ExpiresAt uint64
	Key       []byte
}

func ttlLess(left, right ttlEntry) bool {
	if left.ExpiresAt != right.ExpiresAt {
		return left.ExpiresAt < right.ExpiresAt
	}
	return bytes.Compare(left.Key, right.Key) < 0
}

const defaultDBCount = 16

type Database struct {
	entries *hashmap.HashMap[Entry]
	ttlHeap *heap.MinHeap[ttlEntry]
}

func NewDatabase() Database {
	return Database{
		entries: hashmap.New[Entry](),
		ttlHeap: heap.NewMinHeap(ttlLess),
	}
}

func (db Database) Clone() Database {
	clone := NewDatabase()
	for _, entry := range db.entries.Entries() {
		clone.entries.Set(entry.Key, entry.Value.Clone())
	}
	clone.ttlHeap = db.ttlHeap.Clone()
	return clone
}

func (db *Database) Get(key []byte) (Entry, bool) {
	if db == nil || db.entries == nil {
		return Entry{}, false
	}
	entry, ok := db.entries.Get(key)
	if !ok {
		return Entry{}, false
	}
	if entry.ExpiresAt != nil && currentTimeMs() >= *entry.ExpiresAt {
		return Entry{}, false
	}
	return entry.Clone(), true
}

func (db *Database) Set(key []byte, entry Entry) {
	if db.entries == nil {
		db.entries = hashmap.New[Entry]()
	}
	if db.ttlHeap == nil {
		db.ttlHeap = heap.NewMinHeap(ttlLess)
	}
	if entry.ExpiresAt != nil {
		db.ttlHeap.Push(ttlEntry{ExpiresAt: *entry.ExpiresAt, Key: cloneBytes(key)})
	}
	db.entries.Set(key, entry.Clone())
}

func (db *Database) Delete(key []byte) bool {
	if db.entries == nil {
		return false
	}
	return db.entries.Delete(key)
}

func (db *Database) Exists(key []byte) bool {
	_, ok := db.Get(key)
	return ok
}

func (db *Database) TypeOf(key []byte) (EntryType, bool) {
	entry, ok := db.Get(key)
	if !ok {
		return "", false
	}
	return entry.Type, true
}

func (db *Database) Keys(pattern []byte) [][]byte {
	if db.entries == nil {
		return nil
	}
	keys := make([][]byte, 0)
	for _, key := range db.entries.Keys() {
		if globMatch(pattern, key) {
			keys = append(keys, key)
		}
	}
	sort.Slice(keys, func(i, j int) bool {
		return bytes.Compare(keys[i], keys[j]) < 0
	})
	return keys
}

func (db *Database) DBSize() int {
	if db.entries == nil {
		return 0
	}
	count := 0
	for _, key := range db.entries.Keys() {
		if db.Exists(key) {
			count++
		}
	}
	return count
}

func (db *Database) ExpireLazy(key []byte) {
	if db.entries == nil {
		return
	}
	entry, ok := db.entries.Get(key)
	if !ok || entry.ExpiresAt == nil {
		return
	}
	if currentTimeMs() >= *entry.ExpiresAt {
		db.entries.Delete(key)
	}
}

func (db *Database) ActiveExpire() {
	if db.ttlHeap == nil {
		return
	}
	now := currentTimeMs()
	for {
		entry, ok := db.ttlHeap.Peek()
		if !ok || entry.ExpiresAt > now {
			return
		}
		popped, _ := db.ttlHeap.Pop()
		current, ok := db.entries.Get(popped.Key)
		if ok && current.ExpiresAt != nil && *current.ExpiresAt == popped.ExpiresAt {
			db.entries.Delete(popped.Key)
		}
	}
}

func (db *Database) Clear() {
	db.entries.Clear()
	db.ttlHeap.Clear()
}

type Store struct {
	Databases []Database
	ActiveDB  int
}

func NewStore() *Store {
	databases := make([]Database, defaultDBCount)
	for i := range databases {
		databases[i] = NewDatabase()
	}
	return &Store{Databases: databases}
}

func (s *Store) Clone() *Store {
	if s == nil {
		return NewStore()
	}
	clone := &Store{
		Databases: make([]Database, len(s.Databases)),
		ActiveDB:  s.ActiveDB,
	}
	for i := range s.Databases {
		clone.Databases[i] = s.Databases[i].Clone()
	}
	return clone
}

func (s *Store) WithActiveDB(activeDB int) *Store {
	if s == nil {
		return NewStore().WithActiveDB(activeDB)
	}
	clone := s.Clone()
	clone.ActiveDB = clamp(activeDB, 0, len(clone.Databases)-1)
	return clone
}

func (s *Store) Select(activeDB int) *Store {
	return s.WithActiveDB(activeDB)
}

func (s *Store) CurrentDB() *Database {
	return &s.Databases[s.ActiveDB]
}

func (s *Store) CurrentDBValue() Database {
	return s.Databases[s.ActiveDB]
}

func (s *Store) Get(key []byte) (Entry, bool) {
	return s.CurrentDB().Get(key)
}

func (s *Store) Set(key []byte, entry Entry) *Store {
	clone := s.Clone()
	clone.CurrentDB().Set(key, entry)
	return clone
}

func (s *Store) Delete(key []byte) *Store {
	clone := s.Clone()
	clone.CurrentDB().Delete(key)
	return clone
}

func (s *Store) Exists(key []byte) bool {
	return s.CurrentDB().Exists(key)
}

func (s *Store) Keys(pattern []byte) [][]byte {
	return s.CurrentDB().Keys(pattern)
}

func (s *Store) TypeOf(key []byte) (EntryType, bool) {
	return s.CurrentDB().TypeOf(key)
}

func (s *Store) DBSize() int {
	return s.CurrentDB().DBSize()
}

func (s *Store) ExpireLazy(key []byte) *Store {
	clone := s.Clone()
	clone.CurrentDB().ExpireLazy(key)
	return clone
}

func (s *Store) ActiveExpire() *Store {
	clone := s.Clone()
	clone.CurrentDB().ActiveExpire()
	return clone
}

func (s *Store) ActiveExpireAll() *Store {
	clone := s.Clone()
	for i := range clone.Databases {
		clone.Databases[i].ActiveExpire()
	}
	return clone
}

func (s *Store) FlushDB() *Store {
	clone := s.Clone()
	clone.Databases[clone.ActiveDB] = NewDatabase()
	return clone
}

func (s *Store) FlushAll() *Store {
	clone := s.Clone()
	for i := range clone.Databases {
		clone.Databases[i] = NewDatabase()
	}
	return clone
}

func (s *Store) Clear() *Store {
	clone := s.Clone()
	for i := range clone.Databases {
		clone.Databases[i].Clear()
	}
	clone.ActiveDB = 0
	return clone
}

func currentTimeMs() uint64 {
	return uint64(time.Now().UnixMilli())
}

func clamp(value, minValue, maxValue int) int {
	if value < minValue {
		return minValue
	}
	if value > maxValue {
		return maxValue
	}
	return value
}

func globMatch(pattern, text []byte) bool {
	return globMatchInner(pattern, text)
}

func globMatchInner(pattern, text []byte) bool {
	if len(pattern) == 0 {
		return len(text) == 0
	}
	switch pattern[0] {
	case '*':
		return globMatchInner(pattern[1:], text) || (len(text) > 0 && globMatchInner(pattern, text[1:]))
	case '?':
		return len(text) > 0 && globMatchInner(pattern[1:], text[1:])
	case '[':
		if end := bytes.IndexByte(pattern, ']'); end > 0 {
			if len(text) == 0 {
				return false
			}
			class := pattern[1:end]
			if classContains(class, text[0]) {
				return globMatchInner(pattern[end+1:], text[1:])
			}
			return false
		}
		fallthrough
	default:
		return len(text) > 0 && pattern[0] == text[0] && globMatchInner(pattern[1:], text[1:])
	}
}

func classContains(class []byte, value byte) bool {
	for i := 0; i < len(class); i++ {
		if i+2 < len(class) && class[i+1] == '-' {
			if class[i] <= value && value <= class[i+2] {
				return true
			}
			i += 2
			continue
		}
		if class[i] == value {
			return true
		}
	}
	return false
}
