use crate::{
    data::{WorkingClassArea, WorkingMethodArea},
    native_class,
    virtual_machine::native::field::StringField,
};

#[allow(clippy::too_many_lines)]
pub fn add_native_methods(method_area: &mut WorkingMethodArea, class_area: &mut WorkingClassArea) {
    class_area.extend(native_class!(
        use method_area;

        impl java::lang::Throwable {
            let message: java::lang::String;
            let cause: java::lang::Throwable;

            fn "<init>"(&_this) {
                use _thread, _verbose;
                Ok(())
            }
        }

        impl java::lang::Exception(java::lang::Throwable) {
            fn "<init>"(&_this) {
                use _thread, _verbose;
                Ok(())
            }
        }

        impl java::lang::RuntimeException(java::lang::Exception) {
            fn "<init>"(&_this) {
                use _thread, _verbose;
                Ok(())
            }
        }

        impl java::lang::IllegalArgumentException(java::lang::RuntimeException) {
            fn "<init>"(&_this) {
                use _thread, _verbose;
                Ok(())
            }

            fn toString(&_this) -> StringField {
                use _thread, _verbose;
                Ok("java.lang.IllegalArgumentException".into())
            }
        }

        impl java::lang::ArithmeticException(java::lang::RuntimeException) {
            fn "<init>"(&_this) {
                use _thread, _verbose;
                Ok(())
            }

            fn toString(&_this) -> StringField {
                use _thread, _verbose;
                Ok("java.lang.ArithmeticException".into())
            }
        }

        impl java::lang::AssertionError(java::lang::RuntimeException) {
            fn "<init>"(&_this) {
                use _thread, _verbose;
                Ok(())
            }
        }
    ));
}
