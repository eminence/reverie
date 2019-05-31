//! ffi.rs: re-exports trampoline symbols.
//!
/// NB: rust (as of today's nightly) doesn't export symbols from .c/.S files,
/// also rust doesn't seem to have visibility controls such as
/// __attribute__((visibility("hidden"))), there's no good way to workaround
/// this, see rust issue ##36342 for more details.
/// As a result, we re-export all the needed C/ASM symbols to make sure our
/// cdylib is built correctly.

use core::ffi::c_void;
use crate::consts;
use crate::captured_syscall;
use crate::local_state::*;

static SYSCALL_UNTRACED: u64 = 0x7000_0000;
static SYSCALL_TRACED: u64 = 0x7000_0004;

extern "C" {
    fn _raw_syscall(syscallno: i32,
                    arg0: i64,
                    arg1: i64,
                    arg2: i64,
                    arg3: i64,
                    arg4: i64,
                    arg5: i64,
                    syscall_insn: *mut c_void,
                    sp1: i64,
                    sp2: i64) -> i64;
    fn _remote_syscall_helper();
    fn _remote_funccall_helper();
}

#[no_mangle]
unsafe extern "C" fn traced_syscall(
    syscallno: i32,
    arg0: i64,
    arg1: i64,
    arg2: i64,
    arg3: i64,
    arg4: i64,
    arg5: i64) -> i64 {
    _raw_syscall(syscallno, arg0, arg1, arg2, arg3, arg4, arg5,
                 SYSCALL_TRACED as *mut _, 0, 0)
}

#[no_mangle]
unsafe extern "C" fn untraced_syscall(
    syscallno: i32,
    arg0: i64,
    arg1: i64,
    arg2: i64,
    arg3: i64,
    arg4: i64,
    arg5: i64) -> i64 {
    _raw_syscall(syscallno, arg0, arg1, arg2, arg3, arg4, arg5,
                 SYSCALL_UNTRACED as *mut _, 0, 0)
}

#[repr(C)]
struct syscall_info {
    no: u64,
    args: [u64; 6],
}

static mut PSTATE: Option<*mut ProcessState> = None;

#[no_mangle]
unsafe extern "C" fn syscall_hook(info: *const syscall_info) -> i64 {
    if let Some(pstate) = PSTATE.and_then(|p|p.as_mut()) {
        let sc = info.as_ref().unwrap();
        //let tid = syscall(SYS_gettid as i32, 0, 0, 0, 0, 0, 0).unwrap() as i32;
        //let tp = pstate.get_thread_data(tid).map(|p| p as *mut ThreadState);
        //if let Some(tstate) = tp.and_then(|p|p.as_mut()) {
        let mut tstate: ThreadState = core::mem::zeroed();
            let res = captured_syscall(pstate, &mut tstate, sc.no as i32,
                                   sc.args[0] as i64, sc.args[1] as i64,
                                   sc.args[2] as i64, sc.args[3] as i64,
                                   sc.args[4] as i64, sc.args[5] as i64);
            return res;
        //}
    }
    return -38;      // ENOSYS
}

#[link_section = ".init_array"]
#[used]
static EARLY_TRAMPOLINE_INIT: extern fn() = {
    extern "C" fn trampoline_ctor() {
        let syscall_hook_ptr = consts::SYSTRACE_LOCAL_SYSCALL_HOOK_ADDR as *mut u64;
        unsafe {
            core::ptr::write(syscall_hook_ptr, syscall_hook as u64);
        }

        let ready = consts::SYSTRACE_LOCAL_SYSCALL_TRAMPOLINE as *mut u64;
        unsafe {
            core::ptr::write(ready, 1);
        }
        let state_addr_ptr = consts::SYSTRACE_LOCAL_SYSTRACE_LOCAL_STATE as *const u64;
        unsafe {
            let val = core::ptr::read(state_addr_ptr);
            PSTATE = Some(val as *mut ProcessState);
        }
        let syscall_helper_ptr = consts::SYSTRACE_LOCAL_SYSCALL_HELPER as *mut u64;
        unsafe {
            core::ptr::write(syscall_helper_ptr, _remote_syscall_helper as u64);
        }
        let rpc_helper_ptr = consts::SYSTRACE_LOCAL_RPC_HELPER as *mut u64;
        unsafe {
            core::ptr::write(rpc_helper_ptr, _remote_funccall_helper as u64);
        }
    };
    trampoline_ctor
};
