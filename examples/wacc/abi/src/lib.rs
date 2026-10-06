#![no_std]

use dlmalloc::GlobalDlmalloc;

#[global_allocator]
static ALLOCATOR: GlobalDlmalloc = GlobalDlmalloc;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}

// The component canonical ABI allocates host-to-guest return buffers through a
// `cabi_realloc` core export. wit-bindgen's runtime omits this export for
// wasm32-wasip2 targets, so the guest supplies it over its global allocator.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cabi_realloc(
    old_ptr: *mut u8,
    old_len: usize,
    align: usize,
    new_len: usize,
) -> *mut u8 {
    use core::alloc::{GlobalAlloc, Layout};

    let ptr = if old_len == 0 {
        if new_len == 0 {
            return align as *mut u8;
        }
        ALLOCATOR.alloc(Layout::from_size_align_unchecked(new_len, align))
    } else {
        debug_assert_ne!(new_len, 0, "non-zero old_len requires non-zero new_len!");
        ALLOCATOR.realloc(
            old_ptr,
            Layout::from_size_align_unchecked(old_len, align),
            new_len,
        )
    };
    if ptr.is_null() {
        core::arch::wasm32::unreachable();
    }
    ptr
}

wit_bindgen::generate!({
    world: "unlock-script",
    path: "../../../wit/wacc.wit",
});

struct Component;

impl Guest for Component {
    fn for_great_justice() -> i32 {
        use crate::cryptid::wacc::host::{
            branch, check_eq, check_preimage, check_preimage_value, check_signature, log, push,
            push_value,
        };

        let push_ok = push("/abi/");
        let push_value_ok = push_value(&[0xAB, 0xCD]);
        let branch_result = branch("/abi/proof");
        let log_branch_ok = log(&branch_result);

        // Each check is expected to fail without matching state; the calls
        // exercise the host-side lowering of every import signature.
        let _ = check_eq("/abi/nonexistent");
        let _ = check_preimage("/abi/nonexistent");
        let _ = check_preimage_value(&[0x00, 0x12, 0x34], "/abi/nonexistent");
        let _ = check_signature("/abi/nonexistent", "/abi/nonexistent-msg");

        let log_ok = log("/abi/log-line");
        if push_ok && push_value_ok && log_branch_ok && log_ok && !branch_result.is_empty() {
            1
        } else {
            0
        }
    }
}

export!(Component);
