//! The three exports. The host allocates a buffer with `alloc`, writes UTF-8 into it, hands `call` the op and the
//! input as pointer and length, reads the reply from the packed pointer and length it returns, and frees every
//! buffer — its own and the reply's — with `free`. Nothing is kept between calls.

// Raw buffers cross the boundary here and nowhere else.
#![expect(unsafe_code)]

use std::slice;

use serde_json::{Value as Json, json};

/// A buffer of `len` zeroed bytes the host writes into; freed with `free(ptr, len)`.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0_u8; len].into_boxed_slice()).cast::<u8>()
}

/// A buffer given back, exactly as `alloc` or `call` handed it out.
///
/// # Safety
///
/// `ptr` and `len` came from one `alloc` or one `call` reply, and are given back once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(ptr: *mut u8, len: usize) {
    // SAFETY: the caller returns what was allocated, once; the boxed slice is rebuilt at its own length.
    unsafe { drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len))) };
}

/// One op on one input: the reply's pointer in the high 32 bits and its length in the low 32, to free with `free`.
///
/// # Safety
///
/// `op` and `input` point at `op_len` and `input_len` bytes the host wrote into buffers from `alloc`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn call(
    op: *const u8,
    op_len: usize,
    input: *const u8,
    input_len: usize,
) -> u64 {
    // SAFETY: the host hands over buffers it filled to these lengths, and keeps them until this returns.
    let (op, input) = unsafe {
        (
            slice::from_raw_parts(op, op_len),
            slice::from_raw_parts(input, input_len),
        )
    };
    let reply = reply(op, input);
    let len = reply.len() as u64;
    let ptr = Box::into_raw(reply.into_boxed_slice()).cast::<u8>() as u64;
    (ptr << 32) | len
}

/// The op read as text and the input as JSON, then the op table's answer; a buffer that is neither is a bug.
fn reply(op: &[u8], input: &[u8]) -> Vec<u8> {
    let answer = match (
        std::str::from_utf8(op),
        serde_json::from_slice::<Json>(input),
    ) {
        (Ok(op), Ok(input)) => crate::call(op, &input),
        (Err(error), _) => json!({ "bug": format!("the op is not UTF-8: {error}") }),
        (_, Err(error)) => json!({ "bug": format!("the input is not JSON: {error}") }),
    };
    serde_json::to_vec(&answer).expect("a reply serializes")
}
