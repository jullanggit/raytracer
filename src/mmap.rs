use std::{
    ffi::{c_int, c_long, c_ulong, c_void},
    fs::File,
    io::Error,
    os::fd::AsRawFd as _,
    ptr, slice,
};

// sys/mman.h
const MAP_FAILED: *mut c_void = usize::MAX as *mut c_void; // (void *) -1
// bits/mman-linux.h
const MAP_SHARED: c_int = 1;
const PROT_READ: c_int = 1;
const PROT_WRITE: c_int = 2;

// sys/mman.h
unsafe extern "C" {
    // void *mmap (void *__addr, size_t __len, int __prot, int __flags, int __fd, __off_t __offset) __THROW
    fn mmap(
        addr: *mut c_void,
        len: c_ulong,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: c_long,
    ) -> *mut c_void;
    // int munmap (void *__addr, size_t __len) __THROW
    fn munmap(add: *mut c_void, len: c_ulong) -> c_int;
}

struct MmapFile {
    ptr: *mut u8,
    len: usize,
}
impl MmapFile {
    pub fn new(path: &str, len: usize) -> Self {
        let file = File::create(path).unwrap();

        // SAFETY:
        // - addr = ptr::null_mut() -> OS chooses address
        // - prot & flags are valid flags
        // - fd is a valid file descriptor for the duration of the call
        let ptr = unsafe {
            mmap(
                ptr::null_mut(),
                len as u64,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };

        assert!(
            !ptr::eq(ptr, MAP_FAILED),
            "Error: {}",
            Error::last_os_error()
        );

        Self {
            ptr: ptr.cast(),
            len,
        }
    }
    pub const fn as_slice_mut(&mut self) -> &mut [u8] {
        // SAFETY:
        // - ptr is a valid pointer to memory managed by the OS
        // - len is both the length in bytes and the amount of elements
        unsafe { slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}
impl Drop for MmapFile {
    fn drop(&mut self) {
        // SAFETY:
        // - addr is the pointer returned by mmap()
        // - len is the same length used for mmap()
        let res = unsafe { munmap(self.ptr.cast(), self.len as u64) };
        assert!(res == 0, "Error: {}", Error::last_os_error());
    }
}
