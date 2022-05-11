use uefi::{
    prelude::*,
    proto::media::file::RegularFile,
    table::boot::MemoryType,
};
use goblin::elf64::*;
use log::*;

// e_ident: first 16 bytes of header
// 4: magic, 1: class, 1: data, 1: version, 1: osabi, 1: abiversion, 7: padding
// You should verify your kernel file to make sure it looks like this
const IDENT: [u8; 16] = [0x7f, b'E', b'L', b'F', 0x2, 0x1, 0x1, 0x0, 0x0, 0, 0, 0, 0, 0, 0, 0];

/// Reads the ELF header and program headers
pub fn read_elf_file<'a>(bs: &BootServices, file: &mut RegularFile) -> Result<(&'a mut header::Header, &'a mut [program_header::ProgramHeader]), uefi::Error> {
    // Allocate memory for header
    let hdr_ptr = bs.allocate_pool(MemoryType::LOADER_DATA, core::mem::size_of::<header::Header>())?;
    let hdr_buf = unsafe { core::slice::from_raw_parts_mut(hdr_ptr, core::mem::size_of::<header::Header>()) };

    // Reset file cursor position to beginning of file
    file.set_position(0)?;

    // Read ELF header into buffer
    // Assert that the read number of bytes is the same as the buffer size, otherwise it's invalid
    assert_eq!(file.read(hdr_buf).expect("Failed to read file"), core::mem::size_of::<header::Header>(), "Read number of bytes differs from size of buffer");

    let hdr = <header::Header as plain::Plain>::from_mut_bytes(hdr_buf).expect("Failed to get header from buffer");

    // Verify the header by comparing the first 16 bytes
    if hdr.e_ident.cmp(&IDENT).is_ne() {
        error!("e_ident is not valid");
        return Err(uefi::Error::new(uefi::Status::COMPROMISED_DATA, ()));
    }

    // Allocate buffer for reading program headers
    let phdrs_size = (hdr.e_phentsize * hdr.e_phnum) as usize;
    let phdrs_ptr = bs.allocate_pool(MemoryType::LOADER_DATA, phdrs_size)?;
    let phdrs_buf = unsafe { core::slice::from_raw_parts_mut(phdrs_ptr, phdrs_size) };

    // Set file cursor position to where the program headers are located
    file.set_position(hdr.e_phoff)?;

    // Read program headers into buffer
    // Assert that the read number of bytes is the same as the buffer size, otherwise it's invalid
    assert_eq!(file.read(phdrs_buf).expect("Failed to read file"), phdrs_size, "Read number of bytes differs from size of buffer");

    let phdrs = <program_header::ProgramHeader as plain::Plain>::slice_from_mut_bytes(phdrs_buf).expect("Failed to get program headers from buffer");

    Ok((hdr, phdrs))
}
