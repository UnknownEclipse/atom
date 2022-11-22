#![no_std]
#![cfg_attr(test, no_main)]
#![feature(
    custom_test_frameworks,
    abi_x86_interrupt,
    allocator_api,
    alloc_error_handler
)]
#![test_runner(crate::test::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![deny(unsafe_op_in_unsafe_fn)]

extern crate alloc;

pub mod allocator;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod once_cell;
pub mod serial;
pub mod subscriber;
pub mod task;
pub mod vga_buffer;

pub fn init() {
    gdt::init();
    interrupts::init_idt();
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable();
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("allocation error: {:?}", layout)
}

pub mod test {
    use core::panic::PanicInfo;

    #[cfg(test)]
    use bootloader::{entry_point, BootInfo};

    use crate::{hlt_loop, serial_print, serial_println};

    pub trait Testable {
        fn run(&self);
    }

    impl<T> Testable for T
    where
        T: Fn(),
    {
        fn run(&self) {
            serial_print!("{}...\t", core::any::type_name::<T>());
            self();
            serial_println!("[ok]");
        }
    }

    pub fn test_runner(tests: &[&dyn Testable]) {
        serial_println!("Running {} tests", tests.len());
        for test in tests {
            test.run();
        }
        exit_qemu(QemuExitCode::Success);
    }

    pub fn test_panic_handler(info: &PanicInfo) -> ! {
        serial_println!("[failed]\n");
        serial_println!("Error: {}\n", info);
        exit_qemu(QemuExitCode::Failed);
    }

    /// Entry point for `cargo test`

    #[cfg(test)]
    entry_point!(test_kernel_main);

    #[cfg(test)]
    pub fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
        super::init();
        crate::test_main();
        hlt_loop();
    }

    #[cfg(test)]
    #[panic_handler]
    pub(crate) fn panic(info: &PanicInfo) -> ! {
        test_panic_handler(info)
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u32)]
    pub enum QemuExitCode {
        Success = 0x10,
        Failed = 0x11,
    }

    pub fn exit_qemu(exit_code: QemuExitCode) -> ! {
        use x86_64::instructions::port::Port;

        unsafe {
            let mut port = Port::new(0xf4);
            port.write(exit_code as u32);
        }

        hlt_loop();
    }
}
