package pbt

import (
	"encoding/hex"
	"fmt"
	"math/big"
	"reflect"
	"sort"
	"strconv"
	"strings"
)

const (
	// canonMaxDepth bounds container nesting in a rendered witness.
	canonMaxDepth = 16
	// canonMaxNodes bounds the total values rendered, so a wide or heavily
	// shared (DAG-shaped) witness cannot make rendering explode.
	canonMaxNodes = 10_000
)

// Canon renders a witness as stable text, for failure messages and the CI
// digest.
//
// It is total: it never panics, never recurses without bound and never reads
// the whole of a hostile value.  Nesting deeper than 16 renders as "...", a
// witness with more than 10,000 nodes truncates with "...", pointer cycles end
// at the depth cap, and a value whose String or Error method panics renders as
// its type name.  This matters because redacted mode computes the digest
// outside any recover: a custom generator can build arbitrarily deep values,
// and a runner that crashes on one never returns a verdict (the Python lane's
// security fix, ported).
//
//	Canon(true)               -> "true"
//	Canon(big.NewInt(-12))    -> "-12"
//	Canon([]byte{0, 255})     -> "0x00ff"
//	Canon([]int{1, 2})        -> "[1,2]"
func Canon(v any) string {
	var b strings.Builder
	nodes := 0
	canon(&b, reflect.ValueOf(v), 0, &nodes)
	return b.String()
}

func canon(b *strings.Builder, v reflect.Value, depth int, nodes *int) {
	*nodes++
	if *nodes > canonMaxNodes {
		b.WriteString("...")
		return
	}
	if !v.IsValid() {
		b.WriteString("nil")
		return
	}
	// *big.Int is the engine's integer: render it as plain digits.
	if v.CanInterface() {
		switch x := v.Interface().(type) {
		case *big.Int:
			if x == nil {
				b.WriteString("nil")
			} else {
				b.WriteString(x.String())
			}
			return
		case big.Int:
			b.WriteString(x.String())
			return
		}
	}
	switch v.Kind() {
	case reflect.Bool:
		b.WriteString(strconv.FormatBool(v.Bool()))
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		b.WriteString(strconv.FormatInt(v.Int(), 10))
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		b.WriteString(strconv.FormatUint(v.Uint(), 10))
	case reflect.Float32, reflect.Float64:
		b.WriteString(strconv.FormatFloat(v.Float(), 'g', -1, 64))
	case reflect.String:
		b.WriteString(strconv.Quote(v.String()))
	case reflect.Slice, reflect.Array:
		if v.Type().Elem().Kind() == reflect.Uint8 {
			raw := make([]byte, v.Len())
			for i := range raw {
				raw[i] = byte(v.Index(i).Uint())
			}
			b.WriteString("0x" + hex.EncodeToString(raw))
			return
		}
		if depth >= canonMaxDepth {
			b.WriteString("...")
			return
		}
		b.WriteByte('[')
		for i := 0; i < v.Len(); i++ {
			if i > 0 {
				b.WriteByte(',')
			}
			if *nodes >= canonMaxNodes { // out of budget: stop, do not walk the rest
				b.WriteString("...")
				break
			}
			canon(b, v.Index(i), depth+1, nodes)
		}
		b.WriteByte(']')
	case reflect.Map:
		if depth >= canonMaxDepth {
			b.WriteString("...")
			return
		}
		type entry struct{ key, val string }
		entries := make([]entry, 0, v.Len())
		iter := v.MapRange()
		for iter.Next() {
			var kb, vb strings.Builder
			canon(&kb, iter.Key(), depth+1, nodes)
			canon(&vb, iter.Value(), depth+1, nodes)
			entries = append(entries, entry{kb.String(), vb.String()})
			if *nodes >= canonMaxNodes {
				break
			}
		}
		sort.Slice(entries, func(i, j int) bool { return entries[i].key < entries[j].key })
		b.WriteByte('{')
		for i, e := range entries {
			if i > 0 {
				b.WriteByte(',')
			}
			b.WriteString(e.key + ":" + e.val)
		}
		b.WriteByte('}')
	case reflect.Struct:
		if depth >= canonMaxDepth {
			b.WriteString("...")
			return
		}
		b.WriteByte('{')
		for i := 0; i < v.NumField(); i++ {
			if i > 0 {
				b.WriteByte(',')
			}
			if *nodes >= canonMaxNodes {
				b.WriteString("...")
				break
			}
			canon(b, v.Field(i), depth+1, nodes)
		}
		b.WriteByte('}')
	case reflect.Pointer, reflect.Interface:
		if v.IsNil() {
			b.WriteString("nil")
			return
		}
		if depth >= canonMaxDepth {
			b.WriteString("...")
			return
		}
		canon(b, v.Elem(), depth+1, nodes)
	default: // chan, func, unsafe pointer: identity is not a stable witness
		b.WriteString("<" + v.Type().String() + ">")
	}
}

// safeString is fmt.Sprint for panic values and errors.  fmt itself recovers a
// panicking String or Error method and prints the panic, so this never panics;
// the helper exists to give that guarantee one name.
func safeString(v any) string { return fmt.Sprint(v) }
