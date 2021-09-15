use crate::eic::v2::*;
use crate::gpio::v2::InterruptConfig;

//use super::{AnyExtInt, ExtInt};
use super::ExtInt;

//pub struct DebouncedExtInt<E>
pub struct DebouncedExtInt<I, C, AK, S>
where
    //E: AnyExtInt,
    I: GetEINum,
    C: InterruptConfig,
    AK: AnyClock,
    S: SenseMode,
{
    //pub extint: E,
    pub extint: ExtInt<I, C, AK, S>,
}

impl<I, C, AK, S> DebouncedExtInt<I, C, AK, S>
where
    I: GetEINum,
    C: InterruptConfig,
    AK: AnyClock,
    S: SenseMode,
{
    // Do not need access to the EIController here
    /// Read the pin state of the ExtInt
    /// TODO
    pub fn pin_state(&self) -> bool {
        self.extint.pin_state()
    }

    /// TODO
    pub fn disable_debouncing<N>(
        self,
        eic: &mut Enabled<EIController<WithClock<AK::ClockSource>>, N>,
    ) -> ExtInt<I, C, AK, S>
    where
        N: Counter,
    {
        // Could pass the MASK directly instead of making this function
        // generic over the EINum. Either way is fine.
        eic.disable_debouncing::<I::EINum>();
        // Return the inner ExtInt<...>
        self.extint
    }

    /// TODO
    pub fn set_debouncer_settings<N>(
        &self,
        eic: &mut Enabled<EIController<WithClock<AK::ClockSource>>, N>,
        settings: &DebouncerSettings,
    ) where
        N: Counter,
    {
        // Could pass the MASK directly instead of making this function
        // generic over the EINum. Either way is fine.
        eic.set_debouncer_settings::<I::EINum>(settings);
    }
}
