package = "coding-adventures-virtual-memory"
version = "0.1.0-1"
source = {
    url = "git://github.com/adhithyan15/coding-adventures.git",
}
description = {
    summary = "Virtual memory subsystem with paging, TLB, and page replacement",
    detailed = [[
        Full virtual memory subsystem implementing:
        - PageTableEntry with RISC-V Sv32 flag bits (present, dirty, accessed, etc.)
        - Single-level and two-level (Sv32) page tables
        - TLB with LRU eviction and hit/miss counters
        - Physical frame allocator with bitmap
        - Page replacement policies: FIFO, LRU, Clock (second-chance)
        - MMU with translate, page fault handling, COW fork (clone_address_space)
    ]],
    license = "MIT",
}
dependencies = {
    "lua >= 5.4",
}
build = {
    type = "builtin",
    modules = {
        ["coding_adventures.virtual_memory"] = "src/coding_adventures/virtual_memory/init.lua",
    },
}
