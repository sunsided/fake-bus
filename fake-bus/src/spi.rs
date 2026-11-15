//! A fake SPI device.

use core::cell::RefCell;
use core::fmt;
use embedded_hal::spi::{ErrorKind, ErrorType, Operation};

/// A chip model on the peripheral side of an SPI bus.
///
/// [`FakeSpi`] calls [`select`](Self::select) when chip select asserts,
/// [`transfer`](Self::transfer) for every clocked byte, and [`deselect`](Self::deselect)
/// when chip select deasserts.
pub trait SpiTarget {
    /// Handles chip select asserting at the start of a transaction.
    fn select(&mut self);

    /// Handles chip select deasserting at the end of a transaction. Does nothing by
    /// default.
    fn deselect(&mut self) {}

    /// Exchanges one byte: receives `mosi` and returns the byte the target shifts out on
    /// MISO at the same time.
    fn transfer(&mut self, mosi: u8) -> u8;
}

/// An SPI device (bus plus chip select) with an [`SpiTarget`] attached.
///
/// Every transaction is one chip-select cycle. [`Operation::Read`] clocks out `0x00`;
/// [`Operation::Transfer`] with buffers of unequal length clocks the longer length, pads
/// MOSI with `0x00` and discards surplus MISO bytes, as the `embedded-hal` contract states.
/// [`Operation::DelayNs`] does not advance any clock; use [`FakeDelay`](crate::FakeDelay)
/// for time.
pub struct FakeSpi<'a, M: ?Sized> {
    model: &'a RefCell<M>,
}

impl<'a, M: SpiTarget + ?Sized> FakeSpi<'a, M> {
    /// Creates a device with the specified model attached.
    pub fn new(model: &'a RefCell<M>) -> Self {
        Self { model }
    }

    fn run(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), ErrorKind> {
        let mut model = self.model.borrow_mut();
        model.select();

        for operation in operations {
            match operation {
                Operation::Write(bytes) => bytes.iter().for_each(|&b| {
                    model.transfer(b);
                }),

                Operation::Read(buffer) => buffer.iter_mut().for_each(|b| *b = model.transfer(0)),

                Operation::Transfer(read, write) => {
                    for i in 0..read.len().max(write.len()) {
                        let miso = model.transfer(write.get(i).copied().unwrap_or(0));
                        if let Some(b) = read.get_mut(i) {
                            *b = miso;
                        }
                    }
                }

                Operation::TransferInPlace(buffer) => {
                    buffer.iter_mut().for_each(|b| *b = model.transfer(*b))
                }

                Operation::DelayNs(_) => {}
            }
        }

        model.deselect();
        Ok(())
    }
}

// Manual impls: derives would require `M: Clone`, but only the reference is copied.
impl<M: ?Sized> Clone for FakeSpi<'_, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: ?Sized> Copy for FakeSpi<'_, M> {}

impl<M: fmt::Debug + ?Sized> fmt::Debug for FakeSpi<'_, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FakeSpi")
            .field("model", &self.model)
            .finish()
    }
}

impl<M: ?Sized> ErrorType for FakeSpi<'_, M> {
    type Error = ErrorKind;
}

impl<M: SpiTarget + ?Sized> embedded_hal::spi::SpiDevice for FakeSpi<'_, M> {
    fn transaction(&mut self, operations: &mut [Operation<'_, u8>]) -> Result<(), Self::Error> {
        self.run(operations)
    }
}

impl<M: SpiTarget + ?Sized> embedded_hal_async::spi::SpiDevice for FakeSpi<'_, M> {
    async fn transaction(
        &mut self,
        operations: &mut [Operation<'_, u8>],
    ) -> Result<(), Self::Error> {
        self.run(operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_model::{Model, SPI_READ};
    use embedded_hal::spi::SpiDevice;

    #[test]
    fn one_chip_select_cycle_per_transaction() {
        let model = RefCell::new(Model::new(0));
        let mut spi = FakeSpi::new(&model);
        spi.write(&[0x10, 1, 2, 3]).unwrap();

        let mut buffer = [0; 3];
        spi.transaction(&mut [
            Operation::Write(&[SPI_READ | 0x10]),
            Operation::Read(&mut buffer),
        ])
        .unwrap();

        assert_eq!(buffer, [1, 2, 3]);
        assert_eq!(model.borrow().selects, 2);
        assert_eq!(model.borrow().deselects, 2);
    }

    #[test]
    fn transfer_in_place() {
        let model = RefCell::new(Model::new(0));
        model.borrow_mut().registers[0x20] = 0xAB;
        let mut spi = FakeSpi::new(&model);

        let mut buffer = [SPI_READ | 0x20, 0];
        spi.transfer_in_place(&mut buffer).unwrap();
        assert_eq!(buffer[1], 0xAB);
    }

    #[test]
    fn transfer_with_unequal_lengths() {
        let model = RefCell::new(Model::new(0));
        model.borrow_mut().registers[0x20..0x23].copy_from_slice(&[1, 2, 3]);
        let mut spi = FakeSpi::new(&model);

        // Longer read buffer: MOSI is padded with zeros.
        let mut read = [0; 4];
        spi.transfer(&mut read, &[SPI_READ | 0x20]).unwrap();
        assert_eq!(read[1..], [1, 2, 3]);

        // Longer write buffer: surplus MISO bytes are discarded.
        let mut read = [0; 1];
        spi.transfer(&mut read, &[0x20, 7, 8]).unwrap();
        assert_eq!(model.borrow().registers[0x20..0x22], [7, 8]);
    }

    #[test]
    fn delay_is_ignored() {
        let model = RefCell::new(Model::new(0));
        let mut spi = FakeSpi::new(&model);
        spi.transaction(&mut [Operation::DelayNs(1_000)]).unwrap();
        assert_eq!(model.borrow().time_ns, 0);
        assert_eq!(model.borrow().selects, 1);
    }

    #[test]
    fn async_transfer_in_place() {
        let model = RefCell::new(Model::new(0));
        model.borrow_mut().registers[0x20] = 0xAB;
        let mut spi = FakeSpi::new(&model);

        let mut buffer = [SPI_READ | 0x20, 0];
        embassy_futures::block_on(embedded_hal_async::spi::SpiDevice::transfer_in_place(
            &mut spi,
            &mut buffer,
        ))
        .unwrap();

        assert_eq!(buffer[1], 0xAB);
    }
}
