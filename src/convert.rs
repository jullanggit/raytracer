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

macro_rules! FloatAsConvert {
    ($head:ident, $($tail:ident),+) => {
        $(
            FloatAsConvertInner!($head, $tail);
            FloatAsConvertInner!($tail, $head);
        )+
    };
}
macro_rules! FloatAsConvertInner {
    ($a:ident, $b:ident) => {
        impl TryConvertSpec<$b> for $a {
            fn try_convert(self) -> Option<$b> {
                Some(self as $b)
            }
        }
    };
}
FloatAsConvert!(f16, f32, f64, f128);

impl<T, O> TryConvertSpec<O> for T {
    // failing impl, if neither are implemented. Tries to link to a (hopefully) nonexistent symbol.
    default fn try_convert(self) -> Option<O> {
        unsafe extern "C" {
            fn __convert_not_implemented() -> !;
        }
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
