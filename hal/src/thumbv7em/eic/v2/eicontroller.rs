use core::marker::PhantomData;

use bitfield::*;

use typenum::U0;

use crate::clock::types::{Counter, Decrement, Enabled, Increment};
use crate::gpio::v2::{Interrupt, InterruptConfig, Pin};

use crate::eic::v2::*;

use super::extint::*;

//==============================================================================
// EIController
//==============================================================================
// Struct to represent the external interrupt controller
// You need exclusive access to this to set registers that
// share multiple pins, like the Sense configuration register
/// TODO
pub struct EIController<AK, EP>
where
    AK: AnyClock,
    EP: EnableProtected,
{
    eic: crate::pac::EIC,
    // Config consists of two 32-bit registers with the same layout
    // config.0 covers [`EInum`] 0 to 7, config.1 [`EInum`] 8 to 15
    config: [EIConfigReg; 2],
    _clockmode: PhantomData<AK>,
    _enablestate: PhantomData<EP>,
}

impl<CS> EIController<WithClock<CS>, Configurable>
where
    CS: EIClkSrc + Increment,
{
    /// Create an EIC Controller with a clock source
    ///
    /// This allows for full EIC functionality
    ///
    /// Safety
    ///
    /// Safe because you trade a singleton PAC struct for new singletons
    pub fn new(
        eic: crate::pac::EIC,
        clock: CS,
    ) -> (
        Enabled<EIController<WithClock<CS>, Configurable>, U0>,
        Tokens,
        CS::Inc,
    ) {
        // Software reset the EIC controller on creation
        eic.ctrla.modify(|_, w| w.swrst().set_bit());
        while eic.syncbusy.read().swrst().bit_is_set() {
            cortex_m::asm::nop();
        }

        // Set CKSEL to match the clock resource provided
        eic.ctrla.modify(|_, w| w.cksel().variant(CS::CKSEL));

        unsafe {
            (
                Enabled::new(Self {
                    eic,
                    // Create config register, matching reset state
                    config: [EIConfigReg(0), EIConfigReg(0)],
                    _clockmode: PhantomData,
                    _enablestate: PhantomData,
                }),
                Tokens::new(),
                clock.inc(),
            )
        }
    }
}

impl EIController<NoClock, Configurable> {
    /// Create an EIC Controller without a clock source
    ///
    /// This limits the EIC functionality
    ///
    /// Safety
    ///
    /// Safe because you trade a singleton PAC struct for new singletons
    pub fn new_only_async(
        eic: crate::pac::EIC,
    ) -> (Enabled<EIController<NoClock, Configurable>, U0>, Tokens) {
        // Software reset the EIC controller on creation
        eic.ctrla.modify(|_, w| w.swrst().set_bit());
        while eic.syncbusy.read().swrst().bit_is_set() {
            cortex_m::asm::nop();
        }

        // Setup mode to async for all channels
        eic.asynch.write(|w| unsafe { w.bits(0xFFFF) });

        // Does not use or need any external clock, `CKSEL` is ignored

        unsafe {
            (
                Enabled::new(Self {
                    eic,
                    // Create config register, matching reset state
                    config: [EIConfigReg(0), EIConfigReg(0)],
                    _clockmode: PhantomData,
                    _enablestate: PhantomData,
                }),
                Tokens::new(),
            )
        }
    }
}

impl<K> Enabled<EIController<K, Configurable>, U0>
where
    K: AnyClock,
{
    /// Software reset needs to be synchronised
    fn syncbusy_swrst(&self) {
        while self.0.eic.syncbusy.read().swrst().bit_is_set() {
            cortex_m::asm::nop();
        }
    }
}

impl<AK, EP, N> Enabled<EIController<AK, EP>, N>
where
    AK: AnyClock,
    EP: EnableProtected,
    N: Counter,
{
    /// Enabling the EIC controller needs to be synchronised
    fn syncbusy_enable(&self) {
        while self.0.eic.syncbusy.read().enable().bit_is_set() {
            cortex_m::asm::nop();
        }
    }
}

impl<AK, N> Enabled<EIController<AK, Configurable>, N>
where
    AK: AnyClock,
    N: Counter,
{
    /// TODO
    ///
    /// Currently unused
    pub(super) fn set_sense_mode<E: EINum>(&mut self, sense: Sense) {
        let index: usize = E::OFFSET.into();
        let msb: usize = E::SENSEMSB.into();
        let lsb: usize = E::SENSELSB.into();
        // Set the SENSE bits in the configuration state
        self.0.config[index].set_bit_range(msb, lsb, sense as u8);
        // Write the configuration state to hardware
        //set_sense!(self, index, msb, lsb, einum);
        self.0.eic.config[index]
            .write(|w| unsafe { w.bits(self.0.config[index].bit_range(31, 0)) });
    }

    /// Start EIC controller by writing the enable bit
    /// this "finalizes" the configuration phase
    pub fn finalize(self) -> Enabled<EIController<AK, Protected>, N> {
        self.0.eic.ctrla.modify(|_, w| w.enable().set_bit());
        self.syncbusy_enable();

        Enabled::new(EIController {
            eic: self.0.eic,
            config: self.0.config,
            _clockmode: self.0._clockmode,
            _enablestate: PhantomData,
        })
    }
}

impl<AK, N> Enabled<EIController<AK, Protected>, N>
where
    AK: AnyClock,
    N: Counter,
{
    pub fn disable(self) -> Enabled<EIController<AK, Configurable>, N> {
        self.0.eic.ctrla.modify(|_, w| w.enable().clear_bit());
        self.syncbusy_enable();

        Enabled::new(EIController {
            eic: self.0.eic,
            config: self.0.config,
            _clockmode: self.0._clockmode,
            _enablestate: PhantomData,
        })
    }
}

impl<AK> Enabled<EIController<AK, Configurable>, U0>
where
    AK: AnyClock,
{
    /// Softare reset the EIC controller
    ///
    /// Will clear all registers and leave the controller disabled
    /// Return the same kind that was configured previously
    /// #TODO, not verified, broken, disable for now
    pub fn swrst(mut self) -> Self {
        self.0.eic.ctrla.modify(|_, w| w.swrst().set_bit());
        // Wait until done
        self.syncbusy_swrst();

        // Reset any stored state to default reset values
        self.0.config = [EIConfigReg(0), EIConfigReg(0)];
        self
    }
}

impl<CS> Enabled<EIController<WithClock<CS>, Configurable>, U0>
where
    CS: EIClkSrc + Decrement,
{
    /// Disable and destroy the EIC controller
    pub fn destroy<S>(self, _tokens: Tokens, clock: CS) -> (crate::pac::EIC, CS::Dec)
    where
        S: EIClkSrc + Decrement,
    {
        (self.0.eic, clock.dec())
    }
}

impl Enabled<EIController<NoClock, Configurable>, U0> {
    /// Disable and destroy the EIC controller
    pub fn destroy(self, _tokens: Tokens) -> crate::pac::EIC {
        self.0.eic
    }
}

impl<CS, N> Enabled<EIController<WithClock<CS>, Configurable>, N>
where
    CS: EIClkSrc,
    N: Counter,
{
    /// TODO
    pub fn new_sync<I, C>(
        &self,
        token: Token<I::EINum>,
        pin: Pin<I, Interrupt<C>>,
    ) -> ExtInt<I, C, WithClock<CS>, SenseNone>
    where
        I: GetEINum,
        C: InterruptConfig,
    {
        ExtInt::new_sync(token, pin)
    }

    // Private function that should be accessed through the ExtInt
    // Could pass the MASK directly instead of making this function
    // generic over the EINum. Either way is fine.
    /// TODO
    pub(super) fn enable_debouncing<E: EINum>(&mut self) {
        self.0.eic.debouncen.modify(|r, w| unsafe {
            let bits = r.debouncen().bits();
            w.debouncen().bits(bits | E::MASK)
        });
    }

    /// TODO
    pub(super) fn disable_debouncing<E: EINum>(&mut self) {
        self.0.eic.debouncen.modify(|r, w| unsafe {
            let bits = r.debouncen().bits();
            // Clear specific bit
            w.debouncen().bits(bits & !(E::MASK))
        });
    }

    pub(super) fn set_debouncer_settings<E: EINum>(&mut self, settings: &DebouncerSettings) {
        self.0.eic.dprescaler.write({
            |w| {
                w.tickon()
                    .variant(settings.tickon)
                    .prescaler0()
                    .variant(settings.prescaler0)
                    .states0()
                    .variant(settings.states0)
                    .prescaler1()
                    .variant(settings.prescaler1)
                    .states1()
                    .variant(settings.states1)
            }
        });
    }

    // Private function that should be accessed through the ExtInt
    /// TODO
    pub(super) fn enable_filtering<E: EINum>(&mut self) {
        let index: usize = E::OFFSET.into();
        let bitnum: usize = E::FILTEN.into();

        // Set the FILTEN bit in the configuration state
        self.0.config[index].set_bit(bitnum, true);
        // Write the configuration state to hardware
        self.0.eic.config[index]
            .write(|w| unsafe { w.bits(self.0.config[index].bit_range(31, 0)) });
    }

    /// TODO
    pub(super) fn disable_filtering<E: EINum>(&mut self) {
        let index: usize = E::OFFSET.into();
        let bitnum: usize = E::FILTEN.into();

        // Set the FILTEN bit in the configuration state
        self.0.config[index].set_bit(bitnum, false);
        // Write the configuration state to hardware
        self.0.eic.config[index]
            .write(|w| unsafe { w.bits(self.0.config[index].bit_range(31, 0)) });
    }
}
impl<K, N> Enabled<EIController<K, Configurable>, N>
where
    K: AnyClock,
    N: Counter,
{
    /// TODO
    pub fn new_async<I, C>(
        &self,
        token: Token<I::EINum>,
        pin: Pin<I, Interrupt<C>>,
    ) -> AsyncExtInt<I, C, NoClock, SenseNone>
    where
        I: GetEINum,
        C: InterruptConfig,
    {
        ExtInt::new_async(token, pin)
    }
}
