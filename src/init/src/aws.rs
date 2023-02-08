use std::mem::size_of;

use crate::system;
use system::SystemError;

/// Signal to Nitro hypervisor that booting was successful.
///
/// # Errors
///
/// This function returns an error if it encounters any error when sending or receiving the
/// heartbeat.
fn nitro_heartbeat() -> Result<(), SystemError> {
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

/// Get entropy sample from Nitro hardware.
///
/// # Errors
///
/// This function returns an error if the `nsm_lib::nsm_lib_init` function or
/// `nsm_lib::nsm_get_random` function fails.
pub fn get_entropy(size: usize) -> Result<Vec<u8>, SystemError> {
    use nsm_api::api::ErrorCode;
    use nsm_lib::{nsm_get_random, nsm_lib_init};
    let nsm_fd = nsm_lib_init();
    if nsm_fd < 0 {
        return Err(SystemError {
            message: String::from("Failed to connect to NSM device"),
        });
    };
    let mut dest = Vec::with_capacity(size);
    while dest.len() < size {
        let mut buf = [0u8; 256];
        let mut buf_len = buf.len();
        let status = unsafe { nsm_get_random(nsm_fd, buf.as_mut_ptr(), &mut buf_len) };
        match status {
            ErrorCode::Success => {
                dest.extend_from_slice(&buf);
            }
            _ => {
                return Err(SystemError {
                    message: String::from("Failed to get entropy from NSM device"),
                });
            }
        };
    }
    Ok(dest)
}

/// Initialize nitro device by signaling a nitro heartbeat and inserting the nsm.ko kernel module.
pub fn init_platform() -> Result<(), SystemError> {
    use system::insmod;

    nitro_heartbeat()?;
    insmod("/nsm.ko")?;

    Ok(())
}
