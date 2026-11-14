//go:build !unix

package record

import (
	"errors"

	"github.com/adhithyan15/coding-adventures/code/programs/go/kiln/internal/traced"
)

// LiveFS: the recorder runs only on Linux; here every call refuses, so the
// package builds and a classification can never be mistaken for a real one.
type LiveFS struct{}

var errNoLiveFS = errors.New("the live disk view is not supported on this platform")

// Lstat implements traced.FS.
func (LiveFS) Lstat(string) (traced.FileInfo, error) { return traced.FileInfo{}, errNoLiveFS }

// Readlink implements traced.FS.
func (LiveFS) Readlink(string) (string, error) { return "", errNoLiveFS }

// ReadHead implements traced.FS.
func (LiveFS) ReadHead(string, int) ([]byte, error) { return nil, errNoLiveFS }

func readVerifiedHead(string, string, int64, int) ([]byte, error) { return nil, errNoLiveFS }

var _ traced.FS = LiveFS{}
