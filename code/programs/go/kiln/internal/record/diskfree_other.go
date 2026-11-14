//go:build !(linux || darwin)

package record

import "errors"

// freeBytes: the recorder runs only on Linux; elsewhere the measurement is an
// error, which RunStep treats as "no room" (it fails closed).
func freeBytes(string) (uint64, error) {
	return 0, errors.New("free space cannot be measured on this platform")
}
