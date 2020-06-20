use std::path::PathBuf;

pub fn read_memory(address: u64, size: u64) -> Vec<u8> {
    let mut result = Vec::with_capacity(size as usize);

    for index in 0..size {
        unsafe {
            let ptr = (address + index) as *mut u8;
            result.push(*ptr);
        }
    }
    result
}
pub struct Config {
    pub mods_path: PathBuf,
    pub mods_enabled: bool,
    pub last_character: String,
}