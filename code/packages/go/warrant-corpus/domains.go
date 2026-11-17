package warrantcorpus

import (
	"regexp"
	"strconv"
)

// # The finite input domains of VER04i section 2.6, enumerated in this lane
//
// The corpus does not ship its exhaustive inputs (that would be ~28 MB of text
// per library). It ships a DESCRIPTOR (`{"domain": "bytes_d1"}`) and each lane
// re-creates the values itself. The descriptors are defined once, by the spec,
// and every lane must produce exactly the same values in exactly the same
// order; the `inputs_sha256` digest in the corpus is what proves it did.
//
//	descriptor    values, in order
//	bool          false, true
//	u8_all        0 .. 255
//	u64_edge      0..255; 1<<i for i = 8..63; MAX; then MAX ^ (1<<i) for i = 0..63
//	              (377 values)
//	nat_0_64      0 .. 64
//	bytes_d1      ""; every single byte; every length-2 and length-3 string over the
//	              alphabet A = {00, 01, 80, ff} in lexicographic order; for L = 4..8
//	              the strings 00 x L and a5 x L (347 values)
//	bytes_n(N)    for N <= 3 the bytes_d1 members of length N; otherwise the four
//	              uniform fills over A, then for every position and each of
//	              01, 80, ff the zero string with that one byte set
//
// Integers are uint64: u64_edge holds values far above 2^53, and a native
// unsigned 64-bit type holds all of them exactly (MAX and MAX ^ 1 stay distinct).
//
// A multi-binder claim enumerates the Cartesian product with the LAST binder
// varying fastest (row-major); see product.
var (
	alphabet = []byte{0x00, 0x01, 0x80, 0xff}
	bytesNRE = regexp.MustCompile(`^bytes_n\(([0-9]+)\)$`)
)

// alphabetStrings is every string of `length` symbols over the alphabet, in
// lexicographic order.
func alphabetStrings(length int) [][]byte {
	strs := [][]byte{{}}
	for position := 0; position < length; position++ {
		next := make([][]byte, 0, len(strs)*len(alphabet))
		for _, prefix := range strs {
			for _, symbol := range alphabet {
				extended := make([]byte, len(prefix)+1)
				copy(extended, prefix)
				extended[len(prefix)] = symbol
				next = append(next, extended)
			}
		}
		strs = next
	}
	return strs
}

func filled(length int, fill byte) []byte {
	out := make([]byte, length)
	for i := range out {
		out[i] = fill
	}
	return out
}

func bytesD1() [][]byte {
	values := [][]byte{{}}
	for b := 0; b < 256; b++ {
		values = append(values, []byte{byte(b)})
	}
	values = append(values, alphabetStrings(2)...)
	values = append(values, alphabetStrings(3)...)
	for length := 4; length <= 8; length++ {
		for _, fill := range []byte{0x00, 0xa5} {
			values = append(values, filled(length, fill))
		}
	}
	return values
}

func bytesN(width int) [][]byte {
	need(width >= 0 && width <= maxWidth, "descriptor", "Bytes width is out of bounds")
	if width <= 3 {
		var values [][]byte
		for _, value := range bytesD1() {
			if len(value) == width {
				values = append(values, value)
			}
		}
		return values
	}
	var values [][]byte
	for _, fill := range alphabet {
		values = append(values, filled(width, fill))
	}
	for position := 0; position < width; position++ {
		for _, symbol := range []byte{0x01, 0x80, 0xff} {
			buffer := make([]byte, width)
			buffer[position] = symbol
			values = append(values, buffer)
		}
	}
	return values
}

func u64Edge() []uint64 {
	values := make([]uint64, 0, 377)
	for value := uint64(0); value < 256; value++ {
		values = append(values, value)
	}
	for shift := uint(8); shift < 64; shift++ {
		values = append(values, uint64(1)<<shift)
	}
	values = append(values, u64Max)
	for shift := uint(0); shift < 64; shift++ {
		values = append(values, u64Max^(uint64(1)<<shift))
	}
	return values
}

func rangeValues(count int) []uint64 {
	values := make([]uint64, count)
	for i := range values {
		values[i] = uint64(i)
	}
	return values
}

func asAny[T any](items []T) []any {
	out := make([]any, len(items))
	for i, item := range items {
		out[i] = item
	}
	return out
}

// Domain is the values of a descriptor and the Warrant type text they have.
type Domain struct {
	Values []any
	Type   string
}

// domainValues enumerates the descriptor name.
func domainValues(name string) Domain {
	switch name {
	case "bool":
		return Domain{[]any{false, true}, "Bool"}
	case "u8_all":
		return Domain{asAny(rangeValues(256)), "U8"}
	case "u64_edge":
		return Domain{asAny(u64Edge()), "U64"}
	case "nat_0_64":
		return Domain{asAny(rangeValues(65)), "Nat"}
	case "bytes_d1":
		return Domain{asAny(bytesD1()), "Bytes"}
	}
	if match := bytesNRE.FindStringSubmatch(name); match != nil {
		width, err := strconv.Atoi(match[1])
		need(err == nil, "descriptor", "Bytes width is out of bounds")
		return Domain{asAny(bytesN(width)), "Bytes<" + strconv.Itoa(width) + ">"}
	}
	fail("unsupported-domain", "unknown domain descriptor")
	return Domain{}
}

// domainShape says how many values a descriptor has and which type they are,
// WITHOUT building them.
func domainShape(name string) (size int, typeText string) {
	switch name {
	case "bool":
		return 2, "Bool"
	case "u8_all":
		return 256, "U8"
	case "u64_edge":
		return 256 + 56 + 1 + 64, "U64"
	case "nat_0_64":
		return 65, "Nat"
	case "bytes_d1":
		return 1 + 256 + 16 + 64 + 10, "Bytes"
	}
	if match := bytesNRE.FindStringSubmatch(name); match != nil {
		width, err := strconv.Atoi(match[1])
		need(err == nil && width >= 0 && width <= maxWidth, "descriptor", "Bytes width is out of bounds")
		// Widths 0..3 are the bytes_d1 members of that length: 1, 256, 4^2, 4^3.
		small := []int{1, 256, 16, 64}
		if width < len(small) {
			size = small[width]
		} else {
			size = 4 + 3*width
		}
		return size, "Bytes<" + strconv.Itoa(width) + ">"
	}
	fail("unsupported-domain", "unknown domain descriptor")
	return 0, ""
}

// binderShape validates one binder's {"domain": ..., "type": ...} descriptor and
// returns the domain's name, size and type without enumerating it.
//
// The declared type must agree with the type the descriptor produces, so a corpus
// cannot label byte strings as integers and have the encoders quietly disagree
// with the generator.
func binderShape(descriptor any) (name string, size int, typeText string) {
	object, ok := descriptor.(map[string]any)
	need(ok, "descriptor", "malformed binder domain")
	for key := range object {
		need(key == "domain" || key == "type", "descriptor", "malformed binder domain")
	}
	name, ok = object["domain"].(string)
	need(ok, "descriptor", "malformed binder domain")
	size, typeText = domainShape(name)
	declared, hasType := object["type"]
	need(!hasType || declared == typeText, "descriptor", "binder type disagrees with its domain")
	return name, size, typeText
}

// product visits the Cartesian product of domains with the last binder fastest.
//
// An odometer, not a recursion: a million tuples must not cost a million
// closures. visit returns false to stop early. The slice handed to visit is
// reused, so a caller that keeps it must copy it.
func product(domains [][]any, visit func(values []any) bool) {
	for _, domain := range domains {
		if len(domain) == 0 {
			return
		}
	}
	counters := make([]int, len(domains))
	current := make([]any, len(domains))
	for i, domain := range domains {
		current[i] = domain[0]
	}
	for {
		if !visit(current) {
			return
		}
		position := len(domains) - 1
		for position >= 0 {
			domain := domains[position]
			counters[position]++
			if counters[position] < len(domain) {
				current[position] = domain[counters[position]]
				break
			}
			counters[position] = 0
			current[position] = domain[0]
			position--
		}
		if position < 0 {
			return
		}
	}
}
