use uefi::{
    prelude::*,
    table::boot::{OpenProtocolAttributes, OpenProtocolParams},
    proto::{media::{
        fs::SimpleFileSystem, 
        file::{Directory, FileMode, FileType, File, FileAttribute}
    }, loaded_image::LoadedImage},
    CStr16,
};

pub fn get_loaded_image(bs: &BootServices, hdl: Handle) -> Result<&mut LoadedImage, uefi::Error> {
    Ok(unsafe { &mut *bs.open_protocol::<LoadedImage>(OpenProtocolParams { handle: hdl, agent: hdl, controller: None }, OpenProtocolAttributes::Exclusive)?.interface.get() })
}

/// Get file system from handle (used to get the filesystem on which the EFI image resides, since the kernel is likely to be with it)
pub fn get_fs<'a>(bs: &BootServices, device: Handle, image: Handle) -> Result<Directory, uefi::Error> {
    unsafe { &mut *bs.open_protocol::<SimpleFileSystem>(OpenProtocolParams { handle: device, agent: image, controller: None }, OpenProtocolAttributes::Exclusive)?.interface.get() }.open_volume()
}

/// Get file from path (read-only, bootloader should not be modifying files)
pub fn open_path(dir: &mut Directory, path: &CStr16) -> Result<FileType, uefi::Error> {
    dir.open(path, FileMode::Read, FileAttribute::empty())?.into_type()
}

