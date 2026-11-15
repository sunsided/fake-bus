//! A register-file model that records bus events, for the unit tests.

use crate::{Clock, I2cTarget, SpiTarget};

/// The read bit in the first SPI byte.
pub const SPI_READ: u8 = 0x80;

/// 256 registers behind an auto-incrementing pointer. The first byte of an I²C write
/// transfer, or the first SPI byte (read bit plus address), sets the pointer.
pub struct Model {
    pub address: u8,
    pub registers: [u8; 256],
    pointer: u8,
    expect_pointer: bool,
    spi_read: bool,
    pub ack_start: bool,
    /// The number of bytes acknowledged before the model NACKs.
    pub ack_bytes: usize,
    pub starts: usize,
    pub stops: usize,
    pub selects: usize,
    pub deselects: usize,
    pub time_ns: u64,
    pub clock_calls: usize,
}

impl Model {
    pub fn new(address: u8) -> Self {
        Self {
            address,
            registers: [0; 256],
            pointer: 0,
            expect_pointer: false,
            spi_read: false,
            ack_start: true,
            ack_bytes: usize::MAX,
            starts: 0,
            stops: 0,
            selects: 0,
            deselects: 0,
            time_ns: 0,
            clock_calls: 0,
        }
    }

    fn write_data(&mut self, byte: u8) {
        self.registers[usize::from(self.pointer)] = byte;
        self.pointer = self.pointer.wrapping_add(1);
    }

    fn read_data(&mut self) -> u8 {
        let byte = self.registers[usize::from(self.pointer)];
        self.pointer = self.pointer.wrapping_add(1);
        byte
    }
}

impl I2cTarget for Model {
    fn address(&self) -> u8 {
        self.address
    }

    fn start(&mut self, is_read: bool) -> bool {
        self.starts += 1;
        self.expect_pointer = !is_read;
        self.ack_start
    }

    fn write(&mut self, byte: u8) -> bool {
        if self.ack_bytes == 0 {
            return false;
        }

        self.ack_bytes -= 1;

        if self.expect_pointer {
            self.pointer = byte;
            self.expect_pointer = false;
        } else {
            self.write_data(byte);
        }

        true
    }

    fn read(&mut self) -> u8 {
        self.read_data()
    }

    fn stop(&mut self) {
        self.stops += 1;
    }
}

impl SpiTarget for Model {
    fn select(&mut self) {
        self.selects += 1;
        self.expect_pointer = true;
    }

    fn deselect(&mut self) {
        self.deselects += 1;
    }

    fn transfer(&mut self, mosi: u8) -> u8 {
        if self.expect_pointer {
            self.pointer = mosi & !SPI_READ;
            self.spi_read = mosi & SPI_READ != 0;
            self.expect_pointer = false;
            0
        } else if self.spi_read {
            self.read_data()
        } else {
            self.write_data(mosi);
            0
        }
    }
}

impl Clock for Model {
    fn advance_ns(&mut self, ns: u64) {
        self.time_ns += ns;
        self.clock_calls += 1;
    }
}
