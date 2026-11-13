package trace

import (
	"strconv"
	"strings"
)

// Addr is the destination of a network call, as strace printed it.
//
//	connect(3<socket:[1]>, {sa_family=AF_INET, sin_port=htons(443), sin_addr=inet_addr("93.184.216.34")}, 16)
//	  -> Addr{Family: "AF_INET", Address: "93.184.216.34", Port: 443}
//	bind(3<socket:[2]>, {sa_family=AF_UNIX, sun_path="/tmp/s"}, 110)
//	  -> Addr{Family: "AF_UNIX", Address: "/tmp/s", Port: -1}
//	connect(3<socket:[3]>, {sa_family=AF_UNIX, sun_path=@"dbus"}, 7)
//	  -> Addr{Family: "AF_UNIX", Address: "dbus", Port: -1, Abstract: true}
//
// Port is -1 when the family has none.  A pathname AF_UNIX address is also a
// filesystem path; the parser does not resolve it (stage 1b / §5.4 decide).
type Addr struct {
	Family   string
	Address  string
	Port     int
	Abstract bool
}

const maxSockaddrItems = 16

// parseSockaddr parses one "{sa_family=..., ...}" struct.  "NULL" gives nil
// when allowNull (only sendto may legitimately have no address); for connect
// and bind a NULL address is not a call we understand, so it is an error.
func parseSockaddr(raw string, allowNull bool) (*Addr, string) {
	raw = strings.TrimSpace(raw)
	if raw == "NULL" {
		if allowNull {
			return nil, ""
		}
		return nil, reasonSockaddr
	}
	if len(raw) < 2 || raw[0] != '{' || raw[len(raw)-1] != '}' {
		return nil, reasonSockaddr
	}
	res, r := scanItems(raw[1:len(raw)-1], 0, false, maxSockaddrItems)
	if r != "" {
		return nil, r
	}
	var fam string
	for _, it := range res.items {
		if v, ok := strings.CutPrefix(it, "sa_family="); ok {
			fam = v
		}
	}
	if !strings.HasPrefix(fam, "AF_") || !isErrnoName(fam) {
		return nil, reasonSockaddr
	}
	a := &Addr{Family: fam, Port: -1}
	find := func(prefix string) (string, bool) {
		for _, it := range res.items {
			if v, ok := strings.CutPrefix(it, prefix); ok {
				return v, true
			}
		}
		return "", false
	}
	// call("x") -> x, for inet_addr("1.2.3.4") and htons(80).
	port := func(prefix string) string {
		v, ok := find(prefix)
		if !ok {
			return reasonSockaddr
		}
		n, ok := strings.CutPrefix(v, "htons(")
		if !ok || !strings.HasSuffix(n, ")") {
			return reasonSockaddr
		}
		p, err := strconv.Atoi(n[:len(n)-1])
		if err != nil || p < 0 || p > 65535 {
			return reasonSockaddr
		}
		a.Port = p
		return ""
	}
	switch fam {
	case "AF_INET":
		if r := port("sin_port="); r != "" {
			return nil, r
		}
		v, ok := find("sin_addr=inet_addr(")
		if !ok || !strings.HasSuffix(v, ")") {
			return nil, reasonSockaddr
		}
		s, r := quotedOnly(v[:len(v)-1])
		if r != "" {
			return nil, r
		}
		a.Address = s
	case "AF_INET6":
		if r := port("sin6_port="); r != "" {
			return nil, r
		}
		v, ok := find("inet_pton(")
		if !ok {
			return nil, reasonSockaddr
		}
		inner, r := scanItems("inet_pton("+v, len("inet_pton("), true, 3)
		if r != "" || len(inner.items) != 3 {
			return nil, reasonSockaddr
		}
		s, r := quotedOnly(inner.items[1])
		if r != "" {
			return nil, r
		}
		a.Address = s
	case "AF_UNIX":
		v, ok := find("sun_path=")
		if !ok {
			break // unnamed socket
		}
		if strings.HasPrefix(v, "@") {
			a.Abstract = true
			v = v[1:]
		}
		if len(v) == 0 || v[0] != '"' {
			return nil, reasonSockaddr
		}
		val, trunc, end, r := decodeString(v, 0)
		if r != "" {
			return nil, r
		}
		if trunc {
			return nil, reasonTruncated
		}
		if end != len(v) {
			return nil, reasonSockaddr
		}
		if a.Abstract {
			a.Address = string(val)
		} else {
			s, r := pathBytes(val)
			if r != "" {
				return nil, r
			}
			a.Address = s
		}
	}
	return a, ""
}

// quotedOnly decodes a raw item that must be exactly one quoted string.
func quotedOnly(raw string) (string, string) {
	raw = strings.TrimSpace(raw)
	if len(raw) == 0 || raw[0] != '"' {
		return "", reasonSockaddr
	}
	val, trunc, end, r := decodeString(raw, 0)
	if r != "" {
		return "", r
	}
	if trunc {
		return "", reasonTruncated
	}
	if end != len(raw) {
		return "", reasonSockaddr
	}
	return pathBytes(val)
}

// findSockaddrs returns every "{sa_family=...}" struct nested anywhere in raw
// (sendmsg's msg_name, each message of sendmmsg).  Strings and annotations are
// skipped, so a buffer that merely contains the text "sa_family=" does not
// count.
func findSockaddrs(raw string) ([]string, string) {
	type open struct {
		at   int
		sock bool
	}
	var stack []open
	var out []string
	for j := 0; j < len(raw); {
		c := raw[j]
		switch c {
		case '"':
			e, ok := skipString(raw, j)
			if !ok {
				return nil, reasonSockaddr
			}
			j = e
			continue
		case '<':
			if j > 0 && isTokenChar(raw[j-1]) {
				_, e, r := scanAnnotation(raw, j)
				if r != "" {
					return nil, r
				}
				j = e
				continue
			}
			return nil, reasonSockaddr
		case '.':
			// A bare "..." element of an array means strace truncated the list:
			// destinations beyond it are unseen.
			if strings.HasPrefix(raw[j:], "...") && len(stack) > 0 && raw[stack[len(stack)-1].at] == '[' {
				return nil, reasonSockaddr
			}
		case '{', '[', '(':
			stack = append(stack, open{at: j})
			if c == '{' && strings.HasPrefix(raw[j+1:], "sa_family=") {
				stack[len(stack)-1].sock = true
			}
		case '}', ']', ')':
			if len(stack) == 0 {
				return nil, reasonSockaddr
			}
			top := stack[len(stack)-1]
			stack = stack[:len(stack)-1]
			if top.sock && c == '}' {
				out = append(out, raw[top.at:j+1])
			}
		}
		j++
	}
	if len(stack) != 0 {
		return nil, reasonSockaddr
	}
	return out, ""
}
