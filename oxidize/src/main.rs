#![no_main]
#![no_std]
#![feature(abi_efiapi)]

mod fs;
mod elf;
mod load;

use log::*;
use uefi::{
    prelude::*,
    CStr16,
    table::boot::{MemoryType},
    proto::console::gop::GraphicsOutput,
};

/// Name of the kernel binary encoded in UTF-16: "ferrous-kernel\0"
const KERNEL_PATH: [u16; 15] = [0x66 ,0x65 ,0x72 ,0x72 ,0x6f ,0x75 ,0x73 ,0x2d ,0x6b ,0x65 ,0x72 ,0x6e ,0x65 ,0x6c, 0x00];

#[entry]
fn main(image: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    let entry_point: extern "sysv64" fn(*mut u8, usize) -> ! = {
        debug!("Acquiring loaded image");
        let li = fs::get_loaded_image(st.boot_services(), image)?;

        debug!("Acquiring file system");
        let mut fs = fs::get_fs(st.boot_services(), li.device(), image)?;

        debug!("Acquiring handle to kernel file");
        let mut kernel_file = match fs::open_path(&mut fs, CStr16::from_u16_with_nul(&KERNEL_PATH).unwrap())? {
            uefi::proto::media::file::FileType::Regular(file) => file,
            uefi::proto::media::file::FileType::Dir(_) => { error!("Kernel path should be to a file"); panic!(); },
        };
        
        debug!("Loading kernel image");
        let entry_addr = load::load_kernel_image(st.boot_services(), &mut kernel_file)?;

        let read = |x| {
            unsafe { core::ptr::read(x as *const u8) }
        };

        trace!("Memory at entry: {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x} {:#x}", 
        read(entry_addr + 0), read(entry_addr + 1), read(entry_addr + 2), read(entry_addr + 3),
        read(entry_addr + 4), read(entry_addr + 5), read(entry_addr + 6), read(entry_addr + 7));

        unsafe { core::mem::transmute(entry_addr as *const u8) }
    };

    let (fb, size) = {
        let go = unsafe { &mut *st.boot_services().locate_protocol::<GraphicsOutput>()?.get() };
        let mode = go.current_mode_info();
        debug!("Graphics: {} x {}, {:?}", mode.resolution().0, mode.resolution().1, mode.pixel_format());
        for i in (0..go.frame_buffer().size()).step_by(4) {
            unsafe { go.frame_buffer().write_byte(i + 2, 0xFF); }
        }
        (go.frame_buffer().as_mut_ptr(), go.frame_buffer().size())
    };

    let mmap_buf = {
        let mmap_sizes = st.boot_services().memory_map_size();
        let ptr = st.boot_services().allocate_pool(MemoryType::LOADER_DATA, mmap_sizes.map_size)?;
        unsafe { core::slice::from_raw_parts_mut(ptr, mmap_sizes.map_size) }
    };

    info!("Running kernal");

    let (_st, _mmap) = st.exit_boot_services(image, mmap_buf)?;
	// what to do with mmap?

    entry_point(fb, size);
 }
