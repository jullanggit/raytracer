use std::{
    array,
    cell::Cell,
    f32,
    mem::{Assume, TransmuteFrom},
    simd::Simd,
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
        let bits = 32;
        let mantissa = f32::MANTISSA_DIGITS - 1;

        let f32_mantissa = (1 << (mantissa)) - 1;
        let mantissa_mask: u64 = f32_mantissa + (f32_mantissa << bits);

        let one: u64 = 1.0_f32.to_bits().into();
        let one_by_two = (one << bits) + one;

        let u64 = self.u64();
        let masked_u64 = u64 & mantissa_mask;

        // SAFETY: As we specify Assume::NOTHING, the compiler guarantees memory safety
        let f32s: [f32; 2] = unsafe {
            TransmuteFrom::<u64, { Assume::NOTHING }>::transmute(one_by_two + masked_u64)
        };

        f32s.map(|f32| f32 - 1.)
    }
    /// Efficiently calculates eight f32's at once. [0..1; 2]
    // Unlike the rest of this module, this function was actually created by me!
    // TODO: Detect simd lanes at compile time
    pub fn simd_f32(&mut self) -> Simd<f32, 8> {
        let bits = 32;
        let mantissa = f32::MANTISSA_DIGITS - 1;

        let exponent = Simd::splat((1 << (bits - 2)) - (1 << mantissa));

        // SAFETY: As we specify Assume::NOTHING, the compiler guarantees memory safety
        let u32s: Simd<u32, 8> = Simd::from_array(unsafe {
            TransmuteFrom::<[u64; 4], { Assume::NOTHING }>::transmute(array::from_fn(|_| {
                self.u64()
            }))
        }) >> (bits - mantissa);

        // SAFETY: As we specify Assume::NOTHING, the compiler guarantees memory safety
        let f32s: Simd<f32, 8> = Simd::from_array(unsafe {
            TransmuteFrom::<_, { Assume::NOTHING }>::transmute((u32s + exponent).to_array())
        });

        f32s - Simd::splat(1.)
    }
}

pub fn with_rng<T>(f: impl Fn(&mut Rng) -> T) -> T {
    RNG.with(|rng| {
        let mut current = rng.replace(Rng(0));

        let res = f(&mut current);

        rng.replace(current);

        res
    })
}

pub trait Random {
    fn random() -> Self;
}
macro_rules! impl_random {
    ($($Type:ident),*) => {
        $(
            impl Random for $Type {
                fn random() -> Self {
                    with_rng(Rng::$Type)
                }
            }
        )*
    };
}
impl_random!(f32, u32, u64);
