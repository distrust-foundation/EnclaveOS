use crate::sys::strerror;
use crate::error::{Context, Result};
use std::mem::size_of;

/// Signal to Nitro hypervisor that booting was successful.
///
/// # Errors
///
/// This function returns an error if it encounters any error when sending or receiving the
/// heartbeat.
pub fn nitro_heartbeat() -> Result<()> {
    use libc::{close, connect, read, sockaddr_vm, socket, write, AF_VSOCK, SOCK_STREAM};
    let buf: [u8; 1] = [0xB7; 1]; // AWS Nitro magic heartbeat value
    let family = AF_VSOCK;
    let port = 9000;
    let cid = 3;
    let fd = strerror(unsafe { socket(AF_VSOCK, SOCK_STREAM, 0) })
        .context("Unable to create AF_VSOCK, SOCK_STREAM socket")?;
    let sockaddr = sockaddr_vm {
        svm_family: family
            .try_into()
            .expect("AF_VSOCK does not fit sa_family_t"),
        svm_reserved1: Default::default(),
        svm_port: port,
        svm_cid: cid,
        svm_zero: Default::default(),
    };
    let sockaddr_size = libc::socklen_t::try_from(size_of::<sockaddr_vm>())
        .expect("sizeof sockaddr_vm is larger than socklen_t");

    strerror(unsafe { connect(fd, std::ptr::addr_of!(sockaddr).cast(), sockaddr_size) })
        .with_context(|| {
            format!("Failed to connect to socket: family={family}, port={port}, cid={cid}")
        })?;

    // note: all errno values will cast successfully and we don't care about success values
    unsafe {
        strerror(write(fd, buf.as_ptr().cast(), 1).try_into().unwrap_or(0))?;
        strerror(read(fd, buf.as_ptr() as _, 1).try_into().unwrap_or(0))?;
        strerror(close(fd))?;
    }
    Ok(())
}
