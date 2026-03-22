# frozen_string_literal: true

require "test_helper"

class TestBootloaderConfig < Minitest::Test
  def test_default_config
    config = CodingAdventures::Bootloader::BootloaderConfig.new
    assert_equal 0x00010000, config.entry_address
    assert_equal 0, config.kernel_size
  end
end

class TestBootloaderGeneration < Minitest::Test
  def test_generate_returns_bytes
    config = CodingAdventures::Bootloader::BootloaderConfig.new(kernel_size: 256)
    bl = CodingAdventures::Bootloader::BootloaderGenerator.new(config)
    binary = bl.generate
    assert binary.is_a?(String)
    assert binary.length > 0
  end

  def test_generate_word_aligned
    config = CodingAdventures::Bootloader::BootloaderConfig.new(kernel_size: 256)
    bl = CodingAdventures::Bootloader::BootloaderGenerator.new(config)
    assert_equal 0, bl.generate.length % 4
  end

  def test_instruction_count
    config = CodingAdventures::Bootloader::BootloaderConfig.new(kernel_size: 256)
    bl = CodingAdventures::Bootloader::BootloaderGenerator.new(config)
    assert bl.instruction_count > 0
  end

  def test_estimate_cycles
    config = CodingAdventures::Bootloader::BootloaderConfig.new(kernel_size: 4096)
    bl = CodingAdventures::Bootloader::BootloaderGenerator.new(config)
    assert_equal 6164, bl.estimate_cycles
  end
end

class TestDiskImage < Minitest::Test
  def test_create
    disk = CodingAdventures::Bootloader::DiskImage.new
    assert_equal CodingAdventures::Bootloader::DEFAULT_DISK_SIZE, disk.size
  end

  def test_load_kernel
    disk = CodingAdventures::Bootloader::DiskImage.new
    disk.load_kernel([0xDE, 0xAD])
    assert_equal 0xDE, disk.read_byte_at(0x00080000)
  end

  def test_read_word
    disk = CodingAdventures::Bootloader::DiskImage.new
    disk.load_at(0, [0x78, 0x56, 0x34, 0x12])
    assert_equal 0x12345678, disk.read_word(0)
  end

  def test_out_of_bounds
    disk = CodingAdventures::Bootloader::DiskImage.new(4)
    assert_equal 0, disk.read_word(2)
    assert_equal 0, disk.read_byte_at(10)
  end
end
