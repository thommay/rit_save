use byteorder::WriteBytesExt;
use failure::format_err;
use failure::Error;
use std::fs::Metadata;
use std::io;
use std::io::Write;
use std::path::Path;

pub fn pack_data(mode: &str, name: &str, oid: &str) -> Result<Vec<u8>, Error> {
    let mut w = Vec::new();
    write!(&mut w, "{} {}\0", mode, name)?;
    let b = decode_hex(oid)?;
    for s in b {
        let _ = w.write_u8(s);
    }
    Ok(w)
}

pub fn decode_hex(s: &str) -> Result<Vec<u8>, Error> {
    if s.len() % 2 != 0 {
        Err(format_err!("hex string is not an even length"))
    } else {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.into()))
            .collect()
    }
}

pub fn stat_file(path: &Path) -> io::Result<Metadata> {
    std::fs::metadata(path)
}

pub fn is_executable(mode: u32) -> bool {
    let xugo: u32 = (libc::S_IXUSR | libc::S_IXGRP | libc::S_IXOTH).into();
    if (mode & xugo) > 0 {
        true
    } else {
        false
    }
}
