# frozen_string_literal: true

# D12 Device Driver Framework — unified device abstraction for character, block,
# and network devices.
#
# A device driver is a piece of software that knows how to talk to a specific
# piece of hardware. Without drivers, every program that wanted to read from a
# disk would need to know the exact protocol for that specific disk model.
# Device drivers solve this by providing a **uniform interface** over diverse
# hardware.
#
# Analogy: Think of a universal remote control. You press "Volume Up" and it
# works on your Samsung TV, your Sony soundbar, and your LG projector. Each
# device speaks a different protocol, but the remote translates your single
# button press into the right signal. Device drivers are the universal remote
# for your operating system.
#
# This package provides:
#   - DeviceType enum (CHARACTER, BLOCK, NETWORK)
#   - Device base class with common fields (name, major, minor, etc.)
#   - CharacterDevice, BlockDevice, NetworkDevice abstract classes
#   - DeviceRegistry for registering and looking up devices
#   - Simulated implementations: Disk, Keyboard, Display, NIC
#   - SharedWire for connecting simulated NICs

require_relative "device_driver_framework/version"
require_relative "device_driver_framework/device"
require_relative "device_driver_framework/registry"
require_relative "device_driver_framework/simulated_disk"
require_relative "device_driver_framework/simulated_keyboard"
require_relative "device_driver_framework/simulated_display"
require_relative "device_driver_framework/shared_wire"
require_relative "device_driver_framework/simulated_nic"

module CodingAdventures
  module DeviceDriverFramework
    # Well-known interrupt numbers for devices.
    #
    # These follow the assignments from the spec:
    #
    #   Interrupt   Source        Description
    #   ---------   ------        -----------
    #   32          Timer         Timer tick (assigned in S03)
    #   33          Keyboard      Key pressed
    #   34          Disk          Block I/O completed
    #   35          NIC           Packet received
    #   128         Software      System call (assigned in S03)
    INT_TIMER = 32
    INT_KEYBOARD = 33
    INT_DISK = 34
    INT_NIC = 35
    INT_SYSCALL = 128

    # Well-known major numbers for device drivers.
    #
    # In Unix, the major number identifies which driver handles the device.
    # The minor number identifies which instance within that driver.
    #
    #   Major   Device Type
    #   -----   -----------
    #   1       Display (character)
    #   2       Keyboard (character)
    #   3       Disk (block)
    #   4       NIC (network)
    MAJOR_DISPLAY = 1
    MAJOR_KEYBOARD = 2
    MAJOR_DISK = 3
    MAJOR_NIC = 4
  end
end
