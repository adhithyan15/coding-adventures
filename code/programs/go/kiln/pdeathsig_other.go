//go:build !linux

package main

// armParentDeath: there is no PR_SET_PDEATHSIG off Linux, and `kiln record step`
// refuses off linux/amd64 before it gets here.
func armParentDeath() error { return nil }
