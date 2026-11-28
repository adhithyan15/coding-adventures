package historicalweb_test

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"math/rand"
	"os"
	"reflect"
	"strings"
	"testing"

	historicalweb "github.com/adhithyan15/coding-adventures/code/packages/go/historical-web"
)

type vectors struct {
	HTTP      httpVectors `json:"http-0.9"`
	Documents []document  `json:"html-tags-1992"`
	Exchanges []exchange  `json:"exchanges"`
}
type httpVectors struct {
	Requests        []request  `json:"requests"`
	InvalidAddress  []string   `json:"invalid_addresses"`
	InvalidRequest  []string   `json:"invalid_request_hex"`
	Responses       []response `json:"responses"`
	InvalidResponse []string   `json:"invalid_response_hex"`
}
type request struct {
	Address string `json:"address"`
	WireHex string `json:"wire_hex"`
}
type response struct {
	WireHex string `json:"wire_hex"`
	Body    string `json:"body"`
}
type document struct {
	ID     string            `json:"id"`
	Source string            `json:"source"`
	Tree   []json.RawMessage `json:"tree"`
}
type exchange struct {
	Address     string            `json:"address"`
	RequestHex  string            `json:"request_hex"`
	ResponseHex string            `json:"response_hex"`
	Tree        []json.RawMessage `json:"tree"`
}

func loadVectors(t *testing.T) vectors {
	t.Helper()
	data, err := os.ReadFile("../../../specs/fixtures/historical-web-v1/vectors.json")
	if err != nil {
		t.Fatal(err)
	}
	var v vectors
	if err := json.Unmarshal(data, &v); err != nil {
		t.Fatal(err)
	}
	if len(v.HTTP.Requests) == 0 || len(v.Documents) == 0 || len(v.Exchanges) == 0 {
		t.Fatal("fixture is missing a top-level category")
	}
	return v
}

func decodeHex(t *testing.T, encoded string) []byte {
	t.Helper()
	value, err := hex.DecodeString(encoded)
	if err != nil {
		t.Fatalf("invalid fixture hex %q: %v", encoded, err)
	}
	return value
}

func TestHTTPSharedVectors(t *testing.T) {
	v := loadVectors(t)
	for _, tc := range v.HTTP.Requests {
		wire := decodeHex(t, tc.WireHex)
		got, err := historicalweb.ParseHTTP09Request(wire)
		if err != nil || got != tc.Address {
			t.Errorf("parse %q: got %q, %v", tc.WireHex, got, err)
		}
		encoded, err := historicalweb.EncodeHTTP09Request(tc.Address)
		canonical := []byte("GET " + tc.Address + "\r\n")
		if err != nil || !bytes.Equal(encoded, canonical) {
			t.Errorf("encode %q: %x, %v", tc.Address, encoded, err)
		}
	}
	for _, address := range v.HTTP.InvalidAddress {
		if _, err := historicalweb.EncodeHTTP09Request(address); err == nil {
			t.Errorf("accepted invalid address %q", address)
		}
	}
	for _, wireHex := range v.HTTP.InvalidRequest {
		wire := decodeHex(t, wireHex)
		if _, err := historicalweb.ParseHTTP09Request(wire); err == nil {
			t.Errorf("accepted invalid wire %q", wireHex)
		}
	}
	for _, tc := range v.HTTP.Responses {
		body := decodeHex(t, tc.WireHex)
		got, err := historicalweb.DecodeHTTP09Response(body)
		if err != nil || got != tc.Body {
			t.Errorf("response %q: got %q, %v", tc.WireHex, got, err)
		}
	}
	for _, wireHex := range v.HTTP.InvalidResponse {
		wire := decodeHex(t, wireHex)
		if _, err := historicalweb.DecodeHTTP09Response(wire); err == nil {
			t.Errorf("accepted non-ASCII response %q", wireHex)
		}
	}
}

func nodeJSON(node historicalweb.Node) any {
	switch n := node.(type) {
	case historicalweb.TextNode:
		return n.Text
	case historicalweb.Element:
		return elementJSON(n)
	default:
		panic("unknown historical-web node")
	}
}
func elementJSON(e historicalweb.Element) map[string]any {
	children := make([]any, len(e.Children))
	for i, child := range e.Children {
		children[i] = nodeJSON(child)
	}
	attrs := make(map[string]any, len(e.Attributes))
	for key, value := range e.Attributes {
		if value == nil {
			attrs[key] = nil
		} else {
			attrs[key] = *value
		}
	}
	return map[string]any{"tag": e.Tag, "attributes": attrs, "children": children}
}
func expectedTree(t *testing.T, raw []json.RawMessage) []any {
	t.Helper()
	result := make([]any, len(raw))
	for i, item := range raw {
		var value any
		if err := json.Unmarshal(item, &value); err != nil {
			t.Fatal(err)
		}
		result[i] = value
	}
	return result
}
func actualTree(nodes []historicalweb.Node) []any {
	result := make([]any, len(nodes))
	for i, node := range nodes {
		result[i] = nodeJSON(node)
	}
	return result
}

func TestHTMLSharedVectors(t *testing.T) {
	v := loadVectors(t)
	for _, tc := range v.Documents {
		got, err := historicalweb.ParseHTMLTags1992(tc.Source)
		if err != nil {
			t.Errorf("%s: %v", tc.ID, err)
			continue
		}
		if want := expectedTree(t, tc.Tree); !reflect.DeepEqual(actualTree(got), want) {
			t.Errorf("%s: got %#v want %#v", tc.ID, actualTree(got), want)
		}
	}
}

func TestCombinedExchanges(t *testing.T) {
	v := loadVectors(t)
	for _, tc := range v.Exchanges {
		request := decodeHex(t, tc.RequestHex)
		response := decodeHex(t, tc.ResponseHex)
		address, err := historicalweb.ParseHTTP09Request(request)
		if err != nil || address != tc.Address {
			t.Errorf("request %q: %q, %v", tc.Address, address, err)
		}
		body, err := historicalweb.DecodeHTTP09Response(response)
		if err != nil {
			t.Fatal(err)
		}
		tree, err := historicalweb.ParseHTMLTags1992(body)
		if err != nil {
			t.Fatal(err)
		}
		if want := expectedTree(t, tc.Tree); !reflect.DeepEqual(actualTree(tree), want) {
			t.Errorf("exchange %q: got %#v want %#v", tc.Address, actualTree(tree), want)
		}
	}
}

func TestAnchorFirstValuelessAttributeWins(t *testing.T) {
	tree, err := historicalweb.ParseHTMLTags1992("<A HREF HREF=/later>text</A>")
	if err != nil {
		t.Fatal(err)
	}
	got := actualTree(tree)
	want := []any{map[string]any{
		"tag": "a", "attributes": map[string]any{"href": nil}, "children": []any{"text"},
	}}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %#v, want %#v", got, want)
	}
}

func TestDeepIgnoredMarkupRemainsWellFormed(t *testing.T) {
	var source strings.Builder
	for i := 0; i < 2000; i++ {
		source.WriteString("<A>")
	}
	for i := 0; i < 2000; i++ {
		source.WriteString("</TITLE>")
	}
	source.WriteString("tail")
	tree, err := historicalweb.ParseHTMLTags1992(source.String())
	if err != nil {
		t.Fatal(err)
	}
	assertTree(tree)
}

func TestShortWireDomain(t *testing.T) {
	for length := 0; length <= 2; length++ {
		limit := 1
		for i := 0; i < length; i++ {
			limit *= 256
		}
		for value := 0; value < limit; value++ {
			payload := make([]byte, length)
			n := value
			for i := range payload {
				payload[i] = byte(n & 255)
				n >>= 8
			}
			for _, terminator := range [][]byte{{'\n'}, {'\r', '\n'}} {
				wire := append([]byte("GET "), payload...)
				wire = append(wire, terminator...)
				got, err := historicalweb.ParseHTTP09Request(wire)
				expectedPayload := payload
				if len(terminator) == 1 && length > 0 && payload[length-1] == '\r' {
					expectedPayload = payload[:length-1]
				}
				valid := len(expectedPayload) > 0
				for _, b := range expectedPayload {
					valid = valid && b >= 0x21 && b <= 0x7e
				}
				if valid {
					if err != nil || got != string(expectedPayload) {
						t.Errorf("wire %x: got %q, %v", wire, got, err)
					}
				} else if err == nil {
					t.Errorf("accepted wire %x as %q", wire, got)
				}
			}
		}
	}
}

func TestIndependentProperties(t *testing.T) {
	rng := rand.New(rand.NewSource(0x19921103))
	for i := 0; i < 1000; i++ {
		addressLen := 1 + rng.Intn(32)
		address := make([]byte, addressLen)
		for j := range address {
			address[j] = byte(0x21 + rng.Intn(0x5e))
		}
		wire, err := historicalweb.EncodeHTTP09Request(string(address))
		if err != nil {
			t.Fatal(err)
		}
		got, err := historicalweb.ParseHTTP09Request(wire)
		if err != nil || got != string(address) {
			t.Fatalf("round trip: %q, %v", got, err)
		}
	}
	negatives := [][]byte{[]byte("GET / x\r\n"), []byte("POST /\n"), []byte("GET /\r\nextra"), []byte("GET \xff\n")}
	for _, wire := range negatives {
		if _, err := historicalweb.ParseHTTP09Request(wire); err == nil {
			t.Errorf("negative accepted: %q", wire)
		}
	}
}

func oracleRequest(wire []byte) (string, bool) {
	if len(wire) < 6 || !bytes.Equal(wire[:4], []byte("GET ")) || wire[len(wire)-1] != '\n' {
		return "", false
	}
	end := len(wire) - 1
	if end > 0 && wire[end-1] == '\r' {
		end--
	}
	address := wire[4:end]
	if len(address) == 0 {
		return "", false
	}
	for _, b := range address {
		if b < 0x21 || b > 0x7e {
			return "", false
		}
	}
	return string(address), true
}

func FuzzHTTP09Parser(f *testing.F) {
	for _, seed := range [][]byte{[]byte("GET /\r\n"), []byte("GET / x\r\n"), []byte("POST /\n"), []byte("GET \xff\n")} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, wire []byte) {
		want, valid := oracleRequest(wire)
		got, err := historicalweb.ParseHTTP09Request(wire)
		if (err == nil) != valid || (valid && got != want) {
			t.Fatalf("wire %x: got %q, %v; oracle %q, %v", wire, got, err, want, valid)
		}
	})
}

func FuzzHTTP09Encoder(f *testing.F) {
	for _, seed := range []string{"/", "/index.html", "", "/a b", "\xff"} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, address string) {
		valid := len(address) > 0
		for i := 0; i < len(address); i++ {
			valid = valid && address[i] >= 0x21 && address[i] <= 0x7e
		}
		wire, err := historicalweb.EncodeHTTP09Request(address)
		if (err == nil) != valid {
			t.Fatalf("address %q: got %v, valid %v", address, err, valid)
		}
		if valid && !bytes.Equal(wire, []byte("GET "+address+"\r\n")) {
			t.Fatalf("noncanonical wire %q", wire)
		}
	})
}

func FuzzHTTP09Response(f *testing.F) {
	for _, seed := range [][]byte{nil, []byte("HTTP/1.0 500\r\n"), []byte{0x80}} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, body []byte) {
		valid := true
		for _, b := range body {
			valid = valid && b < 0x80
		}
		got, err := historicalweb.DecodeHTTP09Response(body)
		if (err == nil) != valid || (valid && got != string(body)) {
			t.Fatalf("body %x: got %q, %v", body, got, err)
		}
	})
}

func assertTree(nodes []historicalweb.Node) {
	pending := append([]historicalweb.Node(nil), nodes...)
	for len(pending) > 0 {
		node := pending[len(pending)-1]
		pending = pending[:len(pending)-1]
		switch value := node.(type) {
		case historicalweb.TextNode:
		case historicalweb.Element:
			if value.Tag == "" || value.Tag != strings.ToLower(value.Tag) {
				panic("invalid element tag")
			}
			pending = append(pending, value.Children...)
		default:
			panic("unknown node type")
		}
	}
}

func FuzzHTMLParser(f *testing.F) {
	for _, seed := range []string{"", "<UL><LI>x<LI>y</UL>", "<PLAINTEXT><P>", "<!--", "<A HREF='unterminated"} {
		f.Add(seed)
	}
	f.Fuzz(func(t *testing.T, source string) {
		tree, err := historicalweb.ParseHTMLTags1992(source)
		if err == nil {
			assertTree(tree)
		}
	})
}
