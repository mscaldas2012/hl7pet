//! Standalone counting + byte-tracking `#[global_allocator]` for the bench
//! binaries (spec 009 research.md #3). Deliberately separate from
//! `src/test_alloc.rs`, which stays `cfg(test)`-only and unchanged — each
//! bench target is its own binary and needs its own allocator anyway (only
//! one `#[global_allocator]` may exist per binary).
//!
//! Tracks two figures, mirroring `test_alloc.rs`'s call-counting design plus
//! one addition: total bytes allocated, comparable to the Scala side's
//! `gc.alloc.rate.norm` (bytes/op) — `test_alloc.rs` only ever needed call
//! count, since nothing on its side of the comparison exists yet.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ALLOC_COUNT: Cell<u64> = const { Cell::new(0) };
    static ALLOC_BYTES: Cell<u64> = const { Cell::new(0) };
}

pub struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.with(|c| c.set(c.get() + 1));
        ALLOC_BYTES.with(|b| b.set(b.get() + layout.size() as u64));
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Counted as a fresh allocation-equivalent of `new_size` bytes, not a
        // delta -- matches how a JVM array-growth reallocation (e.g. a
        // StringBuilder/ArrayList resize) shows up in gc.alloc.rate.norm as
        // the new array's full size, not the size difference.
        ALLOC_COUNT.with(|c| c.set(c.get() + 1));
        ALLOC_BYTES.with(|b| b.set(b.get() + new_size as u64));
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Allocation counters observed during one measured call.
#[derive(Debug, Clone, Copy, Default)]
pub struct AllocStats {
    pub call_count: u64,
    pub bytes: u64,
}

/// Runs `f`, returning its result alongside the allocation activity observed
/// strictly during `f`'s execution.
pub fn measure<T>(f: impl FnOnce() -> T) -> (T, AllocStats) {
    // Touch both thread-locals once outside the measured window so any
    // first-access lazy initialization doesn't get counted.
    ALLOC_COUNT.with(|c| c.get());
    ALLOC_BYTES.with(|b| b.get());

    let before_count = ALLOC_COUNT.with(|c| c.get());
    let before_bytes = ALLOC_BYTES.with(|b| b.get());
    let result = f();
    let stats = AllocStats {
        call_count: ALLOC_COUNT.with(|c| c.get()) - before_count,
        bytes: ALLOC_BYTES.with(|b| b.get()) - before_bytes,
    };
    (result, stats)
}
