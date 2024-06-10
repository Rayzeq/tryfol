pub trait Utils {
    #[must_use]
    fn digit_count(self) -> u32;
    #[must_use]
    fn first_digit(self) -> Self;
}

pub trait FormatFixed {
    fn format_fixed(self, min_width: usize) -> String;
}

pub trait Japanese {
    fn as_japanese_direct(&self) -> Option<&'static str>;
    fn to_japanese(&self) -> String;
}

macro_rules! impl_utils_for_signed {
    ($type:ty) => {
        impl Utils for $type {
            fn digit_count(mut self) -> u32 {
                let mut count = 1;

                while self.unsigned_abs() >= 10 {
                    self /= 10;
                    count += 1;
                }

                count
            }

            fn first_digit(mut self) -> Self {
                while self.unsigned_abs() >= 10 {
                    self /= 10;
                }
                self
            }
        }
    };
}

macro_rules! impl_utils_for_unsigned {
    ($type:ty) => {
        impl Utils for $type {
            fn digit_count(mut self) -> u32 {
                let mut count = 1;

                while self >= 10 {
                    self /= 10;
                    count += 1;
                }

                count
            }

            fn first_digit(mut self) -> Self {
                while self >= 10 {
                    self /= 10;
                }
                self
            }
        }
    };
}

macro_rules! impl_utils_for_float {
    ($type:ty) => {
        impl Utils for $type {
            /// The digit count of the integral part, as the decimal part can have infintely many digits.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            fn digit_count(mut self) -> u32 {
                self = self.abs();
                if self < 1.0 {
                    return 1;
                }
                (self.log10()) as u32 + 1
            }

            fn first_digit(self) -> Self {
                self / (10.0 as Self).powf(self.digit_count() as Self)
            }
        }
    };
}

impl_utils_for_signed!(i8);
impl_utils_for_signed!(i16);
impl_utils_for_signed!(i32);
impl_utils_for_signed!(i64);
impl_utils_for_signed!(isize);

impl_utils_for_unsigned!(u8);
impl_utils_for_unsigned!(u16);
impl_utils_for_unsigned!(u32);
impl_utils_for_unsigned!(u64);
impl_utils_for_unsigned!(usize);

impl_utils_for_float!(f32);
impl_utils_for_float!(f64);

impl FormatFixed for f64 {
    fn format_fixed(self, min_width: usize) -> String {
        let int_len = self.digit_count() as usize;
        if int_len >= min_width {
            format!("{self:.0}")
        } else if int_len + 1 == min_width {
            format!(" {self:.0}")
        } else {
            format!("{self:.0$}", min_width - (int_len + 1))
        }
    }
}

impl Japanese for i32 {
    fn as_japanese_direct(&self) -> Option<&'static str> {
        Some(match self {
            0 => "〇",
            1 => "一",
            2 => "二",
            3 => "三",
            4 => "四",
            5 => "五",
            6 => "六",
            7 => "七",
            8 => "八",
            9 => "九",
            10 => "十",
            100 => "百",
            1_000 => "千",
            10_000 => "万",
            100_000_000 => "億",
            _ => return None,
        })
    }

    fn to_japanese(&self) -> String {
        if let Some(x) = self.as_japanese_direct() {
            return x.to_owned();
        }

        let base = self.first_digit();
        let digit_count = self.digit_count();
        let multiplier = 10i32.pow(digit_count);
        let remaining = self - base * multiplier;

        format!(
            "{}{}{}",
            if base == 1 {
                ""
            } else {
                // base is in 0..=9, it must have a direct representation
                base.as_japanese_direct().unwrap()
            },
            // TODO: this will panic for some numbers (e.g 100_000)
            multiplier.as_japanese_direct().unwrap(),
            if remaining == 0 {
                String::new()
            } else {
                remaining.to_japanese()
            }
        )
    }
}
