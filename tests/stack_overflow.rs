#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

use core::panic::PanicInfo;

use atom::{
    serial_print, serial_println,
    test::{exit_qemu, QemuExitCode},
};
use spin::Lazy;
use volatile::Volatile;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

static TEST_IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    let mut idt = InterruptDescriptorTable::new();
    unsafe {
        idt.double_fault
            .set_handler_fn(test_double_fault_handler)
            .set_stack_index(atom::gdt::DOUBLE_FAULT_IST_INDEX);
    }

    idt
});

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_print!("stack_overflow::stack_overflow...\t");

    atom::gdt::init();
    init_test_idt();

    stack_overflow();

    panic!("Execution continued after stack overflow");
}

fn init_test_idt() {
    TEST_IDT.load();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    atom::test::test_panic_handler(info)
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow();
    Volatile::new(&mut 0).read();
}

extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
}
