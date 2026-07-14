//! A tiny, dependency-free building block for defining a **custom global
//! allocator** in any Rust crate.
//!
//! The crate provides [`CustomAllocator`], a thin [`GlobalAlloc`] wrapper that
//! forwards every allocation call to an inner allocator. By default (with the
//! `std` feature enabled) the inner allocator is the standard library's
//! [`System`] allocator, so the wrapper behaves exactly like the default global
//! allocator while giving you a place to hook in custom behavior.
//!
//! # Examples
//!
//! Install it as the process global allocator:
//!
//! ```no_run
//! use global_allocator::CustomAllocator;
//!
//! #[global_allocator]
//! static GLOBAL: CustomAllocator = CustomAllocator::system();
//! ```
//!
//! Wrap any other allocator (useful in `no_std`, where you supply the backend):
//!
//! ```no_run
//! use core::alloc::{GlobalAlloc, Layout};
//! use global_allocator::CustomAllocator;
//!
//! struct MyAllocator;
//! unsafe impl GlobalAlloc for MyAllocator {
//!     unsafe fn alloc(&self, _: Layout) -> *mut u8 { core::ptr::null_mut() }
//!     unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
//! }
//!
//! #[global_allocator]
//! static GLOBAL: CustomAllocator<MyAllocator> = CustomAllocator::new(MyAllocator);
//! ```
//!
//! # `no_std`
//!
//! This crate is `no_std` compatible. The `std` feature (enabled by default)
//! adds the [`System`]-backed constructors. Disable default features to use it
//! in a `no_std` context and provide your own inner allocator.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_op_in_unsafe_fn)]

use core::alloc::{GlobalAlloc, Layout};

#[cfg(feature = "std")]
use std::alloc::System;

/// A minimal [`GlobalAlloc`] wrapper that forwards every operation to an inner
/// allocator `A`.
///
/// With the `std` feature enabled the inner allocator defaults to [`System`],
/// so `CustomAllocator` (i.e. `CustomAllocator<System>`) can be installed
/// directly as a `#[global_allocator]`. Any other type implementing
/// [`GlobalAlloc`] can be wrapped instead, which makes this type a convenient
/// base for building your own allocator without reimplementing the platform
/// allocation logic.
#[cfg(feature = "std")]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CustomAllocator<A = System> {
    inner: A,
}

/// A minimal [`GlobalAlloc`] wrapper that forwards every operation to an inner
/// allocator `A`.
///
/// In `no_std` builds there is no default inner allocator; supply your own via
/// [`CustomAllocator::new`].
#[cfg(not(feature = "std"))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CustomAllocator<A> {
    inner: A,
}

impl<A> CustomAllocator<A> {
    /// Create a `CustomAllocator` that wraps the given inner allocator.
    ///
    /// This is a `const fn`, so it can initialize a `#[global_allocator]`
    /// static.
    #[must_use]
    pub const fn new(inner: A) -> Self {
        Self { inner }
    }

    /// Get a shared reference to the wrapped inner allocator.
    #[must_use]
    pub const fn inner(&self) -> &A {
        &self.inner
    }

    /// Consume the wrapper and return the inner allocator.
    #[must_use]
    pub fn into_inner(self) -> A {
        self.inner
    }
}

#[cfg(feature = "std")]
impl CustomAllocator<System> {
    /// Create a `CustomAllocator` backed by the standard library [`System`]
    /// allocator.
    ///
    /// This is a `const fn`, so it can initialize a `#[global_allocator]`
    /// static.
    #[must_use]
    pub const fn system() -> Self {
        Self::new(System)
    }
}

// SAFETY: Every method forwards to `inner`, which is a correct `GlobalAlloc`
// implementation. The pointer handed to and returned by the caller is exactly
// the one produced by `inner`, so all `GlobalAlloc` invariants are preserved.
unsafe impl<A: GlobalAlloc> GlobalAlloc for CustomAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Upheld by the caller of the `GlobalAlloc` method.
        unsafe { self.inner.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Upheld by the caller of the `GlobalAlloc` method.
        unsafe { self.inner.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Upheld by the caller of the `GlobalAlloc` method.
        unsafe { self.inner.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: Upheld by the caller of the `GlobalAlloc` method.
        unsafe { self.inner.realloc(ptr, layout, new_size) }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_system_alloc_dealloc_roundtrip() {
        let allocator = CustomAllocator::system();
        let layout = Layout::from_size_align(64, 8).unwrap();

        // SAFETY: `layout` has a non-zero size; the block is freed below with the
        // same layout.
        unsafe {
            let ptr = allocator.alloc(layout);
            assert!(!ptr.is_null());
            ptr.write_bytes(0xAB, layout.size());
            allocator.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_alloc_zeroed_is_zeroed() {
        let allocator = CustomAllocator::system();
        let layout = Layout::from_size_align(32, 8).unwrap();

        // SAFETY: `layout` has a non-zero size; the block is freed below with the
        // same layout.
        unsafe {
            let ptr = allocator.alloc_zeroed(layout);
            assert!(!ptr.is_null());
            for i in 0..layout.size() {
                assert_eq!(*ptr.add(i), 0);
            }
            allocator.dealloc(ptr, layout);
        }
    }

    #[test]
    fn test_realloc_preserves_data() {
        let allocator = CustomAllocator::system();
        let layout = Layout::from_size_align(16, 8).unwrap();

        // SAFETY: `ptr` is allocated, grown and freed with matching layouts.
        unsafe {
            let ptr = allocator.alloc(layout);
            assert!(!ptr.is_null());
            *ptr = 42;
            let grown = allocator.realloc(ptr, layout, 128);
            assert!(!grown.is_null());
            assert_eq!(*grown, 42);
            let grown_layout = Layout::from_size_align(128, 8).unwrap();
            allocator.dealloc(grown, grown_layout);
        }
    }

    #[test]
    fn test_inner_accessors() {
        let allocator = CustomAllocator::system();
        // `System` is a zero-sized unit type; just exercise the accessors.
        let _: &System = allocator.inner();
        let _: System = allocator.into_inner();
    }
}
