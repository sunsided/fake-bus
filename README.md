# fake-bus

> Fake `embedded-hal` 1.0 buses that drive stateful chip models in host tests.

| Crate                  | Description                                                                 |
|------------------------|-----------------------------------------------------------------------------|
| [`fake-bus`](fake-bus) | Fake I²C buses (single or shared), SPI devices and delays for chip models. |

Driver crates pair this with a behavioral model of their chip (a `<chip>-sim` crate), so
tests exercise the driver against something that keeps state, instead of a script of
expected transactions.

## License

Copyright © 2026 Markus Mayer

Triple licensed under your choice of either of:

- European Union Public Licence, Version 1.2, ([LICENSE-EUPL](LICENSE-EUPL)
  or https://joinup.ec.europa.eu/collection/eupl/eupl-text-eupl-12)
- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
