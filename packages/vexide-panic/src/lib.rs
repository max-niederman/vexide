//! Panic handler implementation for [`vexide`](https://crates.io/crates/vexide)
//!
//! Supports capturing and printing backtraces to aid in debugging.
//!
//! If the `display_panics` feature is enabled, it will also display the panic message on the V5 Brain display.

#![no_std]

pub mod backtrace;
#[cfg(target_arch = "arm")]
pub mod unwind;

extern crate alloc;

use alloc::string::{String, ToString};
use core::fmt::Write;

use vexide_core::println;
#[cfg(feature = "display_panics")]
use vexide_devices::{
    color::Rgb,
    geometry::Point2,
    screen::{Rect, Screen, ScreenError, Text, TextSize},
};

use crate::backtrace::Backtrace;

#[cfg(target_arch = "wasm32")]
extern "C" {
    /// Prints a backtrace to the debug console
    fn sim_log_backtrace();
}

/// Draw an error box to the screen.
///
/// This function is internally used by the vexide panic handler for displaying
/// panic messages graphically before exiting.
#[cfg(feature = "display_panics")]
fn draw_error(screen: &mut Screen, msg: &str, backtrace: &Backtrace) {
    const ERROR_BOX_MARGIN: i16 = 16;
    const ERROR_BOX_PADDING: i16 = 16;
    const LINE_HEIGHT: i16 = 20;
    const LINE_MAX_WIDTH: usize = 52;

    fn draw_text(screen: &mut Screen, buffer: &str, line: i16) {
        screen.fill(
            &Text::new(
                buffer,
                TextSize::Small,
                Point2 {
                    x: ERROR_BOX_MARGIN + ERROR_BOX_PADDING,
                    y: ERROR_BOX_MARGIN + ERROR_BOX_PADDING + (line * LINE_HEIGHT),
                },
            ),
            Rgb::WHITE,
        );
    }

    let error_box_rect = Rect::new(
        Point2 {
            x: ERROR_BOX_MARGIN,
            y: ERROR_BOX_MARGIN,
        },
        Point2 {
            x: Screen::HORIZONTAL_RESOLUTION - ERROR_BOX_MARGIN,
            y: Screen::VERTICAL_RESOLUTION - ERROR_BOX_MARGIN,
        },
    );

    screen.fill(&error_box_rect, Rgb::RED);
    screen.stroke(&error_box_rect, Rgb::WHITE);

    let mut buffer = String::new();
    let mut line: i16 = 0;

    for (i, character) in msg.char_indices() {
        if !character.is_ascii_control() {
            buffer.push(character);
        }

        if character == '\n' || ((buffer.len() % LINE_MAX_WIDTH == 0) && (i > 0)) {
            draw_text(screen, &buffer, line);
            line += 1;
            buffer.clear();
        }
    }

    if !buffer.is_empty() {
        draw_text(screen, &buffer, line);

        line += 1;
    }

    line += 1;
    draw_text(screen, "stack backtrace:", line);
    line += 1;

    const ROW_LENGTH: usize = 3;
    for (col, frames) in backtrace.frames.chunks(ROW_LENGTH).enumerate() {
        let mut msg = String::new();
        for (row, frame) in frames.iter().enumerate() {
            write!(msg, "{:>3}: {:?}    ", col * ROW_LENGTH + row, frame).unwrap();
        }
        draw_text(screen, msg.trim_end(), line);
        line += 1;
    }
}

#[panic_handler]
/// The panic handler for vexide.
pub fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    println!("{info}");

    let backtrace = Backtrace::capture();

    unsafe {
        #[cfg(feature = "display_panics")]
        draw_error(&mut Screen::new(), &info.to_string(), &backtrace);

        #[cfg(target_arch = "wasm32")]
        sim_log_backtrace();
        #[cfg(not(target_arch = "wasm32"))]
        println!("{backtrace}");

        #[cfg(not(feature = "display_panics"))]
        vexide_core::program::exit();
        // unreachable without display_panics
        #[cfg(feature = "display_panics")]
        loop {
            // Flush the serial buffer so that the panic message is printed
            vex_sdk::vexTasksRun();
        }
    }
}
