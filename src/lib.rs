#![feature(alloc, allocator_api, core_intrinsics)]

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(windows), feature(libc))]

#![cfg_attr(test, feature(plugin))]
#![cfg_attr(test, plugin(speculate))]

#[cfg(windows)]
extern crate kernel32;
#[cfg(not(windows))]
extern crate libc;

#[cfg(not(feature = "std"))]
extern crate core;

#[cfg(test)]
extern crate alloc;


#[cfg(feature = "std")] use std::alloc::*;
#[cfg(feature = "std")] use std::intrinsics;
#[cfg(feature = "std")] use std::ptr::{self, NonNull};

#[cfg(not(feature = "std"))] use core::alloc::*;
#[cfg(not(feature = "std"))] use core::intrinsics;
#[cfg(not(feature = "std"))] use core::ptr::{self, NonNull};



#[cfg(windows)]
#[inline]
fn get_protection(r: bool, w: bool, x: bool) -> u8 {
    match (r, w, x) {
        (true, true, true)   => 0x40,
        (true, false, true)  => 0x20,
        (false, false, true) => 0x10,
        (true, true, false)  => 0x04,
        (true, false, false) => 0x02,

        _ => panic!("Invalid protection requested.")
    }
}

#[cfg(not(windows))]
#[inline]
fn get_protection(r: bool, w: bool, x: bool) -> u8 {
    #[cfg(feature = "std")]      use std::mem;
    #[cfg(not(feature = "std"))] use core::mem;

    unsafe {
        mem::transmute::<_, u8>(r)      |
        mem::transmute::<_, u8>(w) << 1 |
        mem::transmute::<_, u8>(x) << 2
    }
}

type Opaque = u8;

/// An allocator that allocates memory in large uncommited pools of memory,
/// which has the added benefit of preserving pointers when reallocating.
/// 
/// Once initialized, all allocations will use the specified protection
/// and maximum size.
/// 
/// # Note
/// Even though pointers are preserved, most operations still require moves,
/// which doesn't necessarily make this allocator faster than others.
/// 
/// # Implementation
/// - On Windows, `VirtualAlloc`, `VirtualProtect` and `VirtualFree` are used.
/// - On Unix, `mmap`, `mprotect` and `munmap` are used.
pub struct VirtualAlloc {
    max: usize,
    prot: u8
}

impl Default for VirtualAlloc {
    /// Returns a `VirtualAlloc` that can allocate up to 500GB of read-write memory.
    fn default() -> Self {
        VirtualAlloc { max: 500_000_000_000, prot: get_protection(true, true, false) }
    }
}

impl VirtualAlloc {
    /// Returns a `VirtualAlloc` that can allocate up to `max` bytes of read-write memory.
    pub fn new(max: usize) -> Self {
        VirtualAlloc { max, prot: get_protection(true, true, false) }
    }

    /// Returns a `VirtualAlloc` that can allocate up to `max` bytes of memory.
    pub fn with_protection(max: usize, read: bool, write: bool, exec: bool) -> Self {
        VirtualAlloc { max, prot: get_protection(read, write, exec) }
    }

    /// Returns the absolute maximum capacity of the vector.
    #[inline]
    pub fn max_capacity(&self) -> usize {
        self.max
    }

    /// Sets the protection of an allocated buffer.
    #[cfg(windows)]
    #[inline]
    pub fn set_protection<T: ?Sized>(ptr: NonNull<T>, len: usize,
                                     read: bool, write: bool, exec: bool) {
        let prot = get_protection(read, write, exec);
        let mut old = 0;

        unsafe {
            kernel32::VirtualProtect(ptr.as_ptr() as _, len as _, prot as _, &mut old);
        }
    }

    /// Sets the protection of an allocated buffer.
    #[cfg(not(windows))]
    #[inline]
    pub fn set_protection<T: ?Sized>(ptr: NonNull<T>, len: usize,
                                     read: bool, write: bool, exec: bool) {
        let prot = get_protection(read, write, exec);
        
        unsafe {
            libc::mprotect(ptr.as_ptr() as _, len, prot as _);
        }
    }

    #[cfg(windows)]
    fn init(max_size: usize, prot: u8) -> *mut Opaque {
        unsafe {
            kernel32::VirtualAlloc(ptr::null_mut(), max_size as _, 0x00002000, prot as _) as _
        }
    }
    #[cfg(not(windows))]
    fn init(max_size: usize, _: u8) -> *mut Opaque {
        unsafe {
            libc::mmap(ptr::null_mut(), max_size, 0x0, 0x22, -1, 0) as _
        }
    }

    #[cfg(windows)]
    fn grow(&self, ptr: *mut Opaque, needed: usize, prot: u8) -> bool {
        unsafe {
            kernel32::VirtualAlloc(ptr as _, needed as _, 0x00001000, prot as _) != ptr::null_mut()
        }
    }
    #[cfg(not(windows))]
    fn grow(&self, ptr: *mut Opaque, needed: usize, prot: u8) -> bool {
        unsafe {
            libc::mprotect(ptr as _, needed, prot as _) == 0
        }
    }

    #[inline]
    unsafe fn reserve_internal(&self, ptr: *mut Opaque, min: usize) -> bool {
        intrinsics::likely(min <= self.max) &&
        intrinsics::likely(self.grow(ptr, min, self.prot as _))
    }
}

unsafe impl Alloc for VirtualAlloc {
    unsafe fn alloc(&mut self, _: Layout) -> Result<NonNull<Opaque>, AllocErr> {
        match NonNull::new(Self::init(self.max, self.prot)) {
            Some(ptr) => Ok(ptr),
            None => Err(AllocErr)
        }
    }

    unsafe fn alloc_zeroed(&mut self, layout: Layout) -> Result<NonNull<Opaque>, AllocErr> {
        // VirtualAlloc automatically initializes to zero on Windows.
        // mmap automatically initializes to zero on Unix with MAP_ANONYMOUS (which we use).
        self.alloc(layout)
    }

    #[cfg(windows)]
    unsafe fn dealloc(&mut self, ptr: NonNull<Opaque>, _: Layout) {
        kernel32::VirtualFree(ptr.as_ptr() as _, self.max as _, 0x8000);
    }

    #[cfg(not(windows))]
    unsafe fn dealloc(&mut self, ptr: NonNull<Opaque>, _: Layout) {
        libc::munmap(ptr.as_ptr() as _, self.max);
    }

    unsafe fn realloc(&mut self, ptr: NonNull<Opaque>, _: Layout, new_size: usize)
        -> Result<NonNull<Opaque>, AllocErr> {
        // Grow in place directly
        if self.reserve_internal(ptr.as_ptr(), new_size) {
            Ok(ptr)
        } else {
            Err(AllocErr)
        }
    }

    unsafe fn grow_in_place(&mut self, ptr: NonNull<Opaque>, _: Layout, new_size: usize)
        -> Result<(), CannotReallocInPlace> {
        if self.reserve_internal(ptr.as_ptr(), new_size) {
            Ok(())
        } else {
            Err(CannotReallocInPlace)
        }
    }
}

#[cfg(test)]
speculate! {
    use alloc::raw_vec::RawVec;
    
    type VirtualVec<T> = RawVec<T, VirtualAlloc>;

    describe "never-changing pointer" {
        const INITIAL_CAP: usize = 1_000;
        const MAX_CAP: usize = 1_000_000;

        before {
            let allocator = VirtualAlloc::new(MAX_CAP);
            let mut vec = VirtualVec::<u8>::with_capacity_in(INITIAL_CAP, allocator);

            let initial_ptr = vec.ptr();
        }

        it "can double in place implicitely" {
            vec.double();
        }

        it "can double in place explicitely" {
            assert!(vec.double_in_place());
        }

        it "can reserve in place implicitely" {
            vec.reserve(INITIAL_CAP, 500_000);
        }

        it "can reserve in place explicitely" {
            assert!(vec.reserve_in_place(INITIAL_CAP, 500_000));
        }

        it "can't reserve values over maximum implicitely" {
            assert!(vec.try_reserve(INITIAL_CAP, MAX_CAP * 2).is_err());
        }

        it "can't reserve values over maximum explicitely" {
            assert!(!vec.reserve_in_place(INITIAL_CAP, MAX_CAP * 2));
        }

        after {
            assert_eq!(vec.ptr(), initial_ptr);
        }
    }
}
