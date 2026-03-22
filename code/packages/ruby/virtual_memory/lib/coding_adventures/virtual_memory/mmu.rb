# frozen_string_literal: true

# = Memory Management Unit (MMU)
#
# The MMU is the central component that ties together all the pieces
# of virtual memory: page tables, TLB, frame allocator, and page
# replacement policy.
#
# == What the MMU Does
#
# Every time a process accesses memory (load or store instruction),
# the CPU passes the virtual address to the MMU. The MMU's job is to:
#
#   1. Translate the virtual address to a physical address.
#   2. Check that the access is permitted (read/write/execute permissions).
#   3. Handle page faults when a page is not in memory.
#   4. Manage per-process address spaces (isolation).
#
# == The Translation Flow
#
#   CPU issues memory access with virtual address
#         |
#         v
#   +---> TLB lookup (fast path -- ~1 cycle)
#   |     |
#   |     v
#   |   Hit? ----yes----> Return physical address
#   |     |
#   |     no (TLB miss)
#   |     |
#   |     v
#   |   Page table walk (slow path -- 2-3 memory accesses)
#   |     |
#   |     v
#   |   Page present? ----yes----> Cache in TLB, return physical address
#   |     |
#   |     no (page fault)
#   |     |
#   |     v
#   |   Allocate frame, map page, retry
#   |     |
#   +-----+
#
# == Per-Process Address Spaces
#
# Each process has its own page table, so the same virtual address in
# different processes maps to different physical frames. The MMU tracks
# which process is currently running and uses the correct page table.
#
# == Copy-on-Write (COW)
#
# When fork() creates a child process, the MMU clones the parent's
# page table but shares the physical frames. Both processes' pages are
# marked read-only. When either process writes, the page fault handler
# makes a private copy of just that page. This makes fork() nearly
# free, even for processes using hundreds of MB of memory.

module CodingAdventures
  module VirtualMemory
    # Error raised when a process accesses memory it doesn't own.
    class SegmentationFault < RuntimeError; end

    # Error raised when a process violates page permissions.
    class ProtectionFault < RuntimeError; end

    class MMU
      attr_reader :tlb, :frame_allocator, :current_pid

      # Create a new MMU.
      #
      # @param total_frames [Integer] total physical frames available
      # @param replacement_policy [FIFOPolicy, LRUPolicy, ClockPolicy]
      #   the page replacement policy to use when memory is full
      # @param tlb_capacity [Integer] how many TLB entries to cache
      def initialize(total_frames:, replacement_policy: nil, tlb_capacity: DEFAULT_TLB_CAPACITY)
        @page_tables = {}          # pid => TwoLevelPageTable
        @tlb = TLB.new(capacity: tlb_capacity)
        @frame_allocator = FrameAllocator.new(total_frames)
        @replacement_policy = replacement_policy || FIFOPolicy.new

        # Track which (pid, vpn) owns each frame, for page replacement.
        # frame_number => [pid, vpn]
        @frame_owners = {}

        # The currently active process. TLB lookups use this.
        @current_pid = nil
      end

      # Create a new, empty address space for a process.
      #
      # Called when a new process is created from scratch (exec) or
      # when the first process is initialized at boot.
      #
      # @param pid [Integer] the process ID
      def create_address_space(pid)
        @page_tables[pid] = TwoLevelPageTable.new
      end

      # Destroy a process's address space and free all its frames.
      #
      # Called when a process exits. Iterates over all mapped pages,
      # decrements frame reference counts, and frees frames whose
      # refcount reaches zero.
      #
      # @param pid [Integer] the process ID
      def destroy_address_space(pid)
        page_table = @page_tables[pid]
        return unless page_table

        # Walk every second-level table and free mapped frames.
        page_table.directory.each_with_index do |table, _vpn1|
          next if table.nil?

          table.entries.each do |_vpn0, pte|
            next unless pte.present?

            frame = pte.frame_number
            @replacement_policy.remove_frame(frame)
            @frame_owners.delete(frame)
            @frame_allocator.decrement_refcount(frame)
          end
        end

        @page_tables.delete(pid)

        # Invalidate any TLB entries for this process.
        @tlb.flush
      end

      # Map a virtual address to a physical frame for a process.
      #
      # Allocates a frame from the frame allocator and creates the
      # mapping in the process's page table.
      #
      # @param pid [Integer] the process ID
      # @param vaddr [Integer] the virtual address to map
      # @param writable [Boolean] whether the page is writable
      # @param executable [Boolean] whether the page is executable
      # @param user_accessible [Boolean] whether user-mode can access it
      # @return [Integer] the allocated frame number
      # @raise [RuntimeError] if no frames are available
      def map_page(pid, vaddr, writable: true, executable: false, user_accessible: true)
        ensure_address_space!(pid)

        frame = allocate_frame_or_evict
        vpn = vaddr >> PAGE_OFFSET_BITS

        @page_tables[pid].map(vaddr, frame,
          writable: writable,
          executable: executable,
          user_accessible: user_accessible)

        @frame_owners[frame] = [pid, vpn]
        @replacement_policy.add_frame(frame)

        # Invalidate any stale TLB entry for this mapping.
        @tlb.invalidate(pid, vpn)

        frame
      end

      # Translate a virtual address to a physical address.
      #
      # This is the core operation of the MMU. Every memory access goes
      # through this method.
      #
      # @param pid [Integer] the process ID
      # @param vaddr [Integer] the 32-bit virtual address
      # @param write [Boolean] whether this is a write access
      # @return [Integer] the physical address
      # @raise [SegmentationFault] if the page is not mapped
      # @raise [ProtectionFault] if the access violates permissions
      def translate(pid, vaddr, write: false)
        ensure_address_space!(pid)

        vpn = vaddr >> PAGE_OFFSET_BITS
        offset = vaddr & PAGE_OFFSET_MASK

        # Step 1: Check the TLB (fast path).
        cached_frame = @tlb.lookup(pid, vpn)
        if cached_frame
          # TLB hit! But we still need to check write permissions.
          pte = @page_tables[pid].lookup_pte(vaddr)
          if write && pte && !pte.writable?
            handle_cow_fault(pid, vaddr, pte)
            pte = @page_tables[pid].lookup_pte(vaddr)
            cached_frame = pte.frame_number
          end

          # Update accessed/dirty bits.
          if pte
            pte.accessed = true
            pte.dirty = true if write
            @replacement_policy.record_access(cached_frame)
          end

          return (cached_frame << PAGE_OFFSET_BITS) | offset
        end

        # Step 2: TLB miss -- walk the page table.
        result = @page_tables[pid].translate(vaddr)

        if result.nil?
          # Page fault: the page is not mapped or not present.
          handle_page_fault(pid, vaddr)
          result = @page_tables[pid].translate(vaddr)

          if result.nil?
            raise SegmentationFault,
              "Process #{pid}: invalid access to address 0x#{vaddr.to_s(16)}"
          end
        end

        phys_addr, pte = result

        # Step 3: Check permissions.
        if write && !pte.writable?
          handle_cow_fault(pid, vaddr, pte)
          result = @page_tables[pid].translate(vaddr)
          phys_addr, pte = result
        end

        # Step 4: Update accessed/dirty bits.
        pte.accessed = true
        pte.dirty = true if write

        # Step 5: Cache the translation in the TLB.
        @tlb.insert(pid, vpn, pte.frame_number, pte)
        @replacement_policy.record_access(pte.frame_number)

        phys_addr
      end

      # Handle a page fault for a process.
      #
      # A page fault means the process accessed a virtual address that
      # is not currently mapped to a physical frame. We allocate a frame
      # and create the mapping.
      #
      # @param pid [Integer] the process ID
      # @param vaddr [Integer] the faulting virtual address
      # @return [Integer] the physical address after resolution
      def handle_page_fault(pid, vaddr)
        ensure_address_space!(pid)

        vpn = vaddr >> PAGE_OFFSET_BITS

        # Check if a PTE exists but is not present (demand paging).
        pte = @page_tables[pid].lookup_pte(vaddr)

        if pte && !pte.present?
          # Demand paging: allocate a frame and make the page present.
          frame = allocate_frame_or_evict
          pte.frame_number = frame
          pte.present = true
          pte.accessed = true

          @frame_owners[frame] = [pid, vpn]
          @replacement_policy.add_frame(frame)
          @tlb.invalidate(pid, vpn)

          return (frame << PAGE_OFFSET_BITS) | (vaddr & PAGE_OFFSET_MASK)
        end

        if pte.nil?
          # Allocate a new page (lazy allocation).
          frame = allocate_frame_or_evict
          @page_tables[pid].map(vaddr, frame)

          @frame_owners[frame] = [pid, vpn]
          @replacement_policy.add_frame(frame)
          @tlb.invalidate(pid, vpn)

          return (frame << PAGE_OFFSET_BITS) | (vaddr & PAGE_OFFSET_MASK)
        end

        (pte.frame_number << PAGE_OFFSET_BITS) | (vaddr & PAGE_OFFSET_MASK)
      end

      # Clone an address space (copy-on-write fork).
      #
      # Copies all page table entries from the source process to the
      # destination process. Physical frames are shared, not copied.
      # All shared pages are marked read-only in both processes. When
      # either process writes to a shared page, a COW fault creates a
      # private copy.
      #
      # @param from_pid [Integer] the source process ID
      # @param to_pid [Integer] the destination process ID
      def clone_address_space(from_pid, to_pid)
        ensure_address_space!(from_pid)
        create_address_space(to_pid)

        src_table = @page_tables[from_pid]

        src_table.directory.each_with_index do |table, vpn1|
          next if table.nil?

          table.entries.each do |vpn0, pte|
            next unless pte.present?

            # Reconstruct the virtual address from the two-level indices.
            vpn = (vpn1 << 10) | vpn0
            vaddr = vpn << PAGE_OFFSET_BITS

            # Share the physical frame between both processes.
            # Mark both copies as read-only to trigger COW faults.
            pte.writable = false

            @page_tables[to_pid].map(vaddr, pte.frame_number,
              writable: false,
              executable: pte.executable?,
              user_accessible: pte.user_accessible?)

            # Copy the dirty/accessed state.
            child_pte = @page_tables[to_pid].lookup_pte(vaddr)
            child_pte.dirty = pte.dirty?
            child_pte.accessed = pte.accessed?

            # Increment the frame's reference count.
            @frame_allocator.increment_refcount(pte.frame_number)

            # Store that this frame was originally writable (for COW).
            # We use a simple approach: tag the frame as COW-shared.
            @frame_owners[pte.frame_number] = [from_pid, vpn] unless @frame_owners.key?(pte.frame_number)
          end
        end

        # Flush TLB since parent's mappings changed (writable -> read-only).
        @tlb.flush
      end

      # Switch context to a new process.
      #
      # Flushes the TLB because the new process has different mappings.
      # Without flushing, process B might get process A's cached
      # translations, breaking memory isolation.
      #
      # @param new_pid [Integer] the process to switch to
      def context_switch(new_pid)
        @current_pid = new_pid
        @tlb.flush
      end

      # Check if a process has an address space.
      #
      # @param pid [Integer] the process ID
      # @return [Boolean] true if the process has an address space
      def address_space?(pid)
        @page_tables.key?(pid)
      end

      # Get the page table for a process (for testing/inspection).
      #
      # @param pid [Integer] the process ID
      # @return [TwoLevelPageTable, nil] the page table
      def page_table_for(pid)
        @page_tables[pid]
      end

      private

      # Ensure a process has an address space, raising an error if not.
      def ensure_address_space!(pid)
        unless @page_tables.key?(pid)
          raise ArgumentError, "Process #{pid} has no address space"
        end
      end

      # Allocate a frame, evicting a page if memory is full.
      #
      # @return [Integer] the allocated frame number
      # @raise [RuntimeError] if allocation fails even after eviction
      def allocate_frame_or_evict
        frame = @frame_allocator.allocate
        return frame if frame

        # Memory is full! Use the replacement policy to choose a victim.
        victim_frame = @replacement_policy.select_victim

        if victim_frame.nil?
          raise "Out of memory: no frames available and no victims to evict"
        end

        # Evict the victim: unmap it from its owner's page table.
        owner = @frame_owners[victim_frame]
        if owner
          owner_pid, owner_vpn = owner
          owner_table = @page_tables[owner_pid]
          if owner_table
            vaddr = owner_vpn << PAGE_OFFSET_BITS
            owner_table.unmap(vaddr)
            @tlb.invalidate(owner_pid, owner_vpn)
          end
          @frame_owners.delete(victim_frame)
        end

        # Free and re-allocate the victim frame.
        @frame_allocator.free(victim_frame) if @frame_allocator.allocated?(victim_frame)
        new_frame = @frame_allocator.allocate

        if new_frame.nil?
          raise "Out of memory: eviction failed to free a frame"
        end

        new_frame
      end

      # Handle a copy-on-write fault.
      #
      # When a process writes to a read-only page that is shared (via
      # fork), we make a private copy:
      #   1. Allocate a new frame.
      #   2. Copy the old frame's data (simulated -- we just remap).
      #   3. Update the writing process's PTE to point to the new frame.
      #   4. Mark the new mapping as writable.
      #   5. Decrement the old frame's reference count.
      #
      # @param pid [Integer] the process that triggered the fault
      # @param vaddr [Integer] the faulting virtual address
      # @param pte [PageTableEntry] the faulting PTE (read-only, shared)
      def handle_cow_fault(pid, vaddr, pte)
        old_frame = pte.frame_number
        vpn = vaddr >> PAGE_OFFSET_BITS
        refcount = @frame_allocator.refcount(old_frame)

        if refcount > 1
          # Frame is shared. Make a private copy.
          new_frame = allocate_frame_or_evict

          # Remap the page to the new frame with write permission.
          @page_tables[pid].map(vaddr, new_frame,
            writable: true,
            executable: pte.executable?,
            user_accessible: pte.user_accessible?)

          # Update owner tracking.
          @frame_owners[new_frame] = [pid, vpn]
          @replacement_policy.add_frame(new_frame)

          # Decrement the old frame's refcount.
          @frame_allocator.decrement_refcount(old_frame)

          # If the old frame now has refcount 1, the remaining owner
          # can have write access restored.
          if @frame_allocator.refcount(old_frame) == 1
            restore_owner = @frame_owners[old_frame]
            if restore_owner
              r_pid, _r_vpn = restore_owner
              r_pte = @page_tables[r_pid]&.lookup_pte(
                _r_vpn << PAGE_OFFSET_BITS
              )
              r_pte.writable = true if r_pte
            end
          end
        else
          # Frame is not shared. Just make it writable.
          pte.writable = true
        end

        @tlb.invalidate(pid, vpn)
      end
    end
  end
end
