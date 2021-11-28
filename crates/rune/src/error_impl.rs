/// this is rune::Any implementation of Error
use super::Error;

impl rune::Any for Error {
    fn type_hash() -> rune::Hash {
        rune::Hash::from_type_id(std::any::TypeId::of::<Self>())
    }
}
impl rune::compile::InstallWith for Error {
    fn install_with(
        _: &mut rune::compile::Module,
    ) -> ::std::result::Result<(), rune::compile::ContextError> {
        Ok(())
    }
}
impl rune::compile::Named for Error {
    const BASE_NAME: rune::runtime::RawStr = rune::runtime::RawStr::from_str("Error");
}
impl rune::runtime::TypeOf for Error {
    fn type_hash() -> rune::Hash {
        <Self as rune::Any>::type_hash()
    }
    fn type_info() -> rune::runtime::TypeInfo {
        rune::runtime::TypeInfo::Any(rune::runtime::RawStr::from_str(
            std::any::type_name::<Self>(),
        ))
    }
}
impl rune::runtime::UnsafeFromValue for &Error {
    type Output = *const Error;
    type Guard = rune::runtime::RawRef;
    fn from_value(
        value: rune::runtime::Value,
    ) -> ::std::result::Result<(Self::Output, Self::Guard), rune::runtime::VmError> {
        value.into_any_ptr()
    }
    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &*output
    }
}
impl rune::runtime::UnsafeFromValue for &mut Error {
    type Output = *mut Error;
    type Guard = rune::runtime::RawMut;
    fn from_value(
        value: rune::runtime::Value,
    ) -> ::std::result::Result<(Self::Output, Self::Guard), rune::runtime::VmError> {
        value.into_any_mut()
    }
    unsafe fn unsafe_coerce(output: Self::Output) -> Self {
        &mut *output
    }
}
impl rune::runtime::UnsafeToValue for &Error {
    type Guard = rune::runtime::SharedPointerGuard;
    unsafe fn unsafe_to_value(
        self,
    ) -> ::std::result::Result<(rune::runtime::Value, Self::Guard), rune::runtime::VmError> {
        let (shared, guard) = rune::runtime::Shared::from_ref(self);
        Ok((rune::runtime::Value::from(shared), guard))
    }
}
impl rune::runtime::UnsafeToValue for &mut Error {
    type Guard = rune::runtime::SharedPointerGuard;
    unsafe fn unsafe_to_value(
        self,
    ) -> ::std::result::Result<(rune::runtime::Value, Self::Guard), rune::runtime::VmError> {
        let (shared, guard) = rune::runtime::Shared::from_mut(self);
        Ok((rune::runtime::Value::from(shared), guard))
    }
}
