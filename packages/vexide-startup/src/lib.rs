//! This crate provides a working entrypoint for the VEX V5 Brain.
//!
//! # Usage
//!
//! Your entrypoint function should be an async function that takes a single argument of type [`Peripherals`](vexide_devices::peripherals::Peripherals).
//! It can return any type implementing [`Termination`](vexide_core::program::Termination).
//! ```rust
//! #[vexide::main]
//! async fn main(peripherals: Peripherals) { ... }
//! ```

#![no_std]
#![feature(asm_experimental_arch)]
#![allow(clippy::needless_doctest_main)]

extern "C" {
    // These symbols don't have real types so this is a little bit of a hack
    static mut __bss_start: u32;
    static mut __bss_end: u32;
}

extern "Rust" {
    fn main();
}

/// Sets up the user stack, zeroes the BSS section, and calls the user code.
/// This function is designed to be used as an entrypoint for programs on the VEX V5 Brain.
///
/// # Safety
///
/// This function MUST only be called once and should only be called at the very start of program initialization.
/// Calling this function more than one time will seriously mess up both your stack and your heap.
pub unsafe fn program_entry() {
    #[cfg(target_arch = "arm")]
    unsafe {
        use core::arch::asm;
        asm!(
            "
            // Load the user stack
            ldr sp, =__stack_start
            "
        );
    }

    // Clear the BSS section
    #[cfg(target_arch = "arm")]
    unsafe {
        use core::ptr::addr_of_mut;
        let mut bss_start = addr_of_mut!(__bss_start);
        while bss_start < addr_of_mut!(__bss_end) {
            core::ptr::write_volatile(bss_start, 0);
            bss_start = bss_start.offset(1);
        }
    }
    // vexPrivateApiDisable
    // (unsafe { *(0x37fc020 as *const extern "C" fn(u32)) })(COLD_HEADER.options);

    unsafe {
        // Initialize the heap allocator
        // This cfg is mostly just to make the language server happy. All of this code is near impossible to run in the WASM sim.
        #[cfg(target_arch = "arm")]
        vexide_core::allocator::vexos::init_heap();
        // Print the banner
        #[cfg(not(feature = "no-banner"))]
        vexide_core::io::print!(
            "
\x1B[1;38;5;196m=%%%%%#-  \x1B[38;5;254m-#%%%%-\x1B[1;38;5;196m  :*%%%%%+.
\x1B[38;5;208m  -#%%%%#-  \x1B[38;5;254m:%-\x1B[1;38;5;208m  -*%%%%#
\x1B[38;5;226m    *%%%%#=   -#%%%%%+
\x1B[38;5;226m      *%%%%%+#%%%%%%%#=
\x1B[38;5;34m        *%%%%%%%*-+%%%%%+
\x1B[38;5;27m          +%%%*:   .+###%#
\x1B[38;5;93m           .%:\x1B[0m
vexide startup successful!
Running user code...
"
        );
        vex_sdk::vexTasksRun();
        // Run vexos background processing at a regular 2ms interval.
        // This is necessary for serial and devices to work properly.
        vexide_async::task::spawn(async {
            loop {
                vex_sdk::vexTasksRun();
                vexide_async::time::sleep(::core::time::Duration::from_millis(2)).await;
            }
        })
        .detach();
        // Call the user code
        main();
        // Exit the program
        vexide_core::program::exit();
    }
}
