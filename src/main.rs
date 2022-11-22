#![no_std]
#![no_main]
#![feature(custom_test_frameworks, allocator_api)]
#![test_runner(atom::test::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use atom::{
    allocator, hlt_loop, memory, print, println,
    task::{executor::Executor, keyboard::print_keypresses, timer},
};
use bootloader::{entry_point, BootInfo};
use futures_util::StreamExt;
use log::{LevelFilter, Log};
use x86_64::VirtAddr;

entry_point!(kernel_main);

struct OsLogger;

impl Log for OsLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        println!("{}", *record.args());

        // println!("{}", record);
    }

    fn flush(&self) {}
}

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    println!("Hello World{}", "!");

    atom::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator =
        unsafe { memory::BootInfoFrameAllocator::from_map(&boot_info.memory_map) };

    // let page = Page::containing_address(VirtAddr::new(0xdeadbeef));
    // memory::create_example_mapping(page, &mut mapper, &mut frame_allocator);

    // let page_ptr: *mut u64 = page.start_address().as_mut_ptr();
    // unsafe { page_ptr.offset(400).write_volatile(0x_f021_f077_f065_f04e) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    log::set_logger(&OsLogger).expect("failed to initialize logger");
    log::set_max_level(LevelFilter::Info);

    tracing::info!("log enabled");

    let mut executor = Executor::new();
    executor.spawn(print_keypresses());

    executor.spawn(async {
        let mut timer = timer::Ticks::new();
        while timer.next().await.is_some() {
            print!(".");
        }
    });

    executor.run();
    // x86_64::instructions::interrupts::int3();
    // let addresses = [
    //     // the identity-mapped vga buffer page
    //     0xb8000,
    //     // some code page
    //     0x201008,
    //     // some stack page
    //     0x0100_0020_1a10,
    //     // virtual address mapped to physical address 0
    //     boot_info.physical_memory_offset,
    // ];

    // for &address in &addresses {
    //     let virt = VirtAddr::new(address);
    //     // new: use the `mapper.translate_addr` method
    //     let phys = mapper.translate_addr(virt);
    //     println!("{:?} -> {:?}", virt, phys);
    // }

    #[cfg(test)]
    test_main();

    // hlt_loop();
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    atom::test::test_panic_handler(info)
}
