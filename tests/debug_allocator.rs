//! Integration test for a debug allocator built on top of [`CustomAllocator`].
//!
//! Instead of installing the allocator globally (which would trace the whole
//! test harness), this drives a local `DebugAllocator` instance directly and
//! asserts that its allocation/deallocation bookkeeping is correct.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use global_allocator::CustomAllocator;

/// Counts allocations/deallocations and forwards to the wrapped allocator.
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

    fn allocations(&self) -> usize {
        self.allocations.load(Ordering::Relaxed)
    }

    fn deallocations(&self) -> usize {
        self.deallocations.load(Ordering::Relaxed)
    }

    fn bytes_in_use(&self) -> usize {
        self.bytes_in_use.load(Ordering::Relaxed)
    }
}

// SAFETY: All operations forward to `self.inner`, a correct `GlobalAlloc`; the
// bookkeeping only touches atomics.
unsafe impl GlobalAlloc for DebugAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: `layout` requirements are upheld by the caller.
        let ptr = unsafe { self.inner.alloc(layout) };
        if !ptr.is_null() {
            self.allocations.fetch_add(1, Ordering::Relaxed);
            self.bytes_in_use.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.deallocations.fetch_add(1, Ordering::Relaxed);
        self.bytes_in_use.fetch_sub(layout.size(), Ordering::Relaxed);
        // SAFETY: `ptr`/`layout` come from a matching `alloc` call.
        unsafe { self.inner.dealloc(ptr, layout) }
    }
}

#[test]
fn tracks_allocation_and_deallocation() {
    let allocator = DebugAllocator::new();
    let layout = Layout::from_size_align(128, 8).unwrap();

    assert_eq!(allocator.allocations(), 0);
    assert_eq!(allocator.deallocations(), 0);
    assert_eq!(allocator.bytes_in_use(), 0);

    // SAFETY: `layout` has a non-zero size and the block is freed below with the
    // same layout.
    unsafe {
        let ptr = allocator.alloc(layout);
        assert!(!ptr.is_null());

        assert_eq!(allocator.allocations(), 1);
        assert_eq!(allocator.deallocations(), 0);
        assert_eq!(allocator.bytes_in_use(), 128);

        // Data written through the pointer must survive untouched.
        ptr.write_bytes(0x5A, layout.size());
        assert_eq!(*ptr, 0x5A);

        allocator.dealloc(ptr, layout);
    }

    assert_eq!(allocator.allocations(), 1);
    assert_eq!(allocator.deallocations(), 1);
    assert_eq!(allocator.bytes_in_use(), 0);
}

#[test]
fn tracks_multiple_allocations() {
    let allocator = DebugAllocator::new();
    let layout = Layout::from_size_align(64, 8).unwrap();

    let mut ptrs = Vec::new();
    // SAFETY: each block uses `layout` and is freed below with the same layout.
    unsafe {
        for _ in 0..5 {
            let ptr = allocator.alloc(layout);
            assert!(!ptr.is_null());
            ptrs.push(ptr);
        }

        assert_eq!(allocator.allocations(), 5);
        assert_eq!(allocator.bytes_in_use(), 5 * 64);

        for ptr in ptrs {
            allocator.dealloc(ptr, layout);
        }
    }

    assert_eq!(allocator.deallocations(), 5);
    assert_eq!(allocator.bytes_in_use(), 0);
}
