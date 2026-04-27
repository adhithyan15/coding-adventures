package garbagecollector

import (
	"fmt"
	"reflect"
)

const InitialAddress = 0x10000

type HeapObject interface {
	References() []int
	IsMarked() bool
	SetMarked(marked bool)
	TypeName() string
}

type BaseObject struct {
	marked bool
}

func (b *BaseObject) References() []int {
	return nil
}

func (b *BaseObject) IsMarked() bool {
	return b.marked
}

func (b *BaseObject) SetMarked(marked bool) {
	b.marked = marked
}

type ConsCell struct {
	BaseObject
	Car any
	Cdr any
}

func NewConsCell(car, cdr any) *ConsCell {
	return &ConsCell{Car: car, Cdr: cdr}
}

func (c *ConsCell) References() []int {
	refs := make([]int, 0, 2)
	if address, ok := intAddress(c.Car); ok {
		refs = append(refs, address)
	}
	if address, ok := intAddress(c.Cdr); ok {
		refs = append(refs, address)
	}
	return refs
}

func (c *ConsCell) TypeName() string {
	return "ConsCell"
}

type Symbol struct {
	BaseObject
	Name string
}

func NewSymbol(name string) *Symbol {
	return &Symbol{Name: name}
}

func (s *Symbol) TypeName() string {
	return "Symbol"
}

type LispClosure struct {
	BaseObject
	Code   any
	Env    map[string]any
	Params []string
}

func NewLispClosure(code any, env map[string]any, params []string) *LispClosure {
	if env == nil {
		env = map[string]any{}
	}
	return &LispClosure{Code: code, Env: env, Params: params}
}

func (l *LispClosure) References() []int {
	refs := make([]int, 0, len(l.Env))
	for _, value := range l.Env {
		if address, ok := intAddress(value); ok {
			refs = append(refs, address)
		}
	}
	return refs
}

func (l *LispClosure) TypeName() string {
	return "LispClosure"
}

type GarbageCollector interface {
	Allocate(obj HeapObject) int
	Deref(address int) (HeapObject, error)
	Collect(roots []any) int
	HeapSize() int
	Stats() GCStats
	IsValidAddress(address int) bool
}

type GCStats struct {
	TotalAllocations int
	TotalCollections int
	TotalFreed       int
	HeapSize         int
}

type MarkAndSweepGC struct {
	heap             map[int]HeapObject
	nextAddress      int
	totalAllocations int
	totalCollections int
	totalFreed       int
}

func NewMarkAndSweepGC() *MarkAndSweepGC {
	return &MarkAndSweepGC{
		heap:        map[int]HeapObject{},
		nextAddress: InitialAddress,
	}
}

func (g *MarkAndSweepGC) Allocate(obj HeapObject) int {
	address := g.nextAddress
	g.nextAddress++
	g.heap[address] = obj
	g.totalAllocations++
	return address
}

func (g *MarkAndSweepGC) Deref(address int) (HeapObject, error) {
	obj, ok := g.heap[address]
	if !ok {
		return nil, fmt.Errorf("invalid heap address: %d", address)
	}
	return obj, nil
}

func (g *MarkAndSweepGC) Collect(roots []any) int {
	g.totalCollections++

	for _, root := range roots {
		g.markValue(root)
	}

	toDelete := make([]int, 0)
	for address, obj := range g.heap {
		if obj.IsMarked() {
			obj.SetMarked(false)
		} else {
			toDelete = append(toDelete, address)
		}
	}

	for _, address := range toDelete {
		delete(g.heap, address)
	}

	g.totalFreed += len(toDelete)
	return len(toDelete)
}

func (g *MarkAndSweepGC) HeapSize() int {
	return len(g.heap)
}

func (g *MarkAndSweepGC) Stats() GCStats {
	return GCStats{
		TotalAllocations: g.totalAllocations,
		TotalCollections: g.totalCollections,
		TotalFreed:       g.totalFreed,
		HeapSize:         g.HeapSize(),
	}
}

func (g *MarkAndSweepGC) IsValidAddress(address int) bool {
	_, ok := g.heap[address]
	return ok
}

func (g *MarkAndSweepGC) markValue(value any) {
	if address, ok := intAddress(value); ok {
		obj, live := g.heap[address]
		if live && !obj.IsMarked() {
			obj.SetMarked(true)
			for _, ref := range obj.References() {
				g.markValue(ref)
			}
		}
		return
	}

	switch typed := value.(type) {
	case []any:
		for _, item := range typed {
			g.markValue(item)
		}
	case map[string]any:
		for _, item := range typed {
			g.markValue(item)
		}
	default:
		g.markReflectSliceOrMap(value)
	}
}

func (g *MarkAndSweepGC) markReflectSliceOrMap(value any) {
	rv := reflect.ValueOf(value)
	if !rv.IsValid() {
		return
	}

	switch rv.Kind() {
	case reflect.Slice, reflect.Array:
		for i := 0; i < rv.Len(); i++ {
			g.markValue(rv.Index(i).Interface())
		}
	case reflect.Map:
		iter := rv.MapRange()
		for iter.Next() {
			g.markValue(iter.Value().Interface())
		}
	}
}

type SymbolTable struct {
	gc    GarbageCollector
	table map[string]int
}

func NewSymbolTable(gc GarbageCollector) *SymbolTable {
	return &SymbolTable{gc: gc, table: map[string]int{}}
}

func (s *SymbolTable) Intern(name string) int {
	if address, ok := s.table[name]; ok && s.gc.IsValidAddress(address) {
		return address
	}

	address := s.gc.Allocate(NewSymbol(name))
	s.table[name] = address
	return address
}

func (s *SymbolTable) Lookup(name string) (int, bool) {
	address, ok := s.table[name]
	if !ok || !s.gc.IsValidAddress(address) {
		return 0, false
	}
	return address, true
}

func (s *SymbolTable) AllSymbols() map[string]int {
	symbols := map[string]int{}
	for name, address := range s.table {
		if s.gc.IsValidAddress(address) {
			symbols[name] = address
		}
	}
	return symbols
}

func intAddress(value any) (int, bool) {
	switch typed := value.(type) {
	case int:
		return typed, true
	case int8:
		return int(typed), true
	case int16:
		return int(typed), true
	case int32:
		return int(typed), true
	case int64:
		return int(typed), true
	case uint:
		if uint64(typed) <= uint64(^uint(0)>>1) {
			return int(typed), true
		}
	case uint8:
		return int(typed), true
	case uint16:
		return int(typed), true
	case uint32:
		if uint64(typed) <= uint64(^uint(0)>>1) {
			return int(typed), true
		}
	case uint64:
		if typed <= uint64(^uint(0)>>1) {
			return int(typed), true
		}
	}
	return 0, false
}
