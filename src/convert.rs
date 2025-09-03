pub trait TryConvert<Output> {
    fn try_convert(self) -> Option<Output>;
}

// identity
impl<T> TryConvert<T> for T {
    fn try_convert(self) -> Option<T> {
        Some(self)
    }
}
impl<T, O> TryConvert<O> for T {
    default fn try_convert(self) -> Option<O> {
        <Self as TryConvertSpec<O>>::try_convert(self)
    }
}

impl<T, O: From<T>> TryConvert<O> for T {
    default fn try_convert(self) -> Option<O> {
        Some(self.into())
    }
}

impl<T, O: TryFrom<T>> TryConvert<O> for T {
    default fn try_convert(self) -> Option<O> {
        self.try_into().ok()
    }
}

pub trait TryConvertSpec<Output> {
    fn try_convert(self) -> Option<Output>;
}

// convert primitive number types using 'as' if neither From nor TryFrom are available.
macro_rules! AsConvert {
    ($head:ident $(,)? $($tail:ident),*) => {
        $(
            AsConvertInner!($head, $tail);
            AsConvertInner!($tail, $head);
        )*
        AsConvert!($($tail),*);
    };
    () => {}
}
macro_rules! AsConvertInner {
    ($a:ident, $b:ident) => {
        impl TryConvertSpec<$b> for $a {
            #[allow(clippy::allow_attributes)]
            #[allow(clippy::cast_possible_truncation)]
            #[allow(clippy::cast_sign_loss)]
            #[allow(clippy::cast_lossless)] // we actually do use From if it is available :)
            #[allow(clippy::cast_precision_loss)]
            #[allow(clippy::cast_possible_wrap)]
            fn try_convert(self) -> Option<$b> {
                Some(self as $b)
            }
        }
    };
}
AsConvert!(
    f16, f32, f64, f128, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize
);

impl<T, O> TryConvertSpec<O> for T {
    // failing impl, if neither are implemented. Tries to link to a (hopefully) nonexistent symbol.
    default fn try_convert(self) -> Option<O> {
        unsafe extern "C" {
            fn __convert_not_implemented() -> !;
        }
        // SAFETY:
        // yeah not really safe, fingers crossed this symbol is undefined and raises a link-time-error
        unsafe { __convert_not_implemented() }
    }
}

/// Sugar around `TryConvert` that just unwraps the Option.
/// Converts from `Self` to `Output`
pub trait Convert<Output> {
    fn convert(self) -> Output;
}
impl<T: TryConvert<O>, O> Convert<O> for T {
    fn convert(self) -> O {
        self.try_convert().unwrap()
    }
}
