use modular_bitfield::prelude::*;

#[bitfield]
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub struct ConfigReg {
    #[bits = 1]
    shutdown: Shutdown,
    #[bits = 1]
    comp_int: CompInt,
    #[bits = 1]
    alert_polarity: AlertPolarity,
    #[bits = 2]
    fault_queue: FaultQueue,
    #[bits = 2]
    resolution: Resolution,
    #[bits = 1]
    one_shot: OneShot,
}

macro_rules! impl_field {
    ( $doc:expr, $type:ident, $first:ident $(, $subseq:ident )* ) => {
        #[doc = $doc]
        #[doc = " bit(s) in the Sensor Configuration Register.\n"]
        #[doc = "Consult the TCN75A [datasheet] for information on the meanings of each option.\n"]
        #[doc = "[datasheet]: http://ww1.microchip.com/downloads/en/DeviceDoc/21935D.pdf"]
        #[derive(BitfieldSpecifier, Debug, PartialEq)]
        pub enum $type {
            $first = 0,
            $(
                $subseq
            ),*
        }
    }
}

impl_field!("One-Shot", OneShot, Disabled, Enabled);
impl_field!("ADC Resolution", Resolution, Bits9, Bits10, Bits11, Bits12);
impl_field!("Fault Queue", FaultQueue, One, Two, Four, Six);
impl_field!("Alert Polarity", AlertPolarity, ActiveLow, ActiveHigh);
impl_field!("Comp/Int", CompInt, Comparator, Interrupt);
impl_field!("Shutdown", Shutdown, Disable, Enable);

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::TryInto;
    use core::mem::size_of;

    #[test]
    fn test_size() {
        assert_eq!(size_of::<ConfigReg>(), 1);
    }

    #[test]
    fn test_two_fields() {
        let mut cfg : ConfigReg = Default::default();
        cfg.set_shutdown(Shutdown::Disable);
        cfg.set_comp_int(CompInt::Interrupt);

        let val = u8::from_le_bytes(cfg.to_bytes().try_into().unwrap());

        assert_eq!(val, 0b0000010);
    }

    #[test]
    fn test_2bit_val() {
        let mut cfg : ConfigReg = Default::default();
        cfg.set_resolution(Resolution::Bits12);
        cfg.set_fault_queue(FaultQueue::Six);

        let val = u8::from_le_bytes(cfg.to_bytes().try_into().unwrap());
        assert_eq!(val, 0b01111000);
    }

    #[test]
    fn test_reset_defaults() {
        let cfg : ConfigReg = Default::default();

        assert_eq!(cfg.get_shutdown(), Shutdown::Disable);
        assert_eq!(cfg.get_comp_int(), CompInt::Comparator);
        assert_eq!(cfg.get_alert_polarity(), AlertPolarity::ActiveLow);
        assert_eq!(cfg.get_resolution(), Resolution::Bits9);
        assert_eq!(cfg.get_fault_queue(), FaultQueue::One);
        assert_eq!(cfg.get_one_shot(), OneShot::Disabled);

        let val = u8::from_le_bytes(cfg.to_bytes().try_into().unwrap());
        assert_eq!(val, 0);
    }
}
