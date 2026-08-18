//! C/FFI Bindings
//! 
//! Exports C-compatible API for Python and Node.js.

use std::os::raw::{c_char, c_int, c_void};
use imperium_core::*;

#[no_mangle]
pub extern "C" fn imperium_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr() as *const c_char
}

#[no_mangle]
pub extern "C" fn imperium_intent_compile(
    nl_json: *const c_char,
    ir_out: *mut *mut c_char,
) -> c_int {
    // TODO: Implement FFI for intent compilation
    0
}

#[no_mangle]
pub extern "C" fn imperium_simulation_run(
    intent_ir_json: *const c_char,
    rollouts: c_int,
    result_out: *mut *mut c_char,
) -> c_int {
    // TODO: Implement FFI for simulation
    0
}

#[no_mangle]
pub extern "C" fn imperium_capability_invoke(
    capability_id: *const c_char,
    method: *const c_char,
    input_json: *const c_char,
    output_out: *mut *mut c_char,
) -> c_int {
    // TODO: Implement FFI for capability invocation
    0
}