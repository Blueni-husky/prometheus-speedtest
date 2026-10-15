use core::fmt;

pub trait SerializeGoFloat {
    fn serialize_go_float<W: fmt::Write>(&self, write: &mut W) -> fmt::Result;
}

macro_rules! display_impl {
    ($Type:ty) => {
        impl SerializeGoFloat for $Type {
            #[inline]
            fn serialize_go_float<W: fmt::Write>(&self, write: &mut W) -> fmt::Result
            {
                write!(write, "{self}")
            }
        }
    };
    ($($Type:ty),*) => {$(display_impl!{$Type})*};
}

macro_rules! delegate_impl {
    ($Type:ty => $impl:path) => {
        impl SerializeGoFloat for $Type {
            #[inline]
            fn serialize_go_float<W: fmt::Write>(&self, write: &mut W) -> fmt::Result {
                $impl(*self, write)
            }
        }
    };
    ($($Type:ty => $impl:path),*) => {$(delegate_impl!{$Type => $impl})*};
}

display_impl!(u8, i8, u16, i16, u32, i32, u64, i64, usize, isize);
delegate_impl!(f32 => f32_to_go_string, f64 => f64_to_go_string);

impl SerializeGoFloat for bool {
    #[inline]
    fn serialize_go_float<W: fmt::Write>(&self, write: &mut W) -> fmt::Result {
        if *self {
            write.write_char('1')
        } else {
            write.write_char('0')
        }
    }
}

// Prometheus' text parser does not accept Go's hexadecimal float notation,
// so floats are written in the shortest decimal form that round-trips.
macro_rules! to_go_string_impl {
    ($fname:ident, $Type:ty) => {
        fn $fname(float: $Type, out: &mut impl fmt::Write) -> fmt::Result {
            if float.is_nan() {
                return out.write_str("NaN");
            }
            if float.is_infinite() {
                return out.write_str(if float.is_sign_positive() {
                    "+Inf"
                } else {
                    "-Inf"
                });
            }
            if float == 0. {
                return out.write_char('0');
            }

            let abs = float.abs();
            if abs >= 1e21 || abs < 1e-7 {
                write!(out, "{float:e}")
            } else {
                write!(out, "{float}")
            }
        }
    };
}

to_go_string_impl!(f32_to_go_string, f32);
to_go_string_impl!(f64_to_go_string, f64);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_f32_to_go_string() {
        let mut buf = String::new();
        // Exponent 0xfe -> 2^(0xfe - 127) = 2^127
        let float = f32::from_bits(0x7f_00_00_00);
        f32_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "1.7014118e38");
    }

    #[test]
    fn large_f64_to_go_string() {
        let mut buf = String::new();
        // Exponent 0x7fe -> 2^(0x7fe - 1023) = 2^1023
        let float = f64::from_bits(0x7f_e0_00_00_00_00_00_00);
        f64_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "8.98846567431158e307");
    }

    #[test]
    fn small_f32_to_go_string() {
        let mut buf = String::new();
        f32_to_go_string(0.15625, &mut buf).unwrap();
        assert_eq!(buf, "0.15625");
        buf.clear();
        f32_to_go_string(-0.15625, &mut buf).unwrap();
        assert_eq!(buf, "-0.15625");
    }

    #[test]
    fn small_f64_to_go_string() {
        let mut buf = String::new();
        f64_to_go_string(0.15625, &mut buf).unwrap();
        assert_eq!(buf, "0.15625");
        buf.clear();
        f64_to_go_string(-0.15625, &mut buf).unwrap();
        assert_eq!(buf, "-0.15625");
    }

    #[test]
    fn subnormal_f32_to_go_string() {
        let mut buf = String::new();
        // Exponent 2^(0-127)
        let float = f32::from_bits(1);
        assert!(float.is_sign_positive() && float.is_subnormal());
        f32_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "1e-45");
    }

    #[test]
    fn subnormal_f64_to_go_string() {
        let mut buf = String::new();
        // Exponent 2^(0-1023)
        let float = f64::from_bits(1);
        assert!(float.is_sign_positive() && float.is_subnormal());
        f64_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "5e-324");
    }

    #[test]
    fn nan_f32_to_go_string() {
        let mut buf = String::new();
        let float = f32::from_bits(0x7f_80_ca_fe);
        assert!(float.is_nan());
        f32_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "NaN");
    }

    #[test]
    fn nan_f64_to_go_string() {
        let mut buf = String::new();
        let float = f64::from_bits(0x7f_f0_00_00_ca_fe_ba_be);
        assert!(float.is_nan());
        f64_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "NaN");
    }

    #[test]
    fn one_f32_to_go_string() {
        let mut buf = String::new();
        f32_to_go_string(1., &mut buf).unwrap();
        assert_eq!(buf, "1");
    }

    #[test]
    fn one_f64_to_go_string() {
        let mut buf = String::new();
        f64_to_go_string(1., &mut buf).unwrap();
        assert_eq!(buf, "1");
    }

    #[test]
    fn zero_f32_to_go_string() {
        let mut buf = String::new();
        f32_to_go_string(0., &mut buf).unwrap();
        assert_eq!(buf, "0");
    }

    #[test]
    fn zero_f64_to_go_string() {
        let mut buf = String::new();
        f64_to_go_string(0., &mut buf).unwrap();
        assert_eq!(buf, "0");
    }

    #[test]
    fn inf_f32_to_go_string() {
        let mut buf = String::new();
        let float = f32::from_bits(0x7f_80_00_00);
        assert!(float.is_infinite() && float.is_sign_positive());
        f32_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "+Inf");

        buf.clear();
        let float = f32::from_bits(0xff_80_00_00);
        assert!(float.is_infinite() && float.is_sign_negative());
        f32_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "-Inf");
    }

    #[test]
    fn inf_f64_to_go_string() {
        let mut buf = String::new();
        let float = f64::from_bits(0x7f_f0_00_00_00_00_00_00);
        assert!(float.is_infinite() && float.is_sign_positive());
        f64_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "+Inf");

        buf.clear();
        let float = f64::from_bits(0xff_f0_00_00_00_00_00_00);
        assert!(float.is_infinite() && float.is_sign_negative());
        f64_to_go_string(float, &mut buf).unwrap();
        assert_eq!(buf, "-Inf");
    }
}
