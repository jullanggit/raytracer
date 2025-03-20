use std::{cell::Cell, f32};

thread_local! {
    pub static RNG: Cell<Rng> = const {Cell::new(Rng(0xef6_f79e_d30b_a75a))};
}

// taken mostly from fastrand (so indirectly from wyrand)
#[repr(transparent)]
pub struct Rng(u64);
impl Rng {
    /// 0..max
    #[expect(clippy::cast_possible_truncation)]
    pub fn u64(&mut self) -> u64 {
        const WY_CONST_0: u64 = 0x2d35_8dcc_aa6c_78a5;
        const WY_CONST_1: u64 = 0x8bb8_4b93_962e_acc9;

        let s = self.0.wrapping_add(WY_CONST_0);
        self.0 = s;
        let t = u128::from(s) * u128::from(s ^ WY_CONST_1);

        (t as u64) ^ (t >> 64) as u64
    }
    /// 0..max
    #[expect(clippy::cast_possible_truncation)]
    pub fn u32(&mut self) -> u32 {
        self.u64() as u32
    }
    /// 0..1
    pub fn f32(&mut self) -> f32 {
        let bits = 32;
        let mantissa = f32::MANTISSA_DIGITS - 1;

        f32::from_bits((1 << (bits - 2)) - (1 << mantissa) + (self.u32() >> (bits - mantissa)))
            - 1.0
    }
}

/// 0..1
pub fn f32() -> f32 {
    RNG.with(|rng| {
        let mut current = rng.replace(Rng(0));

        let f32 = current.f32();

        rng.replace(current);

        f32
    })
}
