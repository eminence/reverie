#![allow(unused_imports)]
#![allow(dead_code)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::unreadable_literal))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::too_many_arguments))]
#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::inconsistent_digit_grouping)
)]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::let_and_return))]
#![cfg_attr(feature = "cargo-clippy", allow(clippy::type_complexity))]

#[macro_use]
extern crate lazy_static;

pub use reverie_common;
pub use syscalls;

pub mod aux;
pub mod auxv;
pub mod block_events;
pub mod config;
pub mod debug;
pub mod hooks;
pub mod ns;
pub mod patcher;
pub mod remote_rwlock;
pub mod rpc_ptrace;
pub mod sched_wait;
pub mod stubs;
pub mod traced_task;
pub mod vdso;
