use std::mem::size_of;
use crate::sys::SystemError;

/// Signal to Nitro hypervisor that booting was successful.
///
/// # Errors
///
/// This function returns an error if it encounters any error when sending or receiving the
/// heartbeat.
pub fn nitro_heartbeat() -> Result<(), SystemError> {
    use libc::{close, connect, read, sockaddr_vm, socket, write, AF_VSOCK, SOCK_STREAM};
    let buf: [u8; 1] = [0xB7; 1]; // AWS Nitro magic heartbeat value
    let family = AF_VSOCK;
    let port = 9000;
    let cid = 3;
    let fd = unsafe { socket(AF_VSOCK, SOCK_STREAM, 0) };
    let sockaddr = sockaddr_vm {
        svm_family: family.try_into().expect("AF_VSOCK does not fit sa_family_t"),
        svm_reserved1: Default::default(),
        svm_port: port,
        svm_cid: cid,
        svm_zero: Default::default(),
    };
    let sockaddr_size = libc::socklen_t::try_from(size_of::<sockaddr_vm>())
        .expect("sizeof sockaddr_vm is larger than socklen_t");

    if unsafe {
        connect(fd, std::ptr::addr_of!(sockaddr).cast(), sockaddr_size)
    } < 0
    {
        return Err(SystemError {
            message: format!(
                "Failed to connect to socket: family={family}, port={port}, cid={cid}"
            ),
        })
    }

    // TODO: error handling
    unsafe {
        write(fd, buf.as_ptr().cast(), 1);
        read(fd, buf.as_ptr() as _, 1);
        close(fd);
    }
    Ok(())
}
