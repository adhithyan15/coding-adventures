# frozen_string_literal: true

require_relative "test_helper"

module CodingAdventures
  module VirtualMemory
    class TestPageTableEntry < Minitest::Test
      # == Default Values ==
      #
      # A freshly created PTE should have sensible defaults:
      # not present (no physical frame assigned yet), not dirty,
      # not accessed, writable (most pages are), not executable,
      # and user-accessible.

      def test_default_values
        pte = PageTableEntry.new

        assert_equal 0, pte.frame_number
        refute pte.present?
        refute pte.dirty?
        refute pte.accessed?
        assert pte.writable?
        refute pte.executable?
        assert pte.user_accessible?
      end

      # == Custom Initialization ==
      #
      # All flags can be set at creation time. This is used when
      # creating a PTE for a known mapping (e.g., mapping a code
      # page that should be executable but read-only).

      def test_custom_initialization
        pte = PageTableEntry.new(
          frame_number: 42,
          present: true,
          dirty: true,
          accessed: true,
          writable: false,
          executable: true,
          user_accessible: false
        )

        assert_equal 42, pte.frame_number
        assert pte.present?
        assert pte.dirty?
        assert pte.accessed?
        refute pte.writable?
        assert pte.executable?
        refute pte.user_accessible?
      end

      # == Flag Mutation ==
      #
      # The hardware (simulated) sets the accessed and dirty bits
      # during memory access. The OS clears them during page
      # replacement scanning.

      def test_flag_mutation
        pte = PageTableEntry.new(frame_number: 10, present: true)

        # Simulate a read access.
        pte.accessed = true
        assert pte.accessed?

        # Simulate a write access.
        pte.dirty = true
        assert pte.dirty?

        # OS clears accessed bit during clock sweep.
        pte.accessed = false
        refute pte.accessed?
      end

      # == Frame Number ==
      #
      # The frame number can be changed when a page is remapped
      # (e.g., after a COW fault allocates a new frame).

      def test_frame_number_update
        pte = PageTableEntry.new(frame_number: 5, present: true)
        assert_equal 5, pte.frame_number

        pte.frame_number = 99
        assert_equal 99, pte.frame_number
      end

      # == Duplication ==
      #
      # PTE duplication is used during fork() to create independent
      # copies of page table entries.

      def test_dup_creates_independent_copy
        original = PageTableEntry.new(
          frame_number: 7,
          present: true,
          dirty: true,
          writable: true
        )

        copy = original.dup

        # Copy should have the same values.
        assert_equal 7, copy.frame_number
        assert copy.present?
        assert copy.dirty?
        assert copy.writable?

        # Modifying the copy should not affect the original.
        copy.frame_number = 99
        copy.writable = false

        assert_equal 7, original.frame_number
        assert original.writable?
      end

      # == Permission Combinations ==
      #
      # Real page table entries use combinations of permission bits
      # to represent different page types:

      def test_code_page_permissions
        # Code pages: readable + executable, NOT writable.
        # This prevents code injection attacks.
        pte = PageTableEntry.new(
          frame_number: 1,
          present: true,
          writable: false,
          executable: true,
          user_accessible: true
        )

        refute pte.writable?
        assert pte.executable?
        assert pte.user_accessible?
      end

      def test_data_page_permissions
        # Data pages (stack/heap): readable + writable, NOT executable.
        # This is the NX (No-Execute) bit in action.
        pte = PageTableEntry.new(
          frame_number: 2,
          present: true,
          writable: true,
          executable: false,
          user_accessible: true
        )

        assert pte.writable?
        refute pte.executable?
      end

      def test_kernel_page_permissions
        # Kernel pages: not user-accessible.
        # User-mode code accessing a kernel page triggers a fault.
        pte = PageTableEntry.new(
          frame_number: 3,
          present: true,
          writable: true,
          executable: true,
          user_accessible: false
        )

        refute pte.user_accessible?
      end

      # == Present Bit ==
      #
      # A page that is not present triggers a page fault on access.
      # This is the basis for demand paging and swapping.

      def test_not_present_page
        pte = PageTableEntry.new(frame_number: 0, present: false)
        refute pte.present?

        # Making it present (after fault handler allocates a frame).
        pte.present = true
        pte.frame_number = 42
        assert pte.present?
        assert_equal 42, pte.frame_number
      end
    end
  end
end
