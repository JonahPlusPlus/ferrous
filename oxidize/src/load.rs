
use crate::elf;
use goblin::elf64::*;
use uefi::{
    prelude::*,
    proto::media::file::RegularFile,
    table::boot::{AllocateType, MemoryType},
};
use log::*;

/// Load kernel image from file
pub fn load_kernel_image(bs: &BootServices, file: &mut RegularFile) -> Result<u64, uefi::Error> {
    let (hdr, phdrs) = elf::read_elf_file(bs, file)?;
    trace!("Acquired ELF file header and program headers");

    debug!("{:?}", hdr);

    let e_entry = hdr.e_entry;

    // load segments from program headers
    let pages_addr = load_program_segments(bs, file, phdrs)?;
    
    let entry_point_addr = pages_addr + e_entry;

    trace!("Created entry point at {:#x} from e_entry {:#x} and pages_addr {:#x}", entry_point_addr, e_entry, pages_addr);

    bs.free_pool(unsafe { core::mem::transmute(hdr) })?;
    bs.free_pool(unsafe { core::mem::transmute(phdrs.as_mut_ptr()) })?;

    Ok(entry_point_addr)
}

fn load_program_segments(bs: &BootServices, file: &mut RegularFile, phdrs: &[program_header::ProgramHeader]) -> Result<u64, uefi::Error> {
    if phdrs.len() == 0 {
        return Err(uefi::Error::new(uefi::Status::NOT_FOUND, ()));
    }

    let mut load_size = 0;

    for phdr in phdrs {
        if phdr.p_type == program_header::PT_LOAD {
            load_size += phdr.p_memsz;
        }
    }

    // Not sure if this works, but perhaps allocating all the pages at once and loading the kernel might work, as long as there aren't any huge jumps

    let num_pages = ((load_size >> 12) + (if load_size & 0xFFF != 0 { 1 } else { 0 })) as usize;
    let pages_addr = bs.allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, num_pages)?;
    trace!("Allocated {} pages at {:#x}", num_pages, pages_addr);

    if load_size == 0 {
        error!("ELF file did not have any LOAD segments");
        return Err(uefi::Error::new(uefi::Status::LOAD_ERROR, ()));
    } else {
        trace!("Loading {} bytes of LOAD segments", load_size);

        for (i, phdr) in phdrs.iter().enumerate() {
            if phdr.p_type == program_header::PT_LOAD {
                trace!("Loading segment {}", i);
                load_segment(bs, file, phdr, pages_addr)?;
            }
        }
    }

    Ok(pages_addr)
}

fn load_segment(bs: &BootServices, file: &mut RegularFile, phdr: &program_header::ProgramHeader, pages_addr: u64) -> Result<(), uefi::Error> {
    let pages_ptr = unsafe { core::mem::transmute(pages_addr + phdr.p_vaddr) };

    trace!("Set file position to segment offset: {:#x}", phdr.p_offset);
    file.set_position(phdr.p_offset)?;

    if phdr.p_filesz > 0 {
        let seg_size = phdr.p_filesz as usize;
        let ptr = bs.allocate_pool(MemoryType::LOADER_CODE, seg_size)?;
        let buf = unsafe { core::slice::from_raw_parts_mut(ptr, seg_size) };
        
        trace!("Reading segment into buffer");
        assert_eq!(file.read(buf).expect("Failed to read file"), seg_size, "Read number of bytes differs from size of buffer");
    
        trace!("Copying segment from buffer into {:#x}", pages_addr + phdr.p_vaddr);
        unsafe { bs.memmove(pages_ptr, ptr, seg_size); }

        trace!("Freeing buffer");
        bs.free_pool(ptr)?;
    }

    let zf_size = (phdr.p_memsz - phdr.p_filesz) as usize;

    if zf_size > 0 {
        let zf_ptr = phdr.p_vaddr + phdr.p_filesz;

        trace!("Zero-filling {} bytes at {:#x}", zf_size, zf_ptr);

        unsafe { bs.set_mem(core::mem::transmute(zf_ptr), zf_size, 0); }
    }

    trace!("Loaded segment");

    Ok(())
}
