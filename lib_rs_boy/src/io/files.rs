use std::fs::File;
use std::io::Read;
use std::{io, path};

// TODO this might be useless if I move save out of lib
pub fn load_file(file_path: &path::Path) -> io::Result<Vec<u8>> {
    let mut f = File::open(file_path)?;
    let mut buffer = Vec::new();

    f.read_to_end(&mut buffer)?;
    Ok(buffer)
}
