//! A fake I²C bus with one or more targets attached.

use core::cell::RefCell;
use embedded_hal::i2c::{ErrorKind, ErrorType, NoAcknowledgeSource, Operation, SevenBitAddress};

/// A chip model on the target (device) side of an I²C bus.
///
/// [`FakeI2c`] calls the methods in bus order: [`start`](Self::start) for every start or
/// repeated start condition, then [`write`](Self::write) or [`read`](Self::read) for every
/// byte of the transfer, and [`stop`](Self::stop) once at the end of the transaction.
///
/// If the model has inherent methods of the same names, such as `address()`, those win in
/// method-call syntax. Forward to them with a path, `Model::address(self)`, so the impl
/// doesn't read as recursive.
pub trait I2cTarget {
    /// Returns the 7-bit address the target responds to.
    ///
    /// The address may change at runtime, for example when the model emulates an address
    /// pin.
    fn address(&self) -> u8;

    /// Handles a start or repeated start condition addressed to this target and returns
    /// whether the target acknowledges its address.
    ///
    /// `is_read` is the R/W bit: `true` if the controller reads in the following transfer.
    fn start(&mut self, is_read: bool) -> bool;

    /// Receives one byte from the controller and returns whether the target acknowledges
    /// it.
    fn write(&mut self, byte: u8) -> bool;

    /// Sends one byte to the controller.
    fn read(&mut self) -> u8;

    /// Handles the stop condition at the end of a transaction.
    ///
    /// Called after every transaction that reached this target, including transactions
    /// that failed with a NACK. Does nothing by default.
    fn stop(&mut self) {}
}

/// The targets attached to a [`FakeI2c`] bus.
///
/// Implemented for a single model (`&RefCell<M>`), and for arrays and slices of
/// `&RefCell<dyn I2cTarget>` for a shared bus. Implement it yourself to model bus
/// topology such as an I²C multiplexer.
pub trait I2cTargets {
    /// Calls `f` with the target that responds to `address`, or returns `None` if no target
    /// does.
    fn with_target<R>(&self, address: u8, f: impl FnOnce(&mut dyn I2cTarget) -> R) -> Option<R>;
}

impl<T: I2cTargets + ?Sized> I2cTargets for &T {
    fn with_target<R>(&self, address: u8, f: impl FnOnce(&mut dyn I2cTarget) -> R) -> Option<R> {
        (**self).with_target(address, f)
    }
}

impl<M: I2cTarget> I2cTargets for RefCell<M> {
    fn with_target<R>(&self, address: u8, f: impl FnOnce(&mut dyn I2cTarget) -> R) -> Option<R> {
        let mut target = self.borrow_mut();
        (target.address() == address).then(|| f(&mut *target))
    }
}

/// The first target with a matching address responds.
impl I2cTargets for [&RefCell<dyn I2cTarget + '_>] {
    fn with_target<R>(&self, address: u8, f: impl FnOnce(&mut dyn I2cTarget) -> R) -> Option<R> {
        let target = self.iter().find(|t| t.borrow().address() == address)?;
        Some(f(&mut *target.borrow_mut()))
    }
}

/// The first target with a matching address responds.
impl<const N: usize> I2cTargets for [&RefCell<dyn I2cTarget + '_>; N] {
    fn with_target<R>(&self, address: u8, f: impl FnOnce(&mut dyn I2cTarget) -> R) -> Option<R> {
        self.as_slice().with_target(address, f)
    }
}

/// An I²C bus with one or more [`I2cTarget`]s attached.
///
/// Follows the `embedded-hal` 1.0 transaction contract: adjacent operations of the same type
/// form one transfer, so the target sees a repeated start only where the direction changes.
///
/// # Errors
///
/// - [`NoAcknowledgeSource::Address`] if no target has the address, or if the target does
///   not acknowledge a (repeated) start.
/// - [`NoAcknowledgeSource::Data`] if the target does not acknowledge a written byte. The
///   remaining bytes and operations are skipped.
///
/// # Shared bus
///
/// Models of different types share one bus as `&RefCell<dyn I2cTarget>`:
///
/// ```
/// use core::cell::RefCell;
/// use embedded_hal::i2c::{ErrorKind, I2c, NoAcknowledgeSource};
/// use fake_bus::{FakeI2c, I2cTarget};
///
/// /// A touch controller that reports the number of touch points.
/// struct Touch {
///     points: u8,
/// }
///
/// impl I2cTarget for Touch {
///     fn address(&self) -> u8 { 0x15 }
///     fn start(&mut self, _is_read: bool) -> bool { true }
///     fn write(&mut self, _byte: u8) -> bool { true }
///     fn read(&mut self) -> u8 { self.points }
/// }
///
/// /// A sensor that ignores writes while asleep.
/// struct Sensor {
///     awake: bool,
/// }
///
/// impl I2cTarget for Sensor {
///     fn address(&self) -> u8 { 0x1E }
///     fn start(&mut self, _is_read: bool) -> bool { true }
///     fn write(&mut self, _byte: u8) -> bool { self.awake }
///     fn read(&mut self) -> u8 { 0x40 }
/// }
///
/// let touch = RefCell::new(Touch { points: 2 });
/// let sensor = RefCell::new(Sensor { awake: false });
///
/// let targets: [&RefCell<dyn I2cTarget>; 2] = [&touch, &sensor];
/// let mut bus = FakeI2c::new(targets);
///
/// let mut points = [0];
/// bus.read(0x15, &mut points).unwrap();
/// assert_eq!(points, [2]);
///
/// let nack_data = Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data));
/// assert_eq!(bus.write(0x1E, &[0x60]), nack_data);
/// sensor.borrow_mut().awake = true;
/// assert_eq!(bus.write(0x1E, &[0x60]), Ok(()));
///
/// let nack_address = Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address));
/// assert_eq!(bus.read(0x6A, &mut [0]), nack_address);
/// ```
///
/// The bus is `Copy` if `T` is, so each driver can get its own copy. `embedded-hal-bus`
/// sharing works as well.
#[derive(Debug, Clone, Copy)]
pub struct FakeI2c<T> {
    targets: T,
}

impl<T: I2cTargets> FakeI2c<T> {
    /// Creates a bus with the specified targets attached.
    pub fn new(targets: T) -> Self {
        Self { targets }
    }

    /// Returns the targets attached to the bus.
    pub fn targets(&self) -> &T {
        &self.targets
    }

    fn run(&mut self, address: u8, operations: &mut [Operation<'_>]) -> Result<(), ErrorKind> {
        self.targets
            .with_target(address, |target| {
                let result = transfer(target, operations);
                target.stop();
                result
            })
            .unwrap_or(Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)))
    }
}

fn transfer(target: &mut dyn I2cTarget, operations: &mut [Operation<'_>]) -> Result<(), ErrorKind> {
    // Adjacent operations of the same type form one transfer, without a repeated start.
    let mut previous_is_read = None;
    for operation in operations {
        let is_read = matches!(operation, Operation::Read(_));
        if previous_is_read != Some(is_read) && !target.start(is_read) {
            return Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address));
        }

        previous_is_read = Some(is_read);

        match operation {
            Operation::Write(bytes) => {
                if !bytes.iter().all(|&b| target.write(b)) {
                    return Err(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data));
                }
            }

            Operation::Read(buffer) => buffer.iter_mut().for_each(|b| *b = target.read()),
        }
    }

    Ok(())
}

impl<T> ErrorType for FakeI2c<T> {
    type Error = ErrorKind;
}

impl<T: I2cTargets> embedded_hal::i2c::I2c<SevenBitAddress> for FakeI2c<T> {
    fn transaction(
        &mut self,
        address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.run(address, operations)
    }
}

impl<T: I2cTargets> embedded_hal_async::i2c::I2c<SevenBitAddress> for FakeI2c<T> {
    async fn transaction(
        &mut self,
        address: u8,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.run(address, operations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_model::Model;
    use embedded_hal::i2c::I2c;

    const NACK_ADDRESS: ErrorKind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address);
    const NACK_DATA: ErrorKind = ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data);

    #[test]
    fn write_read() {
        let model = RefCell::new(Model::new(0x42));
        let mut bus = FakeI2c::new(&model);
        bus.write(0x42, &[0x10, 1, 2]).unwrap();

        let mut buffer = [0; 2];
        bus.write_read(0x42, &[0x10], &mut buffer).unwrap();
        assert_eq!(buffer, [1, 2]);

        let model = model.borrow();
        assert_eq!(model.starts, 3);
        assert_eq!(model.stops, 2);
    }

    #[test]
    fn adjacent_operations_form_one_transfer() {
        let model = RefCell::new(Model::new(0x42));
        let mut bus = FakeI2c::new(&model);
        bus.transaction(
            0x42,
            &mut [Operation::Write(&[0x10]), Operation::Write(&[7, 8])],
        )
        .unwrap();
        assert_eq!(model.borrow().starts, 1);
        assert_eq!(model.borrow().registers[0x10..0x12], [7, 8]);

        let (mut a, mut b) = ([0], [0]);
        bus.transaction(
            0x42,
            &mut [
                Operation::Write(&[0x10]),
                Operation::Read(&mut a),
                Operation::Read(&mut b),
            ],
        )
        .unwrap();

        assert_eq!((a, b), ([7], [8]));
        assert_eq!(model.borrow().starts, 3);
        assert_eq!(model.borrow().stops, 2);
    }

    #[test]
    fn wrong_address_nacks_without_touching_the_target() {
        let model = RefCell::new(Model::new(0x42));
        let mut bus = FakeI2c::new(&model);
        assert_eq!(bus.write(0x43, &[0x10]), Err(NACK_ADDRESS));
        assert_eq!(model.borrow().starts, 0);
        assert_eq!(model.borrow().stops, 0);
    }

    #[test]
    fn target_nacks_start() {
        let model = RefCell::new(Model::new(0x42));
        model.borrow_mut().ack_start = false;
        let mut bus = FakeI2c::new(&model);
        assert_eq!(bus.read(0x42, &mut [0]), Err(NACK_ADDRESS));
        assert_eq!(model.borrow().stops, 1);
    }

    #[test]
    fn target_nacks_data_and_skips_the_rest() {
        let model = RefCell::new(Model::new(0x42));
        model.borrow_mut().ack_bytes = 2;
        let mut bus = FakeI2c::new(&model);
        assert_eq!(bus.write(0x42, &[0x10, 1, 2, 3]), Err(NACK_DATA));

        let model = model.borrow();
        assert_eq!(model.registers[0x10..0x13], [1, 0, 0]);
        assert_eq!(model.stops, 1);
    }

    #[test]
    fn shared_bus_routes_by_address() {
        let a = RefCell::new(Model::new(0x15));
        let b = RefCell::new(Model::new(0x1E));
        let targets: [&RefCell<dyn I2cTarget>; 2] = [&a, &b];
        let mut bus = FakeI2c::new(targets);

        bus.write(0x1E, &[0x00, 9]).unwrap();
        assert_eq!(b.borrow().registers[0], 9);
        assert_eq!(a.borrow().starts, 0);
        assert_eq!(bus.write(0x6A, &[0x00]), Err(NACK_ADDRESS));

        // A slice works as well.
        let mut bus = FakeI2c::new(&targets[..1]);
        assert_eq!(bus.write(0x1E, &[0x00]), Err(NACK_ADDRESS));
        bus.write(0x15, &[0x00, 5]).unwrap();
        assert_eq!(a.borrow().registers[0], 5);
    }

    #[test]
    fn embedded_hal_bus_sharing() {
        let a = RefCell::new(Model::new(0x15));
        let b = RefCell::new(Model::new(0x1E));
        let bus = RefCell::new(FakeI2c::new([
            &a as &RefCell<dyn I2cTarget>,
            &b as &RefCell<dyn I2cTarget>,
        ]));

        let mut first = embedded_hal_bus::i2c::RefCellDevice::new(&bus);
        let mut second = embedded_hal_bus::i2c::RefCellDevice::new(&bus);
        first.write(0x15, &[0x00, 1]).unwrap();
        second.write(0x1E, &[0x00, 2]).unwrap();
        assert_eq!(a.borrow().registers[0], 1);
        assert_eq!(b.borrow().registers[0], 2);
    }

    #[test]
    fn async_write_read() {
        let model = RefCell::new(Model::new(0x42));
        model.borrow_mut().registers[0x20] = 0xAB;
        let mut bus = FakeI2c::new(&model);

        let mut buffer = [0];
        embassy_futures::block_on(embedded_hal_async::i2c::I2c::write_read(
            &mut bus,
            0x42,
            &[0x20],
            &mut buffer,
        ))
        .unwrap();

        assert_eq!(buffer, [0xAB]);
    }
}
