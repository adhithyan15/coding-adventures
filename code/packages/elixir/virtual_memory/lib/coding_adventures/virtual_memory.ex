defmodule CodingAdventures.VirtualMemory do
  @moduledoc """
  # Virtual Memory Subsystem

  Virtual memory is one of the most important abstractions in computer science.
  It gives every process the illusion that it has the entire memory space to
  itself — starting at address 0, stretching to some large upper limit — even
  though the physical machine has limited RAM shared among many processes.

  ## Analogy: The Apartment Building

  Imagine an apartment building. Each tenant thinks their apartment number
  starts at "Room 1." But the building manager knows Tenant A's "Room 1"
  is actually physical room 401, Tenant B's "Room 1" is physical room 712.
  The tenants never need to know their real room numbers. They just say
  "go to my Room 1" and the building manager (the MMU) translates.

  ## Address Layout (Sv32)

      32-bit virtual address:
      ┌────────────┬────────────┬────────────────┐
      │ VPN[1]     │ VPN[0]     │ Page Offset    │
      │ bits 31–22 │ bits 21–12 │ bits 11–0      │
      │ (10 bits)  │ (10 bits)  │ (12 bits)      │
      └────────────┴────────────┴────────────────┘

  The offset (12 bits) addresses each byte within a 4 KB page (2^12 = 4096).
  The VPN identifies which page. The physical address is formed by replacing
  the VPN with the physical frame number:

      physical_address = (frame_number <<< 12) ||| offset
  """

  import Bitwise

  # ============================================================================
  # Constants
  # ============================================================================

  # PAGE_SIZE is 4096 bytes (4 KB). Standard since Intel 386 (1985).
  @page_size 4096

  # PAGE_OFFSET_BITS is 12 because 2^12 = 4096 = PAGE_SIZE.
  @page_offset_bits 12

  # PAGE_OFFSET_MASK isolates the lower 12 bits: 0xFFF.
  @page_offset_mask @page_size - 1

  # Default TLB capacity — 64 entries.
  @default_tlb_capacity 64

  def page_size, do: @page_size
  def page_offset_bits, do: @page_offset_bits
  def page_offset_mask, do: @page_offset_mask
  def default_tlb_capacity, do: @default_tlb_capacity

  # ============================================================================
  # PageTableEntry
  # ============================================================================

  defmodule PageTableEntry do
    @moduledoc """
    A PageTableEntry describes the mapping for one virtual page.

    Each field corresponds to a hardware bit in a real page table entry (RISC-V Sv32):

    - `frame_number` — Which physical frame this page maps to.
    - `present` — Is this page currently in physical memory?
    - `dirty` — Has this page been written to since it was loaded?
    - `accessed` — Has this page been read or written recently?
    - `writable` — Can this page be written to?
    - `executable` — Can this page contain executable code?
    - `user_accessible` — Can user-mode code access this page?
    """
    @enforce_keys [:frame_number]
    defstruct [
      :frame_number,
      present: true,
      dirty: false,
      accessed: false,
      writable: false,
      executable: false,
      user_accessible: false
    ]

    @type t :: %__MODULE__{
            frame_number: non_neg_integer(),
            present: boolean(),
            dirty: boolean(),
            accessed: boolean(),
            writable: boolean(),
            executable: boolean(),
            user_accessible: boolean()
          }

    @doc "Create a new PTE with the given frame number and flags."
    def new(frame_number, opts \\ []) do
      %__MODULE__{
        frame_number: frame_number,
        present: Keyword.get(opts, :present, true),
        dirty: Keyword.get(opts, :dirty, false),
        accessed: Keyword.get(opts, :accessed, false),
        writable: Keyword.get(opts, :writable, false),
        executable: Keyword.get(opts, :executable, false),
        user_accessible: Keyword.get(opts, :user_accessible, false)
      }
    end
  end

  # ============================================================================
  # PageTable (Single-Level)
  # ============================================================================

  defmodule PageTable do
    @moduledoc """
    A single-level page table: a map from virtual page number (VPN) to PTE.

    This is the simplest implementation — a hash map. It does not match how real
    hardware works (real CPUs walk multi-level tables in hardware), but it serves
    as a building block for the two-level page table.
    """
    defstruct entries: %{}

    @type t :: %__MODULE__{entries: %{non_neg_integer() => PageTableEntry.t()}}

    @doc "Create a new empty page table."
    def new, do: %__MODULE__{}

    @doc "Map a virtual page number to a physical frame with the given flags."
    def map_page(%__MODULE__{} = table, vpn, frame_number, flags \\ []) do
      pte =
        PageTableEntry.new(frame_number,
          present: true,
          writable: Keyword.get(flags, :writable, false),
          executable: Keyword.get(flags, :executable, false),
          user_accessible: Keyword.get(flags, :user_accessible, false)
        )

      %{table | entries: Map.put(table.entries, vpn, pte)}
    end

    @doc "Remove a mapping for a virtual page number."
    def unmap_page(%__MODULE__{} = table, vpn) do
      pte = Map.get(table.entries, vpn)
      new_table = %{table | entries: Map.delete(table.entries, vpn)}
      {new_table, pte}
    end

    @doc "Look up the PTE for a virtual page number."
    def lookup(%__MODULE__{} = table, vpn) do
      Map.get(table.entries, vpn)
    end

    @doc "How many pages are currently mapped."
    def mapped_count(%__MODULE__{} = table) do
      map_size(table.entries)
    end

    @doc "Get all mapped VPNs."
    def get_all_vpns(%__MODULE__{} = table) do
      Map.keys(table.entries)
    end

    @doc "Insert a PTE directly."
    def insert(%__MODULE__{} = table, vpn, %PageTableEntry{} = pte) do
      %{table | entries: Map.put(table.entries, vpn, pte)}
    end
  end

  # ============================================================================
  # TwoLevelPageTable (Sv32)
  # ============================================================================

  defmodule TwoLevelPageTable do
    @moduledoc """
    RISC-V Sv32 two-level page table.

    The 20-bit VPN is split into two 10-bit indices:
    - VPN[1] (bits 31–22) indexes into the page directory (1024 entries)
    - VPN[0] (bits 21–12) indexes into a page table (1024 entries)

    Why two levels? A flat table would need 2^20 = 1,048,576 entries (4 MB per
    process). With two levels, we only allocate second-level tables for regions
    actually in use.
    """
    defstruct directory: %{}

    @type t :: %__MODULE__{directory: %{non_neg_integer() => PageTable.t()}}

    import Bitwise

    @page_offset_bits 12
    @page_offset_mask (1 <<< @page_offset_bits) - 1

    @doc "Create a new empty two-level page table."
    def new, do: %__MODULE__{}

    @doc """
    Split a 32-bit virtual address into VPN[1], VPN[0], and offset.

    Example: address 0x00012ABC
      vpn = 0x12ABC >>> 12 = 0x12 = 18
      vpn1 = 18 >>> 10 = 0
      vpn0 = 18 &&& 0x3FF = 18
      offset = 0xABC
    """
    def split_address(vaddr) do
      # Ensure unsigned by masking to 32 bits
      addr = vaddr &&& 0xFFFFFFFF
      vpn = addr >>> @page_offset_bits
      vpn1 = (vpn >>> 10) &&& 0x3FF
      vpn0 = vpn &&& 0x3FF
      page_offset = addr &&& @page_offset_mask
      {vpn1, vpn0, page_offset}
    end

    @doc """
    Map a virtual address to a physical frame. Creates second-level tables
    on demand — this is what makes two-level page tables memory-efficient.
    """
    def map(%__MODULE__{} = pt, vaddr, frame_number, flags \\ []) do
      {vpn1, vpn0, _offset} = split_address(vaddr)

      table = Map.get(pt.directory, vpn1, PageTable.new())
      table = PageTable.map_page(table, vpn0, frame_number, flags)

      %{pt | directory: Map.put(pt.directory, vpn1, table)}
    end

    @doc """
    Translate a virtual address to {physical_address, pte} or nil.

    This is the page table "walk" that hardware performs on every TLB miss:
    1. Use VPN[1] to index into the page directory.
    2. Use VPN[0] to index into the second-level page table.
    3. Combine the frame number with the offset.
    """
    def translate(%__MODULE__{} = pt, vaddr) do
      {vpn1, vpn0, page_offset} = split_address(vaddr)

      with table when not is_nil(table) <- Map.get(pt.directory, vpn1),
           %PageTableEntry{present: true} = pte <- PageTable.lookup(table, vpn0) do
        phys_addr = (pte.frame_number <<< @page_offset_bits) ||| page_offset
        {phys_addr, pte}
      else
        _ -> nil
      end
    end

    @doc "Remove a mapping for a virtual address."
    def unmap(%__MODULE__{} = pt, vaddr) do
      {vpn1, vpn0, _offset} = split_address(vaddr)

      case Map.get(pt.directory, vpn1) do
        nil ->
          {pt, nil}

        table ->
          {new_table, pte} = PageTable.unmap_page(table, vpn0)
          {%{pt | directory: Map.put(pt.directory, vpn1, new_table)}, pte}
      end
    end

    @doc "Look up the PTE for a virtual address without computing the physical address."
    def lookup_pte(%__MODULE__{} = pt, vaddr) do
      {vpn1, vpn0, _offset} = split_address(vaddr)

      case Map.get(pt.directory, vpn1) do
        nil -> nil
        table -> PageTable.lookup(table, vpn0)
      end
    end

    @doc """
    Update the PTE at a virtual address by applying a function to it.
    Returns the updated TwoLevelPageTable.
    """
    def update_pte(%__MODULE__{} = pt, vaddr, update_fn) do
      {vpn1, vpn0, _offset} = split_address(vaddr)

      case Map.get(pt.directory, vpn1) do
        nil ->
          pt

        table ->
          case PageTable.lookup(table, vpn0) do
            nil ->
              pt

            pte ->
              new_pte = update_fn.(pte)
              new_table = PageTable.insert(table, vpn0, new_pte)
              %{pt | directory: Map.put(pt.directory, vpn1, new_table)}
          end
      end
    end

    @doc """
    Return all mappings as a list of {vaddr, pte} tuples.
    Used by clone_address_space and destroy_address_space.
    """
    def all_mappings(%__MODULE__{} = pt) do
      Enum.flat_map(pt.directory, fn {vpn1, table} ->
        Enum.map(PageTable.get_all_vpns(table), fn vpn0 ->
          pte = PageTable.lookup(table, vpn0)
          vaddr = ((vpn1 <<< 10) ||| vpn0) <<< @page_offset_bits
          {vaddr, pte}
        end)
      end)
    end
  end

  # ============================================================================
  # TLB (Translation Lookaside Buffer)
  # ============================================================================

  defmodule TLB do
    @moduledoc """
    The TLB is a small, fast cache of recent virtual-to-physical translations.

    Without a TLB, every memory access would need 2-3 extra memory accesses to
    walk the page table. Programs exhibit temporal locality (accessing the same
    pages repeatedly), so a small TLB captures most translations.

    The TLB is keyed by {pid, vpn}. On context switch, it is flushed to prevent
    the new process from seeing the old process's translations.
    """
    defstruct capacity: 64,
              entries: %{},
              order: [],
              hits: 0,
              misses: 0

    @type t :: %__MODULE__{
            capacity: pos_integer(),
            entries: %{{non_neg_integer(), non_neg_integer()} => {non_neg_integer(), PageTableEntry.t()}},
            order: [{non_neg_integer(), non_neg_integer()}],
            hits: non_neg_integer(),
            misses: non_neg_integer()
          }

    @doc "Create a new TLB with the given capacity."
    def new(capacity \\ 64), do: %__MODULE__{capacity: capacity}

    @doc """
    Look up a cached translation. Returns {updated_tlb, result} where result
    is {frame, pte} on hit or nil on miss.
    """
    def lookup(%__MODULE__{} = tlb, pid, vpn) do
      key = {pid, vpn}

      case Map.get(tlb.entries, key) do
        nil ->
          {%{tlb | misses: tlb.misses + 1}, nil}

        {_frame, _pte} = result ->
          # Move to end of order list (most recently used)
          new_order = (tlb.order -- [key]) ++ [key]
          {%{tlb | hits: tlb.hits + 1, order: new_order}, result}
      end
    end

    @doc """
    Insert a translation into the TLB. Evicts the LRU entry if at capacity.
    """
    def insert(%__MODULE__{} = tlb, pid, vpn, frame, %PageTableEntry{} = pte) do
      key = {pid, vpn}

      # Remove if already present
      tlb =
        if Map.has_key?(tlb.entries, key) do
          %{tlb | entries: Map.delete(tlb.entries, key), order: tlb.order -- [key]}
        else
          tlb
        end

      # Evict LRU if at capacity
      tlb =
        if map_size(tlb.entries) >= tlb.capacity do
          case tlb.order do
            [lru_key | remaining] ->
              %{tlb | entries: Map.delete(tlb.entries, lru_key), order: remaining}

            [] ->
              tlb
          end
        else
          tlb
        end

      %{tlb |
        entries: Map.put(tlb.entries, key, {frame, pte}),
        order: tlb.order ++ [key]
      }
    end

    @doc "Invalidate a specific entry."
    def invalidate(%__MODULE__{} = tlb, pid, vpn) do
      key = {pid, vpn}
      %{tlb | entries: Map.delete(tlb.entries, key), order: tlb.order -- [key]}
    end

    @doc """
    Flush ALL entries. Called on context switch — the new process has a
    different page table, so all cached translations are stale.
    """
    def flush(%__MODULE__{} = tlb) do
      %{tlb | entries: %{}, order: []}
    end

    @doc """
    Hit rate: hits / (hits + misses). Returns 0 if no lookups performed.
    """
    def hit_rate(%__MODULE__{hits: h, misses: m}) when h + m == 0, do: 0.0
    def hit_rate(%__MODULE__{hits: h, misses: m}), do: h / (h + m)

    @doc "Number of cached entries."
    def size(%__MODULE__{} = tlb), do: map_size(tlb.entries)
  end

  # ============================================================================
  # PhysicalFrameAllocator
  # ============================================================================

  defmodule PhysicalFrameAllocator do
    @moduledoc """
    Manages which physical frames are free and which are in use.

    Uses a MapSet as a "bitmap" where membership means the frame is allocated.
    allocate() scans linearly for the first free frame — simple but O(n).
    Real allocators use free lists or buddy systems for O(1) allocation.
    """
    defstruct total_frames: 0,
              allocated: MapSet.new(),
              free_count: 0

    @type t :: %__MODULE__{
            total_frames: non_neg_integer(),
            allocated: MapSet.t(non_neg_integer()),
            free_count: non_neg_integer()
          }

    @doc "Create a new allocator with the given number of frames, all free."
    def new(total_frames) do
      %__MODULE__{total_frames: total_frames, free_count: total_frames}
    end

    @doc """
    Allocate the first free frame. Returns {updated_allocator, frame_number}
    or {allocator, nil} if out of memory.
    """
    def allocate(%__MODULE__{} = alloc) do
      result =
        Enum.find(0..(alloc.total_frames - 1), fn i ->
          not MapSet.member?(alloc.allocated, i)
        end)

      case result do
        nil ->
          {alloc, nil}

        frame ->
          new_alloc = %{alloc |
            allocated: MapSet.put(alloc.allocated, frame),
            free_count: alloc.free_count - 1
          }
          {new_alloc, frame}
      end
    end

    @doc """
    Free a frame. Raises if the frame is already free (double-free) or
    out of range.
    """
    def free(%__MODULE__{} = alloc, frame_number) do
      if frame_number < 0 or frame_number >= alloc.total_frames do
        raise "Frame number #{frame_number} out of range [0, #{alloc.total_frames})"
      end

      if not MapSet.member?(alloc.allocated, frame_number) do
        raise "Double-free: frame #{frame_number} is already free"
      end

      %{alloc |
        allocated: MapSet.delete(alloc.allocated, frame_number),
        free_count: alloc.free_count + 1
      }
    end

    @doc "Check if a frame is currently allocated."
    def is_allocated(%__MODULE__{} = alloc, frame_number) do
      if frame_number < 0 or frame_number >= alloc.total_frames do
        raise "Frame number #{frame_number} out of range [0, #{alloc.total_frames})"
      end

      MapSet.member?(alloc.allocated, frame_number)
    end
  end

  # ============================================================================
  # Page Replacement Policies
  # ============================================================================

  # Page replacement policies decide which page to evict when physical memory
  # is full. The three classic policies are FIFO, LRU, and Clock.

  # ---- FIFO Policy ----

  defmodule FIFOPolicy do
    @moduledoc """
    FIFO (First-In, First-Out): evict the oldest page — the one that has been
    in memory the longest.

    Simple but can be pathological — it might evict a frequently used page.
    """
    defstruct queue: :queue.new()

    @type t :: %__MODULE__{queue: :queue.queue(non_neg_integer())}

    def new, do: %__MODULE__{}

    @doc "FIFO ignores access patterns — only arrival order matters."
    def record_access(%__MODULE__{} = policy, _frame), do: policy

    @doc "Evict the oldest frame."
    def select_victim(%__MODULE__{} = policy) do
      case :queue.out(policy.queue) do
        {{:value, frame}, new_queue} -> {%{policy | queue: new_queue}, frame}
        {:empty, _} -> {policy, nil}
      end
    end

    @doc "Add a new frame to the back of the queue."
    def add_frame(%__MODULE__{} = policy, frame) do
      %{policy | queue: :queue.in(frame, policy.queue)}
    end

    @doc "Remove a specific frame from tracking."
    def remove_frame(%__MODULE__{} = policy, frame) do
      new_queue =
        policy.queue
        |> :queue.to_list()
        |> Enum.reject(&(&1 == frame))
        |> :queue.from_list()

      %{policy | queue: new_queue}
    end
  end

  # ---- LRU Policy ----

  defmodule LRUPolicy do
    @moduledoc """
    LRU (Least Recently Used): evict the page that has not been accessed for
    the longest time. Based on temporal locality — if a page was used recently,
    it will probably be used again soon.
    """
    defstruct access_order: [],
              timestamps: %{},
              counter: 0

    @type t :: %__MODULE__{
            access_order: [non_neg_integer()],
            timestamps: %{non_neg_integer() => non_neg_integer()},
            counter: non_neg_integer()
          }

    def new, do: %__MODULE__{}

    @doc "Record an access — move the frame to the most recently used position."
    def record_access(%__MODULE__{} = policy, frame) do
      new_order = (policy.access_order -- [frame]) ++ [frame]

      %{policy |
        access_order: new_order,
        timestamps: Map.put(policy.timestamps, frame, policy.counter),
        counter: policy.counter + 1
      }
    end

    @doc "Evict the least recently used frame (front of the list)."
    def select_victim(%__MODULE__{access_order: []} = policy), do: {policy, nil}

    def select_victim(%__MODULE__{access_order: [victim | remaining]} = policy) do
      {%{policy |
        access_order: remaining,
        timestamps: Map.delete(policy.timestamps, victim)
      }, victim}
    end

    @doc "Add a newly allocated frame as most recently used."
    def add_frame(%__MODULE__{} = policy, frame) do
      %{policy |
        access_order: policy.access_order ++ [frame],
        timestamps: Map.put(policy.timestamps, frame, policy.counter),
        counter: policy.counter + 1
      }
    end

    @doc "Remove a frame from tracking."
    def remove_frame(%__MODULE__{} = policy, frame) do
      %{policy |
        access_order: policy.access_order -- [frame],
        timestamps: Map.delete(policy.timestamps, frame)
      }
    end
  end

  # ---- Clock Policy ----

  defmodule ClockPolicy do
    @moduledoc """
    Clock (Second-Chance): a practical approximation of LRU. Pages are arranged
    in a circular buffer with "use bits." A clock hand sweeps around:

    1. If the frame under the hand has use=0, evict it.
    2. If use=1, clear the bit (second chance) and advance the hand.

    This is what most real operating systems use.
    """
    defstruct frames: [],
              use_bits: %{},
              hand: 0

    @type t :: %__MODULE__{
            frames: [non_neg_integer()],
            use_bits: %{non_neg_integer() => boolean()},
            hand: non_neg_integer()
          }

    def new, do: %__MODULE__{}

    @doc "Set the use bit for a frame (it was recently accessed)."
    def record_access(%__MODULE__{} = policy, frame) do
      if Map.has_key?(policy.use_bits, frame) do
        %{policy | use_bits: Map.put(policy.use_bits, frame, true)}
      else
        policy
      end
    end

    @doc """
    Select a victim using the clock algorithm. Sweeps at most 2x around
    the circle (once to clear all bits, once to find use=0).
    """
    def select_victim(%__MODULE__{frames: []} = policy), do: {policy, nil}

    def select_victim(%__MODULE__{} = policy) do
      max_scans = length(policy.frames) * 2
      do_select_victim(policy, max_scans)
    end

    defp do_select_victim(policy, 0), do: {policy, nil}

    defp do_select_victim(%__MODULE__{frames: frames} = policy, remaining) do
      frame = Enum.at(frames, policy.hand)
      use_bit = Map.get(policy.use_bits, frame, false)

      if not use_bit do
        # Evict this frame
        new_frames = List.delete_at(frames, policy.hand)
        new_hand =
          if length(new_frames) > 0,
            do: rem(policy.hand, length(new_frames)),
            else: 0

        {%{policy |
          frames: new_frames,
          use_bits: Map.delete(policy.use_bits, frame),
          hand: new_hand
        }, frame}
      else
        # Second chance — clear the use bit and advance
        new_use_bits = Map.put(policy.use_bits, frame, false)
        new_hand = rem(policy.hand + 1, length(frames))

        do_select_victim(
          %{policy | use_bits: new_use_bits, hand: new_hand},
          remaining - 1
        )
      end
    end

    @doc "Add a new frame with use bit set."
    def add_frame(%__MODULE__{} = policy, frame) do
      %{policy |
        frames: policy.frames ++ [frame],
        use_bits: Map.put(policy.use_bits, frame, true)
      }
    end

    @doc "Remove a frame from tracking."
    def remove_frame(%__MODULE__{} = policy, frame) do
      idx = Enum.find_index(policy.frames, &(&1 == frame))

      if idx do
        new_frames = List.delete_at(policy.frames, idx)

        new_hand =
          cond do
            length(new_frames) == 0 -> 0
            idx < policy.hand -> rem(policy.hand - 1, length(new_frames))
            true -> rem(policy.hand, length(new_frames))
          end

        %{policy |
          frames: new_frames,
          use_bits: Map.delete(policy.use_bits, frame),
          hand: new_hand
        }
      else
        policy
      end
    end
  end

  # ============================================================================
  # MMU (Memory Management Unit)
  # ============================================================================

  defmodule MMU do
    @moduledoc """
    The MMU is the central component that ties everything together:

    - Page tables: one per process, mapping virtual to physical addresses.
    - TLB: shared translation cache for fast lookups.
    - Frame allocator: tracks which physical frames are free.
    - Replacement policy: decides which page to evict when memory is full.
    - Reference counts: tracks shared frames for copy-on-write.

    Every memory access goes through the MMU's translate() function.
    """
    import Bitwise

    @page_offset_bits 12
    @page_offset_mask (1 <<< @page_offset_bits) - 1

    defstruct page_tables: %{},
              tlb: nil,
              frame_allocator: nil,
              policy: nil,
              policy_type: :lru,
              frame_refcounts: %{},
              current_pid: nil

    @type t :: %__MODULE__{
            page_tables: %{non_neg_integer() => TwoLevelPageTable.t()},
            tlb: TLB.t(),
            frame_allocator: PhysicalFrameAllocator.t(),
            policy: FIFOPolicy.t() | LRUPolicy.t() | ClockPolicy.t(),
            policy_type: :fifo | :lru | :clock,
            frame_refcounts: %{non_neg_integer() => non_neg_integer()},
            current_pid: non_neg_integer() | nil
          }

    @doc "Create a new MMU with the given number of frames and replacement policy."
    def new(total_frames, policy_type \\ :lru, tlb_capacity \\ 64) do
      policy =
        case policy_type do
          :fifo -> FIFOPolicy.new()
          :lru -> LRUPolicy.new()
          :clock -> ClockPolicy.new()
        end

      %__MODULE__{
        tlb: TLB.new(tlb_capacity),
        frame_allocator: PhysicalFrameAllocator.new(total_frames),
        policy: policy,
        policy_type: policy_type
      }
    end

    @doc "Create a new, empty address space for a process."
    def create_address_space(%__MODULE__{} = mmu, pid) do
      %{mmu | page_tables: Map.put(mmu.page_tables, pid, TwoLevelPageTable.new())}
    end

    @doc "Destroy a process's address space, freeing all owned frames."
    def destroy_address_space(%__MODULE__{} = mmu, pid) do
      case Map.get(mmu.page_tables, pid) do
        nil ->
          mmu

        pt ->
          mappings = TwoLevelPageTable.all_mappings(pt)

          mmu =
            Enum.reduce(mappings, mmu, fn {_vaddr, pte}, acc ->
              if pte.present do
                decrement_refcount(acc, pte.frame_number)
              else
                acc
              end
            end)

          %{mmu | page_tables: Map.delete(mmu.page_tables, pid)}
      end
    end

    @doc """
    Map a virtual page to a newly allocated physical frame.
    Returns {updated_mmu, frame_number} or {mmu, nil} if out of memory.
    """
    def map_page(%__MODULE__{} = mmu, pid, vaddr, flags \\ []) do
      pt = Map.get(mmu.page_tables, pid)

      if pt == nil do
        raise "No address space for PID #{pid}"
      end

      {mmu, frame} = allocate_frame(mmu)

      case frame do
        nil ->
          {mmu, nil}

        frame_num ->
          pt = TwoLevelPageTable.map(pt, vaddr, frame_num, flags)
          mmu = %{mmu | page_tables: Map.put(mmu.page_tables, pid, pt)}
          mmu = set_refcount(mmu, frame_num, 1)
          mmu = update_policy(mmu, :add_frame, frame_num)
          {mmu, frame_num}
      end
    end

    @doc """
    Translate a virtual address to a physical address.

    Steps:
    1. Check TLB (fast path).
    2. On miss, walk the page table (slow path).
    3. On page fault, allocate a frame (demand paging).
    4. Update accessed/dirty bits.
    5. Cache in TLB.
    """
    def translate(%__MODULE__{} = mmu, pid, vaddr, is_write \\ false) do
      addr = vaddr &&& 0xFFFFFFFF
      vpn = addr >>> @page_offset_bits
      page_offset = addr &&& @page_offset_mask

      # Step 1: Check TLB
      {tlb, tlb_result} = TLB.lookup(mmu.tlb, pid, vpn)
      mmu = %{mmu | tlb: tlb}

      case tlb_result do
        {frame, pte} ->
          # TLB hit
          if is_write and not pte.writable do
            # COW fault
            handle_cow_or_fault(mmu, pid, vaddr, is_write)
          else
            pte = %{pte | accessed: true}
            pte = if is_write, do: %{pte | dirty: true}, else: pte

            # Update TLB entry with new PTE state
            tlb = TLB.insert(mmu.tlb, pid, vpn, frame, pte)
            mmu = %{mmu | tlb: tlb}

            # Update the PTE in the page table too
            mmu =
              case Map.get(mmu.page_tables, pid) do
                nil -> mmu
                pt ->
                  updated_pt = TwoLevelPageTable.update_pte(pt, addr, fn _old -> pte end)
                  %{mmu | page_tables: Map.put(mmu.page_tables, pid, updated_pt)}
              end

            mmu = update_policy(mmu, :record_access, frame)
            phys = (frame <<< @page_offset_bits) ||| page_offset
            {mmu, phys}
          end

        nil ->
          # TLB miss — walk page table
          pt = Map.get(mmu.page_tables, pid)

          if pt == nil do
            raise "No address space for PID #{pid}"
          end

          case TwoLevelPageTable.translate(pt, addr) do
            nil ->
              # Page fault
              handle_page_fault(mmu, pid, vaddr)

            {_phys, pte} ->
              if is_write and not pte.writable do
                handle_cow_or_fault(mmu, pid, vaddr, is_write)
              else
                pte = %{pte | accessed: true}
                pte = if is_write, do: %{pte | dirty: true}, else: pte

                # Update PTE in page table and cache in TLB
                pt = TwoLevelPageTable.update_pte(pt, addr, fn _old -> pte end)
                updated_mmu = %{mmu | page_tables: Map.put(mmu.page_tables, pid, pt)}
                tlb = TLB.insert(updated_mmu.tlb, pid, vpn, pte.frame_number, pte)
                mmu = update_policy(%{updated_mmu | tlb: tlb}, :record_access, pte.frame_number)

                phys = (pte.frame_number <<< @page_offset_bits) ||| page_offset
                {mmu, phys}
              end
          end
      end
    end

    @doc "Handle a page fault by allocating a frame and mapping the page."
    def handle_page_fault(%__MODULE__{} = mmu, pid, vaddr) do
      addr = vaddr &&& 0xFFFFFFFF
      page_offset = addr &&& @page_offset_mask

      {mmu, frame} = allocate_frame(mmu)

      case frame do
        nil ->
          raise "Out of memory: no frames available and no pages to evict"

        frame_num ->
          pt = Map.get(mmu.page_tables, pid)

          if pt == nil do
            raise "No address space for PID #{pid}"
          end

          pt = TwoLevelPageTable.map(pt, addr, frame_num, writable: true, user_accessible: true)
          pt = TwoLevelPageTable.update_pte(pt, addr, fn pte -> %{pte | accessed: true} end)
          mmu = %{mmu | page_tables: Map.put(mmu.page_tables, pid, pt)}
          mmu = set_refcount(mmu, frame_num, 1)
          mmu = update_policy(mmu, :add_frame, frame_num)

          # Cache in TLB
          vpn = addr >>> @page_offset_bits
          pte = TwoLevelPageTable.lookup_pte(pt, addr)
          tlb = TLB.insert(mmu.tlb, pid, vpn, frame_num, pte)
          mmu = %{mmu | tlb: tlb}

          phys = (frame_num <<< @page_offset_bits) ||| page_offset
          {mmu, phys}
      end
    end

    defp handle_cow_or_fault(%__MODULE__{} = mmu, pid, vaddr, _is_write) do
      addr = vaddr &&& 0xFFFFFFFF
      page_offset = addr &&& @page_offset_mask
      vpn = addr >>> @page_offset_bits

      pt = Map.get(mmu.page_tables, pid)

      if pt == nil do
        raise "No address space for PID #{pid}"
      end

      pte = TwoLevelPageTable.lookup_pte(pt, addr)

      if pte == nil or not pte.present do
        handle_page_fault(mmu, pid, vaddr)
      else
        refcount = get_refcount(mmu, pte.frame_number)

        if refcount > 1 do
          # COW: allocate new frame
          {mmu, new_frame} = allocate_frame(mmu)

          case new_frame do
            nil ->
              raise "Out of memory during COW fault"

            new_frame_num ->
              mmu = decrement_refcount(mmu, pte.frame_number)

              new_pte = %{pte |
                frame_number: new_frame_num,
                writable: true,
                dirty: true,
                accessed: true
              }

              pt = TwoLevelPageTable.update_pte(pt, addr, fn _old -> new_pte end)
              mmu = %{mmu | page_tables: Map.put(mmu.page_tables, pid, pt)}
              mmu = set_refcount(mmu, new_frame_num, 1)
              mmu = update_policy(mmu, :add_frame, new_frame_num)

              # Update TLB
              tlb = TLB.invalidate(mmu.tlb, pid, vpn)
              tlb = TLB.insert(tlb, pid, vpn, new_frame_num, new_pte)
              mmu = %{mmu | tlb: tlb}
              mmu = update_policy(mmu, :record_access, new_frame_num)

              phys = (new_frame_num <<< @page_offset_bits) ||| page_offset
              {mmu, phys}
          end
        else
          # Sole owner — make writable
          new_pte = %{pte | writable: true, dirty: true, accessed: true}
          pt = TwoLevelPageTable.update_pte(pt, addr, fn _old -> new_pte end)
          mmu = %{mmu | page_tables: Map.put(mmu.page_tables, pid, pt)}

          tlb = TLB.invalidate(mmu.tlb, pid, vpn)
          tlb = TLB.insert(tlb, pid, vpn, new_pte.frame_number, new_pte)
          mmu = %{mmu | tlb: tlb}
          mmu = update_policy(mmu, :record_access, new_pte.frame_number)

          phys = (new_pte.frame_number <<< @page_offset_bits) ||| page_offset
          {mmu, phys}
        end
      end
    end

    @doc """
    Clone an address space from one process to another (fork with COW).

    Shares physical frames between parent and child, marking all pages as
    read-only. When either writes, a COW fault triggers a private copy.
    """
    def clone_address_space(%__MODULE__{} = mmu, from_pid, to_pid) do
      src_pt = Map.get(mmu.page_tables, from_pid)

      if src_pt == nil do
        raise "No address space for PID #{from_pid}"
      end

      mappings = TwoLevelPageTable.all_mappings(src_pt)

      # Create destination page table and process all mappings
      {mmu, dst_pt, src_pt} =
        Enum.reduce(mappings, {mmu, TwoLevelPageTable.new(), src_pt}, fn
          {vaddr, pte}, {mmu_acc, dst_pt_acc, src_pt_acc} when pte.present ->
            # Mark source as read-only
            src_pt_acc = TwoLevelPageTable.update_pte(src_pt_acc, vaddr, fn p ->
              %{p | writable: false}
            end)

            # Create child mapping (also read-only)
            dst_pt_acc = TwoLevelPageTable.map(dst_pt_acc, vaddr, pte.frame_number,
              executable: pte.executable,
              user_accessible: pte.user_accessible
            )

            dst_pt_acc = TwoLevelPageTable.update_pte(dst_pt_acc, vaddr, fn p ->
              %{p | writable: false, dirty: pte.dirty, accessed: pte.accessed}
            end)

            # Increment refcount
            mmu_acc = increment_refcount(mmu_acc, pte.frame_number)

            # Invalidate parent TLB entry
            vpn = (vaddr &&& 0xFFFFFFFF) >>> @page_offset_bits
            mmu_acc = %{mmu_acc | tlb: TLB.invalidate(mmu_acc.tlb, from_pid, vpn)}

            {mmu_acc, dst_pt_acc, src_pt_acc}

          {_vaddr, _pte}, acc ->
            acc
        end)

      mmu = %{mmu |
        page_tables: mmu.page_tables
          |> Map.put(from_pid, src_pt)
          |> Map.put(to_pid, dst_pt)
      }

      mmu
    end

    @doc """
    Context switch: flush the TLB and set the current PID.
    The TLB is flushed because it contains translations for the old process.
    """
    def context_switch(%__MODULE__{} = mmu, new_pid) do
      %{mmu | tlb: TLB.flush(mmu.tlb), current_pid: new_pid}
    end

    @doc "Get the page table for a process."
    def get_page_table(%__MODULE__{} = mmu, pid) do
      Map.get(mmu.page_tables, pid)
    end

    # ---- Private helpers ----

    defp allocate_frame(%__MODULE__{} = mmu) do
      {alloc, frame} = PhysicalFrameAllocator.allocate(mmu.frame_allocator)
      mmu = %{mmu | frame_allocator: alloc}

      case frame do
        nil ->
          # Try eviction
          evict_page(mmu)

        _ ->
          {mmu, frame}
      end
    end

    defp evict_page(%__MODULE__{} = mmu) do
      {policy, victim} = apply_policy_select(mmu.policy, mmu.policy_type)
      mmu = %{mmu | policy: policy}

      case victim do
        nil ->
          {mmu, nil}

        victim_frame ->
          # Find and unmap the victim
          {mmu, _found} =
            Enum.reduce_while(mmu.page_tables, {mmu, false}, fn {pid, pt}, {mmu_acc, _} ->
              case find_frame_in_pt(pt, victim_frame) do
                nil ->
                  {:cont, {mmu_acc, false}}

                vaddr ->
                  vpn = (vaddr &&& 0xFFFFFFFF) >>> @page_offset_bits
                  pt = TwoLevelPageTable.update_pte(pt, vaddr, fn pte ->
                    %{pte | present: false}
                  end)
                  mmu_acc = %{mmu_acc |
                    page_tables: Map.put(mmu_acc.page_tables, pid, pt),
                    tlb: TLB.invalidate(mmu_acc.tlb, pid, vpn)
                  }
                  mmu_acc = %{mmu_acc | frame_refcounts: Map.delete(mmu_acc.frame_refcounts, victim_frame)}
                  {:halt, {mmu_acc, true}}
              end
            end)

          {mmu, victim_frame}
      end
    end

    defp find_frame_in_pt(pt, target_frame) do
      pt
      |> TwoLevelPageTable.all_mappings()
      |> Enum.find_value(fn {vaddr, pte} ->
        if pte.frame_number == target_frame and pte.present, do: vaddr
      end)
    end

    defp apply_policy_select(policy, :fifo), do: FIFOPolicy.select_victim(policy)
    defp apply_policy_select(policy, :lru), do: LRUPolicy.select_victim(policy)
    defp apply_policy_select(policy, :clock), do: ClockPolicy.select_victim(policy)

    defp update_policy(%__MODULE__{} = mmu, :add_frame, frame) do
      policy =
        case mmu.policy_type do
          :fifo -> FIFOPolicy.add_frame(mmu.policy, frame)
          :lru -> LRUPolicy.add_frame(mmu.policy, frame)
          :clock -> ClockPolicy.add_frame(mmu.policy, frame)
        end

      %{mmu | policy: policy}
    end

    defp update_policy(%__MODULE__{} = mmu, :record_access, frame) do
      policy =
        case mmu.policy_type do
          :fifo -> FIFOPolicy.record_access(mmu.policy, frame)
          :lru -> LRUPolicy.record_access(mmu.policy, frame)
          :clock -> ClockPolicy.record_access(mmu.policy, frame)
        end

      %{mmu | policy: policy}
    end

    defp update_policy(%__MODULE__{} = mmu, :remove_frame, frame) do
      policy =
        case mmu.policy_type do
          :fifo -> FIFOPolicy.remove_frame(mmu.policy, frame)
          :lru -> LRUPolicy.remove_frame(mmu.policy, frame)
          :clock -> ClockPolicy.remove_frame(mmu.policy, frame)
        end

      %{mmu | policy: policy}
    end

    defp get_refcount(%__MODULE__{} = mmu, frame) do
      Map.get(mmu.frame_refcounts, frame, 0)
    end

    defp set_refcount(%__MODULE__{} = mmu, frame, count) do
      %{mmu | frame_refcounts: Map.put(mmu.frame_refcounts, frame, count)}
    end

    defp increment_refcount(%__MODULE__{} = mmu, frame) do
      set_refcount(mmu, frame, get_refcount(mmu, frame) + 1)
    end

    defp decrement_refcount(%__MODULE__{} = mmu, frame) do
      count = get_refcount(mmu, frame) - 1

      if count <= 0 do
        mmu = %{mmu | frame_refcounts: Map.delete(mmu.frame_refcounts, frame)}

        if PhysicalFrameAllocator.is_allocated(mmu.frame_allocator, frame) do
          alloc = PhysicalFrameAllocator.free(mmu.frame_allocator, frame)
          mmu = %{mmu | frame_allocator: alloc}
          update_policy(mmu, :remove_frame, frame)
        else
          mmu
        end
      else
        set_refcount(mmu, frame, count)
      end
    end
  end
end
