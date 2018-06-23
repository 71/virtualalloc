
use std::io;
use std::mem;
use std::ptr;
use std::slice;
use std::intrinsics;

#[cfg(windows)]      use kernel32;
#[cfg(not(windows))] use libc;

/// A vector that can grow lazily without invalidating pointers to its contents.
/// 
/// Furthermore, the protection of the memory it has allocated can be changed at
/// any time using the `set_protection` function.
/// 
/// # Safety
/// A `VirtualVec` must be initialized with a maximum size that **cannot** be exceeded.
/// 
/// Any attempt to grow the vector beyond this maximum capacity will result in a panic, except if
/// performed with `reserve`, which will instead return `Err`.
/// 
/// Even though this limitation exists, it is noteworthy that the vector grows lazily. As such,
/// it is **possible to set this initial maximum capacity to an extremely large number**, even
/// if it greatly exceeds the physical RAM available on a device.
/// 
/// # Note
/// Some operations that typically mutate the vector only require `&self`, since
/// the mutation cannot invalidate borrowed values.
pub struct VirtualVec<T: Sized> {
    ptr: *mut T,
    prot: u8,

    cap: usize,
    max: usize,
    len: usize
}

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
    unsafe {
        mem::transmute::<_, u8>(r)      ||
        mem::transmute::<_, u8>(w) << 1 ||
        mem::transmute::<_, u8>(x) << 2
    }
}

impl<T> VirtualVec<T> {
    #[cfg(windows)]
    fn init(max_size: usize, prot: u8) -> *mut T {
        unsafe {
            kernel32::VirtualAlloc(ptr::null_mut(), max_size as _, 0x00002000, prot as _) as _
        }
    }
    #[cfg(not(windows))]
    fn init(max_size: usize, prot: u8) -> *mut T {
        unsafe {
            libc::mmap(ptr::null(), max_size, 0x0, 0x22, -1, 0) as _
        }
    }

    #[cfg(windows)]
    fn grow(&self, needed: usize, prot: u8) -> bool {
        unsafe {
            kernel32::VirtualAlloc(self.ptr as _, needed as _, 0x00001000, prot as _) != ptr::null_mut()
        }
    }
    #[cfg(not(windows))]
    fn grow(&self, needed: usize, prot: u8) -> bool {
        unsafe {
            libc::mprotect(self.ptr, needed, prot as _) == 0
        }
    }

    #[inline]
    fn reserve_internal(&self, min: usize) -> bool {
        if min > self.max {
            return false
        }

        if self.cap < min {
            if !self.grow(min * mem::size_of::<T>(), self.prot as _) {
                return false
            }

            unsafe {
                (&mut *(self as *const Self as *mut Self)).cap = min;
            }
        }

        true
    }

    #[inline]
    pub(crate) fn reserve_or_panic(&self, min: usize) {
        unsafe {
            if !intrinsics::likely(self.reserve_internal(min)) {
                panic!("Unable to reserve the requested amount of memory.")
            }
        }
    }

    /// Creates a new `VirtualVec`, given its initial capacity, the maximum number of items
    /// it can store and the protection of the memory it allocates.
    #[inline]
    pub fn with_capacity_and_protection(max: usize, cap: usize,
                                        read: bool, write: bool, exec: bool) -> Self {
        let v = Self::with_protection(max, read, write, exec);

        v.reserve_or_panic(cap);
        v
    }

    /// Creates a new `VirtualVec`, given the maximum number of items it can store and the
    /// protection of the memory it allocates.
    #[inline]
    pub fn with_protection(max: usize, read: bool, write: bool, exec: bool) -> Self {
        let prot = get_protection(read, write, exec);

        VirtualVec {
            max, prot, len: 0, cap: 0,
            ptr: Self::init(max, prot)
        }
    }

    /// Creates a new read-only `VirtualVec`, given the maximum number of items it can store.
    pub fn new(max: usize) -> Self {
        VirtualVec::with_protection(max, true, false, false)
    }

    /// Creates a new `VirtualVec`, given its initial capacity and the maximum number of items
    /// it can store.
    pub fn with_capacity(max: usize, cap: usize) -> Self {
        VirtualVec::with_capacity_and_protection(max, cap, true, false, false)
    }

    /// Returns the current length of the vector.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns the current physical capacity of the vector.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap
    }

    /// Returns the absolute maximum capacity of the vector.
    #[inline]
    pub fn max_capacity(&self) -> usize {
        self.max
    }

    /// Reserves the given amount of physical memory.
    /// 
    /// # Errors
    /// This function will return `Ok`, unless either one of these conditions is true:
    /// - The amount of physical RAM remaining is insufficient.
    /// - The requested amount of memory to reserve is greater than the maximum capacity that
    ///   was chosen at initialization.
    #[inline]
    pub fn reserve(&self, min: usize) -> Result<(), ()> {
        if self.reserve_internal(min) {
            Ok(())
        } else {
            Err(())
        }
    }

    /// Sets the protection of the inner buffer.
    #[cfg(windows)]
    #[inline]
    pub fn set_protection(&mut self, read: bool, write: bool, exec: bool) {
        let prot = get_protection(read, write, exec);
        let mut old = 0;

        unsafe {
            kernel32::VirtualProtect(self.ptr as _, self.len as _, prot as _, &mut old);
        }

        self.prot = prot;
    }

    /// Sets the protection of the inner buffer.
    #[cfg(not(windows))]
    #[inline]
    pub fn set_protection(&mut self, read: bool, write: bool, exec: bool) {
        let prot = get_protection(read, write, exec);
        
        unsafe {
            libc::mprotect(self.ptr, self.len, prot as _);
        }

        self.prot = prot;
    }

    /// Returns the underlying pointer to the content of the vector.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    /// Returns the mutable underlying pointer to the content of the vector.
    #[inline]
    pub fn as_mut(&self) -> *mut T {
        self.ptr
    }

    /// Returns a slice to the contents of the vector.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe {
            slice::from_raw_parts(self.ptr, self.len)
        }
    }

    /// Returns a mutable slice to the contents of the vector.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            slice::from_raw_parts_mut(self.ptr, self.len)
        }
    }
}

impl<T> Default for VirtualVec<T> {
    /// Creates a `VirtualVec` that can store up to 1,000,000,000 elements.
    fn default() -> VirtualVec<T> {
        VirtualVec::new(1_000_000_000)
    }
}

impl io::Write for VirtualVec<u8> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let len = data.len();

        if self.reserve_internal(self.len() + len) {
            unsafe {
                ptr::copy_nonoverlapping(data.as_ptr(), self.ptr, len);
            }

            Ok(len)
        } else {
            Err(io::Error::new(io::ErrorKind::Other,
                               "Unable to reserve memory for write operation."))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<T> Drop for VirtualVec<T> {
    #[cfg(windows)]
    fn drop(&mut self) {
        unsafe {
            kernel32::VirtualFree(self.ptr as _, self.max as _, 0x8000);
        }
    }

    #[cfg(not(windows))]
    fn drop(&mut self) {
        unsafe {
            libc::munmap(self.ptr, self.max);
        }
    }
}