// Package hddsimulator implements a fixed-geometry, deterministic teaching HDD.
//
// It models 32 eight-byte sectors, a four-track head, and synchronous integer
// seek, rotation, and transfer timing. It is an in-memory model, not a drive
// controller, filesystem, cache, scheduler, persistent device, or real-time
// latency prediction.
package hddsimulator
