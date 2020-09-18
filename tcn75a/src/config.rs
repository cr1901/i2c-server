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
            mask: self.mask | rhs_proxy.mask,
        }

        // Was a typo, but weirdly enough doesn't compile... errors with:
        // cannot infer type for type parameter `T`
        // ConfigRegProxy {
        //     val: self.into().val | rhs_proxy.val,
        //     mask: self.into().mask | rhs_proxy.mask
        // }
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
            mask: T::mask(),
        }
    }
}

pub trait ConfigRegField: private::Sealed + Into<u8> {
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

macro_rules! impl_field {
    ( $type:ident, $width:expr, $offset:expr, $first:ident $(, $subseq:ident )* ) => {
        #[repr(u8)]
        pub enum $type {
            $first = 0,
            $(
                $subseq
            ),*
        }

        impl ConfigRegField for $type {
            const WIDTH: u8 = $width;
            const OFFSET: u8 = $offset;
        }

        impl From<$type> for u8
        {
            fn from(field: $type) -> u8 {
                field as u8
            }
        }

        impl<T> BitOr<T> for $type where T: ConfigRegField {
            type Output = ConfigRegProxy;

            fn bitor(self, rhs: T) -> Self::Output {
                ConfigRegProxy {
                    val: self.val() | rhs.val(),
                    mask: Self::mask() | T::mask()
                }
            }
        }
    }
}

impl_field!(OneShot, 1, 7, Disabled, Enabled);
impl_field!(Resolution, 2, 5, Bits9, Bits10, Bits11, Bits12);
impl_field!(FaultQueue, 2, 3, One, Two, Four, Six);
impl_field!(AlertPolarity, 1, 2, ActiveLow, ActiveHigh);
impl_field!(CompInt, 1, 1, Comparator, Interrupt);
impl_field!(Shutdown, 1, 0, Disable, Enable);

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
    fn test_val_and_mask() {
        let proxy = Shutdown::Disable | CompInt::Interrupt;

        assert_eq!(proxy.val, 0b0000010);
        assert_eq!(proxy.mask, 0b00000011);
    }

    #[test]
    fn test_2bit_val() {
        let proxy: ConfigRegProxy = Resolution::Bits12.into();

        assert_eq!(proxy.val, 0b01100000);
        assert_eq!(proxy.mask, 0b01100000);
    }

    #[test]
    fn test_reset_defaults() {
        let proxy = OneShot::Disabled
            | Resolution::Bits9
            | FaultQueue::One
            | AlertPolarity::ActiveLow
            | CompInt::Comparator
            | Shutdown::Disable;

        assert_eq!(proxy.val, 0);
        assert_eq!(proxy.mask, 0b11111111);
    }
}
