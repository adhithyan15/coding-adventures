package hddgeometry

import "errors"

// ErrInvalidAddress identifies a coordinate outside its declared dimensions.
var ErrInvalidAddress = errors.New("invalid_hdd_address")
