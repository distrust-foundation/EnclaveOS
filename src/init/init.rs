extern crate libc;
use libc::mount;
use libc::read;
use libc::write;
use libc::close;
use libc::reboot;
use libc::socket;
use libc::connect;
use libc::freopen;
use libc::fdopen;
use libc::c_void;
use libc::sockaddr;
use libc::sockaddr_vm;
use libc::SOCK_STREAM;
use libc::AF_VSOCK;
use libc::MS_NOSUID;
use libc::MS_NOEXEC;
use libc::RB_AUTOBOOT;
use std::mem::zeroed;
use std::mem::size_of;

fn main() {
    unsafe {
        mount(
            b"devtmpfs\0".as_ptr() as _,
            b"/dev\0".as_ptr() as _,
            b"devtmpfs\0".as_ptr() as _,
            MS_NOSUID | MS_NOEXEC,
            b"mode=0755\0".as_ptr() as *const c_void,
        );
        freopen(
            b"/dev/console\0".as_ptr() as _,
            b"r\0".as_ptr() as _,
            fdopen(0, b"r\0".as_ptr() as *const i8)
        );
        freopen(
            b"/dev/console\0".as_ptr() as _,
            b"w\0".as_ptr() as _,
            fdopen(1, b"w\0".as_ptr() as *const i8)
        );
        freopen(
            b"/dev/console\0".as_ptr() as _,
            b"w\0".as_ptr() as _,

            fdopen(2, b"w\0".as_ptr() as *const i8)
        );
        let mut sa: sockaddr_vm = zeroed();
        sa.svm_family = AF_VSOCK as _;
        sa.svm_port = 9000;
        sa.svm_cid = 3;
        let fd = socket(AF_VSOCK, SOCK_STREAM, 0);
        connect(
            fd,
            &sa as *const _ as *mut sockaddr,
            size_of::<sockaddr_vm>() as _,
        );
        let mut buf: [u8; 1] = [0; 1];
        buf[0] = 0xB7;
        write(fd, buf.as_ptr() as _, 1);
        read(fd, buf.as_ptr() as _, 1);
        close(fd);
        println!("Hello World from Rust init!");
        reboot(RB_AUTOBOOT);
    }
}
