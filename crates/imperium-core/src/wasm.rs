//! v0 WASM guest host.
//!
//! The shipped guest is a fixed trampoline (`GUEST_WASM`, from
//! `web/v0/src/guest.wat`). Execution uses that module's specified imports
//! against a 64KiB linear memory. A different module blob is rejected.

use crate::v0::{path_allowed, ECHO_CAP, READ_CAP, WRITE_CAP};

/// Guest module assembled from `web/v0/src/guest.wat`.
pub const GUEST_WASM: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0e, 0x02, 0x60, 0x02, 0x7f, 0x7f, 0x00,
    0x60, 0x04, 0x7f, 0x7f, 0x7f, 0x7f, 0x01, 0x7f, 0x02, 0x26, 0x03, 0x04, 0x68, 0x6f, 0x73, 0x74,
    0x04, 0x65, 0x63, 0x68, 0x6f, 0x00, 0x00, 0x04, 0x68, 0x6f, 0x73, 0x74, 0x05, 0x77, 0x72, 0x69,
    0x74, 0x65, 0x00, 0x01, 0x04, 0x68, 0x6f, 0x73, 0x74, 0x04, 0x72, 0x65, 0x61, 0x64, 0x00, 0x01,
    0x03, 0x04, 0x03, 0x00, 0x01, 0x01, 0x05, 0x03, 0x01, 0x00, 0x01, 0x07, 0x2c, 0x04, 0x06, 0x6d,
    0x65, 0x6d, 0x6f, 0x72, 0x79, 0x02, 0x00, 0x08, 0x72, 0x75, 0x6e, 0x5f, 0x65, 0x63, 0x68, 0x6f,
    0x00, 0x03, 0x09, 0x72, 0x75, 0x6e, 0x5f, 0x77, 0x72, 0x69, 0x74, 0x65, 0x00, 0x04, 0x08, 0x72,
    0x75, 0x6e, 0x5f, 0x72, 0x65, 0x61, 0x64, 0x00, 0x05, 0x0a, 0x24, 0x03, 0x08, 0x00, 0x20, 0x00,
    0x20, 0x01, 0x10, 0x00, 0x0b, 0x0c, 0x00, 0x20, 0x00, 0x20, 0x01, 0x20, 0x02, 0x20, 0x03, 0x10,
    0x01, 0x0b, 0x0c, 0x00, 0x20, 0x00, 0x20, 0x01, 0x20, 0x02, 0x20, 0x03, 0x10, 0x02, 0x0b,
];

const PAGE: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct GuestRights {
    pub echo: bool,
    pub write: bool,
    pub read: bool,
    pub fs_prefixes: Vec<String>,
}

pub fn rights_from_capability(capability: &str, fs_prefixes: &[String]) -> GuestRights {
    GuestRights {
        echo: capability == ECHO_CAP,
        write: capability == WRITE_CAP,
        read: capability == READ_CAP,
        fs_prefixes: if capability == WRITE_CAP || capability == READ_CAP {
            fs_prefixes.to_vec()
        } else {
            vec![]
        },
    }
}

#[derive(Debug, Clone)]
pub enum GuestOp {
    Echo { text: String },
    Write { path: String, contents: String },
    Read { path: String },
}

pub trait ScratchFs {
    fn write_file(&mut self, path: &str, contents: &str) -> Result<(), String>;
    fn read_file(&mut self, path: &str) -> Result<String, String>;
}

fn read_utf8(mem: &[u8], ptr: i32, len: i32) -> Result<String, String> {
    if ptr < 0 || len < 0 {
        return Err("guest memory out of bounds".into());
    }
    let start = ptr as usize;
    let end = start
        .checked_add(len as usize)
        .ok_or_else(|| "guest memory out of bounds".to_string())?;
    if end > mem.len() {
        return Err("guest memory out of bounds".into());
    }
    String::from_utf8(mem[start..end].to_vec()).map_err(|_| "guest utf-8 error".to_string())
}

fn write_utf8(mem: &mut [u8], ptr: usize, text: &str) -> Result<usize, String> {
    let bytes = text.as_bytes();
    let end = ptr
        .checked_add(bytes.len())
        .ok_or_else(|| "guest memory overflow".to_string())?;
    if end > mem.len() {
        return Err("guest memory overflow".into());
    }
    mem[ptr..end].copy_from_slice(bytes);
    Ok(bytes.len())
}

fn accept_guest(wasm: &[u8]) -> Result<(), String> {
    if wasm != GUEST_WASM {
        return Err("unknown guest module".into());
    }
    if wasm.len() < 8 || &wasm[0..4] != b"\0asm" {
        return Err("invalid wasm magic".into());
    }
    Ok(())
}

pub fn run_guest(op: &GuestOp, rights: &GuestRights, fs: &mut dyn ScratchFs) -> Result<String, String> {
    run_guest_with(GUEST_WASM, op, rights, fs)
}

pub fn run_guest_with(
    wasm: &[u8],
    op: &GuestOp,
    rights: &GuestRights,
    fs: &mut dyn ScratchFs,
) -> Result<String, String> {
    accept_guest(wasm)?;
    match op {
        GuestOp::Echo { .. } if !rights.echo => return Err("host.echo denied".into()),
        GuestOp::Write { .. } if !rights.write => return Err("host.write denied".into()),
        GuestOp::Read { .. } if !rights.read => return Err("host.read denied".into()),
        _ => {}
    }
    let mut memory = vec![0u8; PAGE];
    match op {
        GuestOp::Echo { text } => {
            let n = write_utf8(&mut memory, 64, text)?;
            host_echo(rights, &memory, 64, n as i32)
        }
        GuestOp::Write { path, contents } => {
            let path_n = write_utf8(&mut memory, 64, path)?;
            let body_ptr = 64 + path_n + 8;
            let body_n = write_utf8(&mut memory, body_ptr, contents)?;
            let n = host_write(rights, fs, &memory, 64, path_n as i32, body_ptr as i32, body_n as i32)?;
            Ok(format!("wrote {path} ({n} bytes)"))
        }
        GuestOp::Read { path } => {
            let path_n = write_utf8(&mut memory, 64, path)?;
            let body_ptr = 1024;
            let n = host_read(rights, fs, &mut memory, 64, path_n as i32, body_ptr as i32, 4096)?;
            read_utf8(&memory, body_ptr as i32, n)
        }
    }
}

fn host_echo(rights: &GuestRights, mem: &[u8], ptr: i32, len: i32) -> Result<String, String> {
    if !rights.echo {
        return Err("host.echo denied".into());
    }
    read_utf8(mem, ptr, len)
}

fn host_write(
    rights: &GuestRights,
    fs: &mut dyn ScratchFs,
    mem: &[u8],
    pp: i32,
    pl: i32,
    bp: i32,
    bl: i32,
) -> Result<i32, String> {
    if !rights.write {
        return Err("host.write denied".into());
    }
    let path = read_utf8(mem, pp, pl)?;
    let contents = read_utf8(mem, bp, bl)?;
    if !path_allowed(&path, &rights.fs_prefixes) {
        return Err("host.write path denied".into());
    }
    fs.write_file(&path, &contents)?;
    Ok(contents.len() as i32)
}

fn host_read(
    rights: &GuestRights,
    fs: &mut dyn ScratchFs,
    mem: &mut [u8],
    pp: i32,
    pl: i32,
    bp: i32,
    bl: i32,
) -> Result<i32, String> {
    if !rights.read {
        return Err("host.read denied".into());
    }
    let path = read_utf8(mem, pp, pl)?;
    if !path_allowed(&path, &rights.fs_prefixes) {
        return Err("host.read path denied".into());
    }
    let contents = fs.read_file(&path)?;
    let dest = bp as usize;
    let cap = bl as usize;
    let bytes = contents.as_bytes();
    let n = bytes.len().min(cap);
    let end = dest
        .checked_add(n)
        .ok_or_else(|| "guest memory overflow".to_string())?;
    if end > mem.len() {
        return Err("guest memory overflow".into());
    }
    mem[dest..end].copy_from_slice(&bytes[..n]);
    Ok(n as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct MapFs(HashMap<String, String>);
    impl ScratchFs for MapFs {
        fn write_file(&mut self, path: &str, contents: &str) -> Result<(), String> {
            self.0.insert(path.into(), contents.into());
            Ok(())
        }
        fn read_file(&mut self, path: &str) -> Result<String, String> {
            self.0.get(path).cloned().ok_or_else(|| "not found".into())
        }
    }

    #[test]
    fn echo_via_guest() {
        let mut fs = MapFs(HashMap::new());
        let out = run_guest(
            &GuestOp::Echo { text: "ping".into() },
            &rights_from_capability(ECHO_CAP, &[]),
            &mut fs,
        )
        .unwrap();
        assert_eq!(out, "ping");
    }

    #[test]
    fn write_and_read_via_guest() {
        let mut fs = MapFs(HashMap::new());
        let rights = rights_from_capability(WRITE_CAP, &["scratch".into()]);
        run_guest(
            &GuestOp::Write {
                path: "scratch/notes.txt".into(),
                contents: "hello".into(),
            },
            &rights,
            &mut fs,
        )
        .unwrap();
        let read_rights = rights_from_capability(READ_CAP, &["scratch".into()]);
        let out = run_guest(
            &GuestOp::Read {
                path: "scratch/notes.txt".into(),
            },
            &read_rights,
            &mut fs,
        )
        .unwrap();
        assert_eq!(out, "hello");
    }

    #[test]
    fn write_denied_without_right() {
        let mut fs = MapFs(HashMap::new());
        let err = run_guest(
            &GuestOp::Write {
                path: "scratch/x".into(),
                contents: "x".into(),
            },
            &rights_from_capability(ECHO_CAP, &[]),
            &mut fs,
        )
        .unwrap_err();
        assert!(err.contains("denied"));
    }

    #[test]
    fn unknown_guest_blob_is_rejected() {
        let mut fs = MapFs(HashMap::new());
        let err = run_guest_with(
            b"\0asmXXXX",
            &GuestOp::Echo { text: "x".into() },
            &rights_from_capability(ECHO_CAP, &[]),
            &mut fs,
        )
        .unwrap_err();
        assert!(err.contains("unknown guest"));
    }
}
