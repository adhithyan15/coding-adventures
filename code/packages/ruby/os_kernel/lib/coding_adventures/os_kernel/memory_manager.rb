# frozen_string_literal: true

module CodingAdventures
  module OsKernel
    PERM_READ    = 0x01
    PERM_WRITE   = 0x02
    PERM_EXECUTE = 0x04

    MemoryRegion = Data.define(:base, :size, :permissions, :owner, :name)

    class MemoryManager
      attr_reader :regions

      def initialize(regions = [])
        @regions = regions.dup
      end

      def find_region(address)
        @regions.find { |r| address >= r.base && address < r.base + r.size }
      end

      def check_access(pid, address, perm)
        r = find_region(address)
        return false if r.nil?
        return false if r.owner != -1 && r.owner != pid
        (r.permissions & perm) == perm
      end

      def allocate_region(pid, base, size, perm, name)
        @regions << MemoryRegion.new(base: base, size: size, permissions: perm, owner: pid, name: name)
      end

      def region_count = @regions.length
    end
  end
end
