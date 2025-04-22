use std::{
    ffi::{c_int, c_ulong},
    io::Error,
};

const CPU_SETSIZE: usize = 1024;
const NCPUBITS: usize = c_ulong::BITS as usize;

#[repr(C)]
struct cpu_set_t {
    #[expect(clippy::integer_division)]
    cpu_mask: [c_ulong; CPU_SETSIZE / NCPUBITS],
}
impl cpu_set_t {
    const fn zero() -> Self {
        Self { cpu_mask: [0; _] }
    }
    const fn set(&mut self, cpu: usize) {
        // get indices
        #[expect(clippy::integer_division)]
        let array_index = cpu / NCPUBITS;
        let bit_index = cpu % NCPUBITS;
        // set bit
        self.cpu_mask[array_index] |= 1 << bit_index;
    }
}

unsafe extern "C" {
    // int sched_setaffinity (__pid_t __pid, size_t __cpusetsize, const cpu_set_t *__cpuset) __THROW;
    fn sched_setaffinity(pid_t: c_int, cpusetsize: c_ulong, cpuset: *const cpu_set_t) -> c_int;
}

pub fn set_cpu_affinity(cpu: usize) {
    let mut cpuset = cpu_set_t::zero();
    cpuset.set(cpu);

    // SAFETY
    // - pid_t = 0 -> is the current process
    // - size_of::<cpu_set_t>() is the correct size for cpu_set_t, as the kernel expects it
    // - cpuset is a valid cpu_set_t and the pointer remains valid for the duration of the call
    #[expect(clippy::undocumented_unsafe_blocks)]
    let ret = unsafe { sched_setaffinity(0, size_of::<cpu_set_t>() as u64, &cpuset) };

    assert_eq!(ret, 0, "Error: {}", Error::last_os_error());
}
