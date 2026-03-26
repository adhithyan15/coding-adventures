package systemboard

import (
	"github.com/adhithyan15/coding-adventures/code/packages/go/bootloader"
	"github.com/adhithyan15/coding-adventures/code/packages/go/display"
	oskernel "github.com/adhithyan15/coding-adventures/code/packages/go/os-kernel"
	rombios "github.com/adhithyan15/coding-adventures/code/packages/go/rom-bios"
)

// =========================================================================
// Address Space Constants
// =========================================================================
//
// These constants define the memory layout for the entire simulated computer.
// They form the contract between all S-series packages.

const (
	// ROMBase is where BIOS firmware lives. The CPU starts here on power-on.
	ROMBase uint32 = 0xFFFF0000

	// ROMSize is the size of the ROM region (64 KB).
	ROMSize uint32 = 0x00010000

	// BootProtocolAddr is where the BIOS writes hardware configuration.
	BootProtocolAddr uint32 = 0x00001000

	// BootloaderBase is where the bootloader code lives.
	BootloaderBase uint32 = 0x00010000

	// KernelBase is where the kernel code and data are loaded.
	KernelBase uint32 = 0x00020000

	// IdleProcessBase is PID 0's memory region.
	IdleProcessBase uint32 = 0x00030000

	// UserProcessBase is PID 1's memory region (hello-world).
	UserProcessBase uint32 = 0x00040000

	// KernelStackTop is where the kernel stack starts (grows downward).
	KernelStackTop uint32 = 0x0006FFF0

	// DiskMappedBase is where the disk image is memory-mapped.
	DiskMappedBase uint32 = 0x10000000

	// FramebufferBase is where the display framebuffer lives.
	FramebufferBase uint32 = 0xFFFB0000

	// KeyboardPort is the keyboard I/O port address.
	KeyboardPort uint32 = 0xFFFC0000
)

// =========================================================================
// SystemConfig
// =========================================================================

// SystemConfig holds all configuration for the complete simulated computer.
type SystemConfig struct {
	// MemorySize is the total addressable RAM (default: 1 MB).
	MemorySize int

	// DisplayConfig configures the text display (default: 80x25 VGA).
	DisplayConfig display.DisplayConfig

	// BIOSConfig configures the BIOS firmware.
	BIOSConfig rombios.BIOSConfig

	// BootloaderConfig configures the bootloader.
	BootloaderConfig bootloader.BootloaderConfig

	// KernelConfig configures the OS kernel.
	KernelConfig oskernel.KernelConfig

	// UserProgram is the binary for the user program (hello-world).
	// If nil, the default hello-world binary is generated.
	UserProgram []byte
}

// DefaultSystemConfig returns a configuration with sensible defaults for
// the hello-world demo. All addresses, sizes, and intervals are pre-configured
// so that PowerOn() + Run(100000) produces "Hello World" on the display.
func DefaultSystemConfig() SystemConfig {
	biosConfig := rombios.DefaultBIOSConfig()
	biosConfig.MemorySize = 1024 * 1024 // Skip memory probe, use 1 MB

	blConfig := bootloader.DefaultBootloaderConfig()

	return SystemConfig{
		MemorySize:       1024 * 1024,
		DisplayConfig:    display.DefaultDisplayConfig(),
		BIOSConfig:       biosConfig,
		BootloaderConfig: blConfig,
		KernelConfig:     oskernel.DefaultKernelConfig(),
		UserProgram:      nil, // Will generate default hello-world
	}
}
