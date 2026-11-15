//! A fake delay that advances a model's time.

use core::cell::RefCell;
use core::fmt;

/// A model whose time can be advanced.
pub trait Clock {
    /// Advances the model's time by `ns` nanoseconds.
    ///
    /// Nanoseconds are the finest unit of [`DelayNs`](embedded_hal::delay::DelayNs), so no
    /// delay is rounded before it reaches the model. A `u64` holds about 584 years. A model
    /// with a coarser time step should carry the remainder over to the next call, or short
    /// delays add up to nothing.
    fn advance_ns(&mut self, ns: u64);
}

/// A delay provider that advances the time of a [`Clock`] instead of waiting.
///
/// Every `delay_ns`, `delay_us` and `delay_ms` call results in exactly one
/// [`advance_ns`](Clock::advance_ns) call with the full duration.
pub struct FakeDelay<'a, M: ?Sized> {
    model: &'a RefCell<M>,
}

impl<'a, M: Clock + ?Sized> FakeDelay<'a, M> {
    /// Creates a delay provider for the specified model.
    pub fn new(model: &'a RefCell<M>) -> Self {
        Self { model }
    }

    fn advance_ns(&mut self, ns: u64) {
        self.model.borrow_mut().advance_ns(ns);
    }
}

// Manual impls: derives would require `M: Clone`, but only the reference is copied.
impl<M: ?Sized> Clone for FakeDelay<'_, M> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<M: ?Sized> Copy for FakeDelay<'_, M> {}

impl<M: fmt::Debug + ?Sized> fmt::Debug for FakeDelay<'_, M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FakeDelay")
            .field("model", &self.model)
            .finish()
    }
}

// The default `delay_us` and `delay_ms` split long delays into several `delay_ns` calls;
// the overrides pass the whole duration at once.
impl<M: Clock + ?Sized> embedded_hal::delay::DelayNs for FakeDelay<'_, M> {
    fn delay_ns(&mut self, ns: u32) {
        self.advance_ns(u64::from(ns));
    }

    fn delay_us(&mut self, us: u32) {
        self.advance_ns(u64::from(us) * 1_000);
    }

    fn delay_ms(&mut self, ms: u32) {
        self.advance_ns(u64::from(ms) * 1_000_000);
    }
}

impl<M: Clock + ?Sized> embedded_hal_async::delay::DelayNs for FakeDelay<'_, M> {
    async fn delay_ns(&mut self, ns: u32) {
        self.advance_ns(u64::from(ns));
    }

    async fn delay_us(&mut self, us: u32) {
        self.advance_ns(u64::from(us) * 1_000);
    }

    async fn delay_ms(&mut self, ms: u32) {
        self.advance_ns(u64::from(ms) * 1_000_000);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_model::Model;
    use embedded_hal::delay::DelayNs;

    #[test]
    fn blocking_delays_advance_the_full_duration() {
        let model = RefCell::new(Model::new(0));
        let mut delay = FakeDelay::new(&model);
        delay.delay_ns(5);
        delay.delay_us(5);
        delay.delay_ms(u32::MAX);
        assert_eq!(
            model.borrow().time_ns,
            5 + 5_000 + u64::from(u32::MAX) * 1_000_000
        );
        assert_eq!(model.borrow().clock_calls, 3);
    }

    #[test]
    fn async_delays_advance_the_full_duration() {
        let model = RefCell::new(Model::new(0));
        let mut delay = FakeDelay::new(&model);
        embassy_futures::block_on(async {
            embedded_hal_async::delay::DelayNs::delay_ns(&mut delay, 5).await;
            embedded_hal_async::delay::DelayNs::delay_us(&mut delay, 5).await;
            embedded_hal_async::delay::DelayNs::delay_ms(&mut delay, 5).await;
        });

        assert_eq!(model.borrow().time_ns, 5 + 5_000 + 5_000_000);
    }
}
