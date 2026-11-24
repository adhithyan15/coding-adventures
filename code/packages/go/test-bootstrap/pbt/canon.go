package pbt

import (
	"fmt"

	"github.com/adhithyan15/coding-adventures/code/packages/go/test-bootstrap/internal/corpusjson"
)

// Canon renders a witness as stable text, for failure messages and the CI
// digest.  It is total: it never panics, never recurses without bound and never
// reads the whole of a hostile value (nesting deeper than 16 and more than
// 10,000 nodes render as "...").  The implementation is shared with the fuzz
// lane and lives in internal/corpusjson.
//
//	Canon(true)               -> "true"
//	Canon(big.NewInt(-12))    -> "-12"
//	Canon([]byte{0, 255})     -> "0x00ff"
//	Canon([]int{1, 2})        -> "[1,2]"
func Canon(v any) string { return corpusjson.Canon(v) }

// safeString is fmt.Sprint for panic values and errors.  fmt itself recovers a
// panicking String or Error method and prints the panic, so this never panics;
// the helper exists to give that guarantee one name.
func safeString(v any) string { return fmt.Sprint(v) }
