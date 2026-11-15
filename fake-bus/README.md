# fake-bus

[![Crates.io][crates-image]][crates-link]
[![Docs][docs-image]][docs-link]
[![Build Status][build-image]][build-link]
[![Coverage][codecov-image]][codecov-link]
[![Safety Dance][safety-image]][safety-link]
![MSRV][msrv-image]
[![EUPL 1.2 licensed][license-eupl-image]][license-eupl-link]
[![Apache 2.0 licensed][license-apache-image]][license-apache-link]
[![MIT licensed][license-mit-image]][license-mit-link]

> Fake `embedded-hal` 1.0 I²C, SPI and delay implementations that drive stateful chip
> models in host tests.

You write a behavioral model of the chip; the fakes put it on a bus:

- `FakeI2c`: blocking and async `I2c` on one `I2cTarget`, or on several targets sharing
  one bus (address routing, address and data NACKs). Adjacent operations of the same type
  form one transfer, as `embedded-hal` 1.0 requires, so the model sees a repeated start
  only where the direction changes.
- `FakeSpi`: blocking and async `SpiDevice` on an `SpiTarget`, one chip-select cycle per
  transaction.
- `FakeDelay`: blocking and async `DelayNs` that advances a `Clock` (in nanoseconds)
  instead of waiting.
- `no_std`, no `unsafe`, no heap. Models sit in a `RefCell`, so tests can inspect and
  change them while a driver owns the bus.

Unlike [`embedded-hal-mock`](https://crates.io/crates/embedded-hal-mock), which checks a
script of expected transactions, the model decides how to respond.

```rust,ignore
use core::cell::RefCell;
use fake_bus::{FakeDelay, FakeI2c, I2cTarget};

let touch = RefCell::new(TouchModel::new());
let compass = RefCell::new(CompassModel::new());

// One shared bus with two chips at different addresses.
let targets: [&RefCell<dyn I2cTarget>; 2] = [&touch, &compass];
let bus = FakeI2c::new(targets);

let mut driver = Compass::new(bus);
compass.borrow_mut().set_field([100, 0, -50]);
let field = driver.read_magnetic_field(&mut FakeDelay::new(&compass))?;
```

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

[crates-image]: https://img.shields.io/crates/v/fake-bus
[crates-link]: https://crates.io/crates/fake-bus
[docs-image]: https://docs.rs/fake-bus/badge.svg
[docs-link]: https://docs.rs/fake-bus/
[build-image]: https://github.com/sunsided/fake-bus/workflows/Rust/badge.svg
[build-link]: https://github.com/sunsided/fake-bus/actions
[codecov-image]: https://codecov.io/gh/sunsided/fake-bus/graph/badge.svg
[codecov-link]: https://codecov.io/gh/sunsided/fake-bus
[safety-image]: https://img.shields.io/badge/unsafe-forbidden-success.svg
[safety-link]: https://github.com/rust-secure-code/safety-dance/
[msrv-image]: https://img.shields.io/badge/rustc-1.85+-blue.svg
[license-eupl-image]: https://img.shields.io/badge/license-EUPL_1.2-blue.svg
[license-eupl-link]: https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12
[license-apache-image]: https://img.shields.io/badge/license-Apache_2.0-blue.svg
[license-apache-link]: https://www.apache.org/licenses/LICENSE-2.0
[license-mit-image]: https://img.shields.io/badge/license-MIT-blue.svg
[license-mit-link]: https://opensource.org/licenses/MIT
