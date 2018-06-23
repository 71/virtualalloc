#![feature(core_intrinsics)]

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(windows), feature(libc))]
#![cfg_attr(feature = "allocator", feature(allocator_api, alloc))]

#[cfg(windows)]
extern crate kernel32;

#[cfg(all(feature = "allocator", not(feature = "std")))]
extern crate core;

#[cfg(feature = "allocator")]
mod alloc;
#[cfg(feature = "std")]
mod vec;

#[cfg(feature = "allocator")]
pub use alloc::VirtualAlloc;
#[cfg(feature = "std")]
pub use vec::VirtualVec;
