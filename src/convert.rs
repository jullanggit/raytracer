pub trait TryConvert<Output> {
    fn try_convert(self) -> Option<Output>;
}

// identity
impl<T> TryConvert<T> for T {
    fn try_convert(self) -> Option<T> {
        Some(self)
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

pub trait Convert<Output> {
    fn convert(self) -> Output;
}
impl<T: TryConvert<O>, O> Convert<O> for T {
    fn convert(self) -> O {
        self.try_convert().unwrap()
    }
}
