//go:build !unix && !windows

package irtojvmclassfile

import "os"

// On platforms that are neither Unix nor Windows (js/wasm, wasip1, plan9)
// nobody has checked what the filesystem can promise, so writeStrategyAuto
// refuses rather than guessing.  The Rust port makes the same choice.

// writeClassFilePlatform is the writeStrategyAuto walk on other platforms.
func writeClassFilePlatform(*JVMClassArtifact, string, string) (string, error) {
	return "", backendError("secure class-file output is unsupported on this platform")
}

// isReparsePoint reports Windows reparse points; there are none here.
func isReparsePoint(os.FileInfo) bool {
	return false
}
