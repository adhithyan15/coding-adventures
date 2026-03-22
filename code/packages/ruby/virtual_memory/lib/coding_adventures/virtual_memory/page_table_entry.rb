# frozen_string_literal: true

# = Page Table Entry (PTE)
#
# A Page Table Entry is the fundamental building block of virtual memory. It
# describes the mapping between one virtual page and one physical frame.
#
# == What is a PTE?
#
# Think of a page table as a phone book. Each entry maps a "name" (virtual
# page number) to an "address" (physical frame number). But a PTE stores
# more than just the frame number -- it also stores metadata about what
# you're allowed to do with that page.
#
# == Permission Bits
#
# Every PTE carries permission flags that the hardware checks on every
# memory access:
#
#   +----------+-----------------------------------------------------+
#   | Flag     | Meaning                                             |
#   +----------+-----------------------------------------------------+
#   | present  | Is this page currently in physical memory?          |
#   | dirty    | Has this page been written to since it was loaded?  |
#   | accessed | Has this page been read or written recently?        |
#   | writable | Can this page be written to?                        |
#   | executable | Can code on this page be executed?                |
#   | user_accessible | Can user-mode (non-kernel) code access it?  |
#   +----------+-----------------------------------------------------+
#
# == RISC-V Sv32 PTE bit layout
#
#   +--------------------+---+---+---+---+---+---+---+---+
#   | PPN (frame number) | D | A | G | U | X | W | R | V |
#   | bits 31-10         | 7 | 6 | 5 | 4 | 3 | 2 | 1 | 0 |
#   +--------------------+---+---+---+---+---+---+---+---+
#
#   V = Valid (our "present")   R = Readable
#   W = Writable                X = Executable
#   U = User-accessible         G = Global (not used here)
#   A = Accessed                D = Dirty

module CodingAdventures
  module VirtualMemory
    class PageTableEntry
      attr_accessor :frame_number, :present, :dirty, :accessed,
        :writable, :executable, :user_accessible

      # Create a new page table entry.
      #
      # By default, a freshly created PTE is not present (not mapped to any
      # physical frame), not dirty (never written), not accessed, writable
      # (most pages are), not executable, and user-accessible.
      #
      # @param frame_number [Integer] the physical frame this page maps to
      # @param present [Boolean] whether the page is in physical memory
      # @param writable [Boolean] whether writes are allowed
      # @param executable [Boolean] whether code execution is allowed
      # @param user_accessible [Boolean] whether user-mode can access it
      def initialize(
        frame_number: 0,
        present: false,
        dirty: false,
        accessed: false,
        writable: true,
        executable: false,
        user_accessible: true
      )
        @frame_number = frame_number
        @present = present
        @dirty = dirty
        @accessed = accessed
        @writable = writable
        @executable = executable
        @user_accessible = user_accessible
      end

      # Is this page currently in physical memory?
      #
      # If not present, any access triggers a page fault (interrupt 14).
      # The page might not be present because:
      #   - It was never allocated (lazy allocation)
      #   - It was swapped to disk
      #   - It is a demand-paged allocation waiting for first access
      def present?
        @present
      end

      # Has this page been written to since it was loaded into memory?
      #
      # The dirty bit is critical for page replacement: if a page is dirty,
      # it must be written back to disk before the frame can be reused.
      # If clean, we can simply discard the frame (the disk copy is current).
      def dirty?
        @dirty
      end

      # Has this page been read or written recently?
      #
      # The accessed bit is used by page replacement algorithms (especially
      # the Clock algorithm) to approximate LRU behavior. The hardware sets
      # this bit on every access; the OS periodically clears it.
      def accessed?
        @accessed
      end

      # Can this page be written to?
      #
      # Code pages (text segment) are read-only. Stack and heap pages are
      # writable. Copy-on-write pages start as read-only even if logically
      # writable -- the write triggers a fault that makes a private copy.
      def writable?
        @writable
      end

      # Can code on this page be executed?
      #
      # The NX (No-Execute) bit prevents code injection attacks. Data pages
      # (stack, heap) should not be executable. Only the text segment
      # (where the program's compiled code lives) is executable.
      def executable?
        @executable
      end

      # Can user-mode code access this page?
      #
      # Kernel pages are not user-accessible. This prevents user programs
      # from reading or modifying kernel data structures. A user-mode
      # access to a kernel page triggers a page fault.
      def user_accessible?
        @user_accessible
      end

      # Create a deep copy of this PTE.
      #
      # Used during fork/clone operations where we need independent copies
      # of page table entries so that modifying one process's PTE does not
      # affect the other's.
      def dup
        PageTableEntry.new(
          frame_number: @frame_number,
          present: @present,
          dirty: @dirty,
          accessed: @accessed,
          writable: @writable,
          executable: @executable,
          user_accessible: @user_accessible
        )
      end
    end
  end
end
