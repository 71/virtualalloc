virtualalloc
============

An allocator that can grow without invalidating pointers to its contents.

This crate provides the `VirtualAlloc` struct, which implements `Alloc` and can
be used to allocate memory lazily.

Behind the scenes, `VirtualAlloc` uses
- `VirtualAlloc`, `VirtualProtect` and `VirtualFree` on Windows.
- `mmap`, `mprotect` and `munmap` everywhere else.

## Installation

Add the following code to `Cargo.toml`:
```toml
[dependencies]
virtualalloc = { git = "https://github.com/6A/virtualalloc" }
```

## Usage
```rust
use virtualalloc::VirtualAlloc;

// First, create an allocator.
// It must have a maximum capacity that can never be exceeded. This capacity can be
// extremely large, even if it exceeds the available physical memory.
//
// Here, an allocator of maximum capacity 500MB and of
// protection read-write-execute is created.
let allocator = VirtualAlloc::with_protection(500_000_000, true, true, true);

// Then, the allocator can be used to allocate read-write-memory of a maximum size
// of 500MB. However, like other allocators, the physical memory will be allocated
// lazily when it is needed.
//
// However, when it does grow, existing pointers will **not** be invalidated, and will
// still point to the same location.
//
// Therefore, using the VirtualAlloc allocator in a container such as RawVec will
// keep its contents at the same position, even as it grows.
let mut vec = VirtualVec::<u8>::with_capacity_in(1_000_000, allocator);

let initial_ptr = vec.ptr();

// Here, we can see reserving additional memory does not move the pointer:
vec.reserve(INITIAL_CAP, 500_000);
assert_eq!( vec.ptr(), initial_ptr );

// And here, we can see that reserving memory in place always succeeds.
assert!( ec.reserve_in_place(INITIAL_CAP, 500_000) );
assert_eq!( vec.ptr(), initial_ptr );

// However, it is impossible to reserve memory over 500MB, even if the
// machine has more than 500 available megabytes of RAM.
assert!(  vec.try_reserve(INITIAL_CAP, 500_000_000).is_err() );
assert!( !vec.reserve_in_place(INITIAL_CAP, 500_000_000)     );
```
