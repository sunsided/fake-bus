//! # Fake `embedded-hal` buses for stateful chip models
//!
//! Host tests of `embedded-hal` drivers need something on the other end of the bus. This
//! crate provides the bus side; you provide a behavioral model of the chip that keeps its
//! own state (registers, pointers, measurements), and the fakes drive it:
//!
//! - [`FakeI2c`] implements the blocking and async [`I2c`](embedded_hal::i2c::I2c) traits
//!   on one [`I2cTarget`] or on several targets sharing one bus.
//! - [`FakeSpi`] implements the blocking and async
//!   [`SpiDevice`](embedded_hal::spi::SpiDevice) traits on an [`SpiTarget`].
//! - [`FakeDelay`] implements the blocking and async
//!   [`DelayNs`](embedded_hal::delay::DelayNs) traits by advancing a [`Clock`] instead of
//!   waiting.
//!
//! Models live in a [`RefCell`](core::cell::RefCell), so a test can inspect or change them
//! while a driver owns the bus. All fakes hold shared references only, so they are `Copy`
//! and can be handed to several drivers.
//!
//! Unlike [`embedded-hal-mock`](https://crates.io/crates/embedded-hal-mock), which checks
//! a script of expected transactions, this crate does not know what the driver should do.
//! The model decides how to respond, so tests check behavior rather than exact bus traffic.
//!
//! ## Example
//!
//! ```
//! use core::cell::RefCell;
//! use embedded_hal::i2c::I2c;
//! use fake_bus::{FakeI2c, I2cTarget};
//!
//! /// A chip with 256 registers behind an auto-incrementing register pointer.
//! struct Registers {
//!     registers: [u8; 256],
//!     pointer: u8,
//!     expect_pointer: bool,
//! }
//!
//! impl I2cTarget for Registers {
//!     fn address(&self) -> u8 {
//!         0x42
//!     }
//!
//!     fn start(&mut self, is_read: bool) -> bool {
//!         // The first byte of a write transfer is the register pointer.
//!         self.expect_pointer = !is_read;
//!         true
//!     }
//!
//!     fn write(&mut self, byte: u8) -> bool {
//!         if self.expect_pointer {
//!             self.pointer = byte;
//!             self.expect_pointer = false;
//!         } else {
//!             self.registers[usize::from(self.pointer)] = byte;
//!             self.pointer = self.pointer.wrapping_add(1);
//!         }
//!
//!         true
//!     }
//!
//!     fn read(&mut self) -> u8 {
//!         let byte = self.registers[usize::from(self.pointer)];
//!         self.pointer = self.pointer.wrapping_add(1);
//!         byte
//!     }
//! }
//!
//! let chip = RefCell::new(Registers { registers: [0; 256], pointer: 0, expect_pointer: false });
//! let mut bus = FakeI2c::new(&chip);
//!
//! bus.write(0x42, &[0x10, 1, 2, 3]).unwrap();
//!
//! let mut buffer = [0; 3];
//! bus.write_read(0x42, &[0x10], &mut buffer).unwrap();
//! assert_eq!(buffer, [1, 2, 3]);
//!
//! // The test can look at the model directly.
//! assert_eq!(chip.borrow().registers[0x12], 3);
//! ```
//!
//! ## Shared buses
//!
//! Several targets on one I²C bus are an array (or slice) of
//! `&RefCell<dyn I2cTarget>`; see [`FakeI2c`]. No heap allocation is needed.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod delay;
mod i2c;
mod spi;

#[cfg(test)]
mod test_model;

pub use delay::{Clock, FakeDelay};
pub use i2c::{FakeI2c, I2cTarget, I2cTargets};
pub use spi::{FakeSpi, SpiTarget};
