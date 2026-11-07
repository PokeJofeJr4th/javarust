use std::sync::Arc;

use jvmrs_lib::{access, method, FieldType};

use crate::{
    class::{code::NativeNoop, Field},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    native_class,
    virtual_machine::native::field::StringField,
};

#[allow(clippy::too_many_lines)]
pub fn add_native_methods(
    java_lang_object: &Arc<str>,
    java_lang_string: &Arc<str>,
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
) {
    let noop_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeNoop),
        ..Default::default()
    };

    let mut throwable = RawClass::new(
        access!(public native),
        "java/lang/Throwable".into(),
        java_lang_object.clone(),
    );
    throwable.fields.extend([
        (
            Field {
                access_flags: access!(native),
                name: "message".into(),
                descriptor: FieldType::Object(java_lang_string.clone()),
                ..Default::default()
            },
            0,
        ),
        (
            Field {
                access_flags: access!(native),
                name: "cause".into(),
                descriptor: FieldType::Object(throwable.this.clone()),
                ..Default::default()
            },
            1,
        ),
    ]);

    throwable.register_method(noop_init, method_area);

    class_area.extend(native_class!(
        use method_area;

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

    class_area.extend([throwable]);
}
