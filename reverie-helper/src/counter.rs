//! counter syscall events

use std::sync::atomic::Ordering;

use reverie_common::local_state::*;

/// syscall events
pub enum NoteInfo {
    SyscallEntry,
}

/// note a syscall event
#[allow(unused)]
pub fn note_syscall(p: &mut ProcessState, no: i32, note: NoteInfo) {
    match note {
        NoteInfo::SyscallEntry => {
            p.nr_syscalls += 1;
            p.stats.nr_syscalls.fetch_add(1, Ordering::SeqCst);
            p.stats.nr_syscalls_captured.fetch_add(1, Ordering::SeqCst);
            unsafe { core::ptr::write(p.pstate_store.as_mut(), p.nr_syscalls) };
        }
    }
}
