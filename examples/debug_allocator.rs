//! A simple **debug allocator** built on top of [`CustomAllocator`].
//!
//! It wraps the `System`-backed [`CustomAllocator`] and prints a line on every
//! allocation and deallocation. Install it as the process `#[global_allocator]`
//! and every heap operation in the program is traced.
//!
//! Run it with:
//!
//! ```text
//! cargo run --example debug_allocator
//! ```

use core::alloc::{GlobalAlloc, Layout};
use global_allocator::CustomAllocator;

/// An allocator that logs every allocation / deallocation, then forwards the
/// actual work to the wrapped [`CustomAllocator`].
struct DebugAllocator {
    inner: CustomAllocator,
}

impl DebugAllocator {
    const fn new() -> Self {
        Self {
            inner: CustomAllocator::system(),
        }
    }
}

// SAFETY: Every method forwards to `self.inner`, a correct `GlobalAlloc`. The
// logging never touches the allocated memory or invalidates the pointers
// returned by `inner`.
unsafe impl GlobalAlloc for DebugAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: `layout` requirements are upheld by the caller.
        let ptr = unsafe { self.inner.alloc(layout) };
        eprintln!("[alloc]   {:>6} bytes @ {:p}", layout.size(), ptr);
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
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
}
