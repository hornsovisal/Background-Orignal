#![no_std]
#![no_main]

use aya_ebpf::{macros::map, maps::HashMap};

mod tracepoints;
mod kprobs;
mod binding;
mod cgroups;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { core::hint::unreachable_unchecked() }
}

#[map]
pub static SCOPED_APPS: HashMap<u32, u32> = HashMap::with_max_entries(1024, 0);


pub mod address_family {
    pub const AF_INET: u16 = 2;
    pub const AF_INET6: u16 = 10;
}

pub mod transport_protocol {
    pub const TCP: u16 = 6;
    pub const UDP: u16 = 17;
}