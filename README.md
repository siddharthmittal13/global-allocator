# global-allocator

A tiny, dependency-free building block for defining a **custom global allocator**
in any Rust crate.

It provides [`CustomAllocator`], a thin [`GlobalAlloc`] wrapper that forwards
every allocation call to an inner allocator (the standard library's `System`
allocator by default). Use it as-is, or as a base you extend to add your own
behavior — logging, metrics, pooling, arenas — without reimplementing the
platform allocation logic.

## Why

Writing a `#[global_allocator]` from scratch means correctly implementing four
`unsafe` methods and getting every invariant right. `CustomAllocator` gives you
a correct, minimal starting point that you can wrap or compose.

## Usage

Add it to your `Cargo.toml`:

```toml
[dependencies]
global-allocator = "0.1"
```

Install it as the global allocator:

```rust
use global_allocator::CustomAllocator;

#[global_allocator]
static GLOBAL: CustomAllocator = CustomAllocator::system();

fn main() {
    let v = vec![1, 2, 3]; // allocated through CustomAllocator
    println!("{v:?}");
}
```

Wrap any other allocator (for example in `no_std`, supply your own backend):

```rust
use global_allocator::CustomAllocator;

# struct MyAllocator;
# unsafe impl core::alloc::GlobalAlloc for MyAllocator {
#     unsafe fn alloc(&self, _: core::alloc::Layout) -> *mut u8 { core::ptr::null_mut() }
#     unsafe fn dealloc(&self, _: *mut u8, _: core::alloc::Layout) {}
# }
#[global_allocator]
static GLOBAL: CustomAllocator<MyAllocator> = CustomAllocator::new(MyAllocator);
```

## Extending it

Because `CustomAllocator<A>` implements `GlobalAlloc` for any inner `A`, you can
build your own allocator by wrapping it and forwarding to it:

```rust
use core::alloc::{GlobalAlloc, Layout};
use global_allocator::CustomAllocator;

struct Logging(CustomAllocator);

unsafe impl GlobalAlloc for Logging {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // ... your logic here ...
        unsafe { self.0.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(ptr, layout) }
    }
}
```

## `no_std`

The crate is `no_std` compatible. The `std` feature (enabled by default) adds the
`System`-backed constructors `CustomAllocator::system()` and the
`CustomAllocator<System>` default. Disable it to use the crate in a `no_std`
context and provide your own inner allocator:

```toml
[dependencies]
global-allocator = { version = "0.1", default-features = false }
```

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your
option.
