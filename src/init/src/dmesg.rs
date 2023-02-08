use libc::{clock_gettime, timespec, CLOCK_BOOTTIME};

// TODO: this file contains some ugly hacks. should they be fixed?

#[macro_export]
macro_rules! dprintln {
    () => { ::std::println!("{}", boot_time()) };
    ($($arg:tt)*) => { ::std::println!("{} {}", $crate::dmesg::boot_time(), format!($($arg)*)) };
}
pub use dprintln as println;

#[macro_export]
macro_rules! deprintln {
    () => { ::std::eprintln!("{}", boot_time()) };
    ($($arg:tt)*) => { ::std::eprintln!("{} {}", $crate::dmesg::boot_time(), format!($($arg)*)) };
}
pub use deprintln as eprintln;

/// Dmesg-formatted seconds since boot
#[allow(clippy::must_use_candidate)]
pub fn boot_time() -> String {
    let mut t = timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    unsafe {
        clock_gettime(CLOCK_BOOTTIME, std::ptr::addr_of_mut!(t));
    }
    format!("[ {: >4}.{}]", t.tv_sec, t.tv_nsec / 1000)
}
