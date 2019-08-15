#![feature(format_args_nl, slice_internals)]
#![allow(unused_attributes)]

#[macro_use]
pub mod macros;
pub mod consts;
pub mod entry;
pub mod show;
pub mod state;

#[macro_use]
extern crate lazy_static;

#[link_section = ".init_array"]
#[used]
static ECHO_DSO_CTORS: extern fn() = {
    extern "C" fn echo_ctor() {
    };
    echo_ctor
};

pub use crate::entry::captured_syscall;
