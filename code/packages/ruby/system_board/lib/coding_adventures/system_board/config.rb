# frozen_string_literal: true

module CodingAdventures
  module SystemBoard
    ROM_BASE          = 0xFFFF0000
    BOOT_PROTOCOL_ADDR = 0x00001000
    BOOTLOADER_BASE   = 0x00010000
    KERNEL_BASE       = 0x00020000
    IDLE_PROCESS_BASE = 0x00030000
    USER_PROCESS_BASE = 0x00040000
    KERNEL_STACK_TOP  = 0x0006FFF0
    DISK_MAPPED_BASE  = 0x10000000
    FRAMEBUFFER_BASE  = 0xFFFB0000

    SystemConfig = Data.define(
      :memory_size, :display_config, :bios_config,
      :bootloader_config, :kernel_config, :user_program
    ) do
      def initialize(
        memory_size: 1024 * 1024,
        display_config: nil,
        bios_config: nil,
        bootloader_config: nil,
        kernel_config: nil,
        user_program: nil
      )
        super(
          memory_size: memory_size,
          display_config: display_config || CodingAdventures::Display::DisplayConfig.new,
          bios_config: bios_config || CodingAdventures::RomBios::BIOSConfig.new(memory_size: 1024 * 1024),
          bootloader_config: bootloader_config || CodingAdventures::Bootloader::BootloaderConfig.new,
          kernel_config: kernel_config || CodingAdventures::OsKernel.default_kernel_config,
          user_program: user_program
        )
      end
    end

    def self.default_system_config
      SystemConfig.new
    end
  end
end
