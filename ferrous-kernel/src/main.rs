#![no_std]
#![no_main]

use core::panic::PanicInfo;

/// Panic Handler
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}


/// Entry Point
#[no_mangle]
pub extern "sysv64" fn _start(fb: *mut u8, size: usize) -> ! {
    let frame_buf = unsafe { core::slice::from_raw_parts_mut(fb, size) };

    for i in (0..size).step_by(4) {
        frame_buf[i] = 0xFF;
    }

    loop {}
}
