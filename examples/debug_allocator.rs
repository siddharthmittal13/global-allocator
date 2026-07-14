//! A simple **debug allocator** built on top of [`CustomAllocator`].
//!
//! It wraps the `System`-backed [`CustomAllocator`] and prints a line on every
//! allocation and deallocation, while also tracking counts and live bytes with
//! atomic counters. Install it as the process `#[global_allocator]` and every
//! heap operation in the program is traced.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example debug_allocator
//! ```

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use global_allocator::CustomAllocator;

/// An allocator that logs and counts every allocation / deallocation, then
/// forwards the actual work to the wrapped [`CustomAllocator`].
struct DebugAllocator {
    inner: CustomAllocator,
    allocations: AtomicUsize,
    deallocations: AtomicUsize,
    bytes_in_use: AtomicUsize,
}

impl DebugAllocator {
    const fn new() -> Self {
        Self {
            inner: CustomAllocator::system(),
            allocations: AtomicUsize::new(0),
            deallocations: AtomicUsize::new(0),
            bytes_in_use: AtomicUsize::new(0),
        }
    }
}

// SAFETY: Every method forwards to `self.inner`, a correct `GlobalAlloc`. The
// bookkeeping only reads/writes atomics and prints; it never touches the
// allocated memory or invalidates the pointers returned by `inner`.
unsafe impl GlobalAlloc for DebugAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: `layout` requirements are upheld by the caller.
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            self.allocations.fetch_add(1, Ordering::Relaxed);
            self.bytes_in_use.fetch_add(layout.size(), Ordering::Relaxed);
            // Using the libc `write`-free `eprintln!` is safe here: it does not
            // recursively allocate on any supported platform for short strings.
            eprintln!("[alloc]   {:>6} bytes @ {:p}", layout.size(), ptr);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        self.bytes_in_use.fetch_sub(layout.size(), Ordering::Relaxed);
        eprintln!("[dealloc] {:>6} bytes @ {:p}", layout.size(), ptr);
        // SAFETY: `ptr`/`layout` come from a matching `alloc` call.
        unsafe { self.inner.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: DebugAllocator = DebugAllocator::new();

fn main() {
    println!("Allocating a Vec...");
    let mut data: Vec<u32> = Vec::with_capacity(4);
    for i in 0..4 {
        data.push(i);
    }

    println!("Allocating a String...");
    let text = String::from("hello, custom allocator");

    println!("Vec = {data:?}");
    println!("String = {text:?}");

    // Force the heap allocations above to be freed before we read the totals.
    drop(data);
    drop(text);

    println!(
        "\nTotals: {} allocations, {} deallocations, {} bytes still in use",
        GLOBAL.allocations.load(Ordering::Relaxed),
        GLOBAL.deallocations.load(Ordering::Relaxed),
        GLOBAL.bytes_in_use.load(Ordering::Relaxed),
    );
}
