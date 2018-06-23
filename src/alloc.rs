
#[cfg(feature = "std")] use std::alloc::*;
#[cfg(feature = "std")] use std::ptr::*;

#[cfg(not(feature = "std"))] use core::alloc::*;
#[cfg(not(feature = "std"))] use core::ptr::*;

type Opaque = u8;

/// An allocator that allocates memory in large uncommited pools of memory,
/// which has the added benefit of preserving pointers when reallocating.
/// 
/// # Note
/// Even though pointers are preserved, most operations still require moves,
/// which doesn't necessarily make this allocator faster than others.
/// 
/// # Implementation
/// - On Windows, `VirtualAlloc`, `VirtualProtect` and `VirtualFree` are used.
/// - On Unix, `mmap`, `mprotect` and `munmap` are used.
pub struct VirtualAlloc;

// TODO

unsafe impl Alloc for VirtualAlloc {
    unsafe fn alloc(&mut self, layout: Layout) -> Result<NonNull<Opaque>, AllocErr> {

    }

    unsafe fn dealloc(&mut self, ptr: NonNull<Opaque>, layout: Layout) {

    }

    unsafe fn realloc(&mut self, ptr: NonNull<Opaque>, layout: Layout, new_size: usize)
        -> Result<NonNull<Opaque>, AllocErr> {
        ()
    }

    unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<NonNull<Opaque>, AllocErr> {

    }

    unsafe fn alloc_excess(&mut self, layout: Layout) -> Result<Excess, AllocErr> {

    }

    unsafe fn realloc_excess(&mut self, ptr: NonNull<Opaque>, layout: Layout, new_size: usize)
        -> Result<Excess, AllocErr> {
        ()
    }

    unsafe fn grow_in_place(&mut self, ptr: NonNull<Opaque>, layout: Layout, new_size: usize)
        -> Result<(), CannotReallocInPlace> {
        ()
    }

    fn alloc_one<T>(&mut self) -> Result<NonNull<T>, AllocErr> where Self: Sized
    {

    }

    unsafe fn dealloc_one<T>(&mut self, ptr: NonNull<T>) where Self: Sized
    {
    }
}
