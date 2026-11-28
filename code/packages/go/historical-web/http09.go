// Package historicalweb implements the executable profiles in HWEB00.
package historicalweb

import "errors"

var errInvalidRequest = errors.New("invalid HTTP/0.9 request")
var errInvalidAddress = errors.New("invalid HTTP/0.9 document address")

// EncodeHTTP09Request formats one CERN HTTP/0.9 request line.
func EncodeHTTP09Request(address string) ([]byte, error) {
	if !validAddress(address) {
		return nil, errInvalidAddress
	}
	return append(append([]byte("GET "), address...), '\r', '\n'), nil
}

func validAddress(address string) bool {
	if len(address) == 0 {
		return false
	}
	for i := 0; i < len(address); i++ {
		if address[i] < 0x21 || address[i] > 0x7e {
			return false
		}
	}
	return true
}

// ParseHTTP09Request parses exactly one GET request terminated by CRLF or LF.
func ParseHTTP09Request(wire []byte) (string, error) {
	if len(wire) < 6 || string(wire[:4]) != "GET " {
		return "", errInvalidRequest
	}
	end := len(wire)
	if wire[end-1] != '\n' {
		return "", errInvalidRequest
	}
	end--
	if end > 0 && wire[end-1] == '\r' {
		end--
	}
	if end <= 4 || !validAddress(string(wire[4:end])) {
		return "", errInvalidRequest
	}
	return string(wire[4:end]), nil
}

// DecodeHTTP09Response decodes the complete ASCII response body.
func DecodeHTTP09Response(body []byte) (string, error) {
	for _, b := range body {
		if b > 0x7f {
			return "", errors.New("HTTP/0.9 response is not ASCII")
		}
	}
	return string(body), nil
}
