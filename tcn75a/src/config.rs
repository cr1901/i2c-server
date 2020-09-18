use core::ops::BitOr;

pub struct ConfigReg(u8);
pub struct ConfigRegProxy {
    val: u8,
    mask: u8,
}

impl ConfigRegProxy {
    pub(crate) fn modify(self, old: ConfigReg) -> ConfigReg {
        todo!()
    }
}

impl<T> BitOr<T> for ConfigRegProxy
where
    T: Into<ConfigRegProxy>,
{
    type Output = ConfigRegProxy;

    fn bitor(self, rhs: T) -> Self::Output {
        let rhs_proxy = rhs.into();

        ConfigRegProxy {
            val: self.val | rhs_proxy.val,
            mask: self.mask | rhs_proxy.mask
        }
        // Was a typo, but weirdly enough doesn't compile...
        // ConfigReg(self.into().0 | rhs.into().0)
    }
}

pub trait ConfigRegField : private::Sealed + Into<u8> {
    // Use associated const to get the mask.
    const WIDTH: u8;
    const OFFSET: u8;

    fn val(self) -> u8 {
        self.into() << Self::OFFSET
    }

    fn mask() -> u8 {
        ((1u8 << Self::WIDTH) - 1) << Self::OFFSET
    }
}

// impl for ConfigRegProxy, which includes mask info
impl<T> From<T> for ConfigRegProxy
where
    T: ConfigRegField,
{
    fn from(field: T) -> Self {
        ConfigRegProxy {
            val: field.val(),
            mask: T::mask()
        }
    }
}

#[repr(u8)]
pub enum OneShot {
    Enabled,
    Disabled,
}

#[repr(u8)]
pub enum Resolution {
    Bits9,
    Bits10,
    Bits11,
    Bits12,
}

#[repr(u8)]
pub enum FaultQueue {
    One,
    Two,
    Four,
    Six,
}

#[repr(u8)]
pub enum AlertPolarity {
    ActiveHigh,
    ActiveLow,
}

#[repr(u8)]
pub enum CompInt {
    Comparator,
    Interrupt,
}

#[repr(u8)]
pub enum Shutdown {
    Disable,
    Enable,
}

impl ConfigRegField for OneShot {
    const WIDTH: u8 = 1;
    const OFFSET: u8 = 7;
}

impl ConfigRegField for Resolution {
    const WIDTH: u8 = 2;
    const OFFSET: u8 = 5;
}

impl ConfigRegField for FaultQueue {
    const WIDTH: u8 = 2;
    const OFFSET: u8 = 3;
}

impl ConfigRegField for AlertPolarity {
    const WIDTH: u8 = 1;
    const OFFSET: u8 = 2;
}

impl ConfigRegField for CompInt {
    const WIDTH: u8 = 1;
    const OFFSET: u8 = 1;
}

impl ConfigRegField for Shutdown {
    const WIDTH: u8 = 1;
    const OFFSET: u8 = 0;
}

impl From<OneShot> for u8
{
    fn from(field: OneShot) -> u8 {
        field as u8
    }
}

impl From<Resolution> for u8
{
    fn from(field: Resolution) -> u8 {
        field as u8
    }
}

impl From<FaultQueue> for u8
{
    fn from(field: FaultQueue) -> u8 {
        field as u8
    }
}

impl From<AlertPolarity> for u8
{
    fn from(field: AlertPolarity) -> u8 {
        field as u8
    }
}

impl From<CompInt> for u8
{
    fn from(field: CompInt) -> u8 {
        field as u8
    }
}

impl From<Shutdown> for u8
{
    fn from(field: Shutdown) -> u8 {
        field as u8
    }
}


mod private {
    pub trait Sealed {}

    // Implement for those same types, but no others.
    impl Sealed for super::OneShot {}
    impl Sealed for super::Resolution {}
    impl Sealed for super::FaultQueue {}
    impl Sealed for super::AlertPolarity {}
    impl Sealed for super::CompInt {}
    impl Sealed for super::Shutdown {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask() {
        let proxy = Shutdown | CompInt;

        assert_eq!(proxy.mask, 0b00000011);
    }
}
