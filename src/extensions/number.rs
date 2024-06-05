pub trait NumberExt {
    fn first_digit(self) -> (Self, u32)
    where
        Self: Sized;
    fn as_japanese_direct(&self) -> Option<&'static str>;
    fn to_japanese(&self) -> String;
}

impl NumberExt for i32 {
    fn first_digit(mut self) -> (Self, u32) {
        let mut i = 0;
        while self >= 10 {
            self /= 10;
            i += 1;
        }
        (self, i)
    }

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

        let (base, digit_count) = self.first_digit();
        let multiplier = 10i32.pow(digit_count);
        let remaining = self - base * multiplier;

        format!(
            "{}{}{}",
            if base == 1 {
                ""
            } else {
                unsafe { base.as_japanese_direct().unwrap_unchecked() }
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
