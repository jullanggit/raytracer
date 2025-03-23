use std::{
    cell::Cell,
    f32,
    mem::{Assume, TransmuteFrom},
};

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
    /// Efficiently calculates two f32's at once. [0..1; 2]
    // Unlike the rest of this module, this function was actually created by me!
    pub fn f32_by_two(&mut self) -> [f32; 2] {
        let u64 = self.u64();

        let bits = 32;
        let mantissa = f32::MANTISSA_DIGITS - 1;

        let f32_mantissa = (1 << (mantissa)) - 1;
        let mantissa_mask: u64 = f32_mantissa + (f32_mantissa << bits);

        let masked_u64 = u64 & mantissa_mask;

        let one: u64 = 1.0_f32.to_bits().into();
        let one_by_two = (one << bits) + one;

        // SAFETY: As we specify Assume::NOTHING, the compiler guarantees memory safety
        let f32s: [f32; 2] = unsafe {
            TransmuteFrom::<u64, { Assume::NOTHING }>::transmute(one_by_two + masked_u64)
        };

        f32s.map(|f32| f32 - 1.)
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
