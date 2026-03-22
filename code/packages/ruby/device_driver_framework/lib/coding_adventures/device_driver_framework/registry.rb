# frozen_string_literal: true

module CodingAdventures
  module DeviceDriverFramework
    # DeviceRegistry — the kernel's phonebook for devices.
    #
    # When a driver initializes a device, it registers it here. When the kernel
    # needs to perform I/O, it looks up the device here. Think of it as a
    # telephone directory: you look up a name ("disk0") or a number pair
    # (major=3, minor=0), and get back a reference to the actual device.
    #
    # The registry maintains three data structures for fast lookup:
    #
    #   devices_by_name         — Hash mapping name string to Device instance
    #   devices_by_major_minor  — Hash mapping [major, minor] pair to Device
    #   all_devices             — Array of all registered Devices (preserves order)
    #
    # Why three structures? Different parts of the kernel need different access
    # patterns. The syscall handler looks up by name (the file descriptor table
    # maps fds to device names). The interrupt handler looks up by major/minor
    # (the IDT entry stores the device's major/minor). Enumeration (e.g., "list
    # all block devices") iterates the full list.
    #
    # Example boot sequence:
    #   registry = DeviceRegistry.new
    #   display = SimulatedDisplay.new
    #   display.init
    #   registry.register(display)   # Now accessible as "display0"
    #   registry.lookup_by_name("display0")  # => the display instance
    class DeviceRegistry
      attr_reader :all_devices

      def initialize
        @devices_by_name = {}
        @devices_by_major_minor = {}
        @all_devices = []
      end

      # Register a device in the registry.
      #
      # The device must already be initialized (device.initialized must be true).
      # Registration fails if a device with the same name already exists, or if
      # a device with the same (major, minor) pair already exists. These
      # constraints prevent accidental collisions — two devices claiming to be
      # "disk0" would cause chaos.
      #
      # @param device [Device] The device to register
      # @raise [ArgumentError] If the device is not initialized
      # @raise [ArgumentError] If a device with the same name exists
      # @raise [ArgumentError] If a device with the same (major, minor) exists
      def register(device)
        unless device.initialized
          raise ArgumentError, "Device '#{device.name}' must be initialized before registration"
        end

        if @devices_by_name.key?(device.name)
          raise ArgumentError, "Device with name '#{device.name}' is already registered"
        end

        key = [device.major, device.minor]
        if @devices_by_major_minor.key?(key)
          raise ArgumentError,
            "Device with major=#{device.major}, minor=#{device.minor} is already registered"
        end

        @devices_by_name[device.name] = device
        @devices_by_major_minor[key] = device
        @all_devices << device
      end

      # Remove a device from the registry by name.
      #
      # @param name [String] The device name to unregister
      # @return [Device, nil] The removed device, or nil if not found
      def unregister(name)
        device = @devices_by_name.delete(name)
        return nil unless device

        @devices_by_major_minor.delete([device.major, device.minor])
        @all_devices.delete(device)
        device
      end

      # Look up a device by its human-readable name.
      #
      # @param name [String] The device name (e.g., "disk0")
      # @return [Device, nil] The device, or nil if not found
      def lookup_by_name(name)
        @devices_by_name[name]
      end

      # Look up a device by its (major, minor) number pair.
      #
      # In Unix, the kernel routes I/O requests using these numbers:
      #   1. Look up major number to find the driver
      #   2. Pass the minor number to the driver to select the instance
      #
      # @param major [Integer] Driver identifier
      # @param minor [Integer] Instance identifier
      # @return [Device, nil] The device, or nil if not found
      def lookup_by_major_minor(major, minor)
        @devices_by_major_minor[[major, minor]]
      end

      # Return all registered devices.
      #
      # @return [Array<Device>] All devices, in registration order
      def list_all
        @all_devices.dup
      end

      # Return all registered devices of a specific type.
      #
      # Useful for enumeration, e.g., "list all block devices" so the
      # filesystem layer knows which disks are available.
      #
      # @param device_type [Integer] One of DeviceType::CHARACTER, BLOCK, NETWORK
      # @return [Array<Device>] Devices of that type
      def list_by_type(device_type)
        @all_devices.select { |d| d.device_type == device_type }
      end

      # Return the number of registered devices.
      #
      # @return [Integer] Count of devices
      def size
        @all_devices.length
      end
    end
  end
end
