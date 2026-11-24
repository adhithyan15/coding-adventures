package tfv

import (
	"errors"
	"strings"
	"testing"
)

func TestAccepts(t *testing.T) {
	good := map[string]string{
		"header only":            "tfv 1",
		"header only newline":    "tfv 1\n",
		"comments and blanks":    "tfv 1\n\n# a note\r with CR\nr R0\n",
		"scalars":                "tfv 1\nr R5 Ka N Kb T Kc F Kd I-12 Ke I0\n",
		"float":                  "tfv 1\nr R1 Kf D3ff0000000000000\n",
		"bytes and text":         "tfv 1\nr R3 Ka X Kb S Kc S706274\n",
		"nested":                 "tfv 1\nr R1 Kv L2 L0 M1 I1 L1 N\n",
		"same field two records": "tfv 1\na R1 Kx N\nb R1 Kx N\n",
		"id charset":             "tfv 1\nAz09._/-x R0\n",
		"nul and noncharacter":   "tfv 1\nr R2 Ka S00 Kb Sefbfbf\n",
		"bom inside text":        "tfv 1\nr R1 Ka Sefbbbf\n",
	}
	for name, doc := range good {
		if err := Validate(doc); err != nil {
			t.Errorf("%s: unexpected rejection: %v", name, err)
		}
	}
}

func deep(n int) string {
	return strings.Repeat("L1 ", n) + "N"
}

func TestRejects(t *testing.T) {
	bad := map[string]string{
		"empty":               "",
		"lone newline":        "\n",
		"bad header":          "bad header\n",
		"bom header":          "\xef\xbb\xbftfv 1\n",
		"crlf header":         "tfv 1\r\n",
		"uppercase hex":       "tfv 1\nr R1 Kop S706274 Kdata XFF\n",
		"odd hex":             "tfv 1\nr R1 Ka X0\n",
		"minus zero":          "tfv 1\nr R1 Ka I-0\n",
		"leading zero":        "tfv 1\nr R1 Ka I007\n",
		"empty int":           "tfv 1\nr R1 Ka I\n",
		"int sign":            "tfv 1\nr R1 Ka I+1\n",
		"int underscore":      "tfv 1\nr R1 Ka I1_0\n",
		"arabic digit":        "tfv 1\nr R1 Ka I١\n",
		"too many digits":     "tfv 1\nr R1 Ka I1" + strings.Repeat("0", 1000) + "\n",
		"count too big":       "tfv 1\nr R2 Ka N\n",
		"count huge":          "tfv 1\nr R99999999999 Ka N\n",
		"count leading zero":  "tfv 1\nr R01 Ka N\n",
		"count negative":      "tfv 1\nr R-1\n",
		"count empty":         "tfv 1\nr R\n",
		"trailing tokens":     "tfv 1\nr R1 Ka N N\n",
		"double space":        "tfv 1\nr  R1 Ka N\n",
		"trailing space":      "tfv 1\nr R0 \n",
		"leading space":       "tfv 1\n r R0\n",
		"tab":                 "tfv 1\nr\tR0\n",
		"nbsp":                "tfv 1\nr R1 Ka N\n",
		"duplicate field":     "tfv 1\nr R2 Ka N Ka T\n",
		"duplicate id":        "tfv 1\nr R0\n# c\nr R0\n",
		"bad id":              "tfv 1\nr! R0\n",
		"missing R":           "tfv 1\nr K1\n",
		"id only":             "tfv 1\nr\n",
		"bad field name":      "tfv 1\nr R1 KA N\n",
		"field digit first":   "tfv 1\nr R1 K1a N\n",
		"missing K":           "tfv 1\nr R1 a N\n",
		"missing value":       "tfv 1\nr R1 Ka\n",
		"unknown tag":         "tfv 1\nr R1 Ka Z\n",
		"N body":              "tfv 1\nr R1 Ka N1\n",
		"short float":         "tfv 1\nr R1 Ka D3ff\n",
		"upper float":         "tfv 1\nr R1 Ka D3FF0000000000000\n",
		"overlong utf8":       "tfv 1\nr R1 Ka Sc080\n",
		"surrogate utf8":      "tfv 1\nr R1 Ka Seda080\n",
		"beyond unicode":      "tfv 1\nr R1 Ka Sf4908080\n",
		"truncated utf8":      "tfv 1\nr R1 Ka Se2\n",
		"cr in record":        "tfv 1\nr R1 Ka N\r\n",
		"list count":          "tfv 1\nr R1 Ka L2 N\n",
		"list count huge":     "tfv 1\nr R1 Ka L99999999999\n",
		"list leading zero":   "tfv 1\nr R1 Ka L01 N\n",
		"map missing value":   "tfv 1\nr R1 Ka M1 N\n",
		"duplicate map key":   "tfv 1\nr R1 Ka M2 I1 N I1 T\n",
		"duplicate nan keys":  "tfv 1\nr R1 Ka M2 D7ff8000000000000 N D7ff0000000000001 N\n",
		"duplicate list keys": "tfv 1\nr R1 Ka M2 L1 I1 N L1 I1 N\n",
		"depth 65":            "tfv 1\nr R1 Ka " + deep(65) + "\n",
		"invalid utf8 doc":    "tfv 1\n# \xff\n",
		"too large":           "tfv 1\n# " + strings.Repeat("a", MaxInputBytes) + "\n",
	}
	for name, doc := range bad {
		err := Validate(doc)
		if err == nil {
			t.Errorf("%s: accepted %q", name, doc)
			continue
		}
		if !errors.Is(err, ErrInvalid) {
			t.Errorf("%s: error %v does not wrap ErrInvalid", name, err)
		}
	}
}

func TestDepthBoundary(t *testing.T) {
	if err := Validate("tfv 1\nr R1 Ka " + deep(64) + "\n"); err != nil {
		t.Fatalf("depth 64 must be accepted: %v", err)
	}
	if err := Validate("tfv 1\nr R1 Ka M1 I1 " + deep(63) + "\n"); err != nil {
		t.Fatalf("mixed depth 64 must be accepted: %v", err)
	}
	if err := Validate("tfv 1\nr R1 Ka M1 I1 " + deep(64) + "\n"); err == nil {
		t.Fatal("mixed depth 65 must be rejected")
	}
}

func TestIntDigitBoundary(t *testing.T) {
	for _, sign := range []string{"", "-"} {
		ok := "tfv 1\nr R1 Ka I" + sign + "1" + strings.Repeat("0", 999) + "\n"
		if err := Validate(ok); err != nil {
			t.Fatalf("1000 digits must be accepted: %v", err)
		}
	}
}

func TestSignedZeroFloatsAreDistinctKeys(t *testing.T) {
	doc := "tfv 1\nr R1 Ka M2 D0000000000000000 N D8000000000000000 N\n"
	if err := Validate(doc); err != nil {
		t.Fatalf("-0 and +0 are different keys: %v", err)
	}
	inf := "tfv 1\nr R1 Ka M2 D7ff0000000000000 N D7ff0000000000000 N\n"
	if err := Validate(inf); err == nil {
		t.Fatal("equal infinities are duplicate keys")
	}
}

func TestMapKeyOrderIgnoredByDeepEquality(t *testing.T) {
	// Two maps as keys with the same entries in different order are the same key.
	doc := "tfv 1\nr R1 Ka M2 M2 I1 N I2 N N M2 I2 N I1 N N\n"
	if err := Validate(doc); err == nil {
		t.Fatal("maps equal up to entry order are duplicate keys")
	}
}
