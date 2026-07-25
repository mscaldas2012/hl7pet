//! Shared counting-allocator test harness (crate-free, `cfg(test)` only).
//!
//! Only one `#[global_allocator]` may exist per test binary, and `cargo test
//! --lib` links every `src/*.rs` module's unit tests into one binary — so
//! this lives here once, and both `scanner.rs` (spec 005 SC-004) and
//! `parser.rs` (spec 006 SC-004) call [`count_allocs`] rather than each
//! declaring their own.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ALLOC_COUNT: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOC_COUNT.with(|c| c.set(c.get() + 1));
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOC_COUNT.with(|c| c.set(c.get() + 1));
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

pub(crate) fn count_allocs(f: impl FnOnce()) -> usize {
    ALLOC_COUNT.with(|c| c.get()); // touch the thread-local once so first-access lazy init happens outside the measured window
    let before = ALLOC_COUNT.with(|c| c.get());
    f();
    ALLOC_COUNT.with(|c| c.get()) - before
}
