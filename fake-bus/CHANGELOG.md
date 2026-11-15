# Changelog

All notable changes to this project will be documented in this file.
This project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `I2cTarget`, `SpiTarget` and `Clock` traits for chip models.
- `FakeI2c`: blocking and async `embedded-hal` I²C bus on one model or on several models
  sharing a bus (arrays and slices of `&RefCell<dyn I2cTarget>`), with address and data
  NACKs and repeated starts only where the transfer direction changes.
- `FakeSpi`: blocking and async `SpiDevice`, one chip-select cycle per transaction.
- `FakeDelay`: blocking and async `DelayNs` that advances the model's clock in nanoseconds.
- `Ticks` (with the `Micros` and `Millis` aliases): converts nanoseconds into the whole
  ticks of a coarser model clock, carrying the remainder, optionally as `u32` chunks.
- Re-exports of `embedded_hal` and `embedded_hal_async`.
