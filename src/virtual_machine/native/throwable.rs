use std::sync::Arc;

use crate::{
    access,
    class::{
        code::{NativeNoop, NativeStringMethod},
        Field, FieldType,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    method,
};

#[allow(clippy::too_many_lines)]
pub fn add_native_methods(
    java_lang_object: &Arc<str>,
    java_lang_string: &Arc<str>,
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
) {
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

    let throwable_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeNoop),
        ..Default::default()
    };

    throwable.register_method(throwable_init, method_area);

    let mut exception = RawClass::new(
        access!(public native),
        "java/lang/Exception".into(),
        throwable.this.clone(),
    );

    let exception_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeNoop),
        ..Default::default()
    };

    exception.register_method(exception_init, method_area);

    let mut runtime_exception = RawClass::new(
        access!(public native),
        "java/lang/RuntimeException".into(),
        exception.this.clone(),
    );

    let runtime_exception_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeNoop),
        ..Default::default()
    };

    runtime_exception.register_method(runtime_exception_init, method_area);

    let mut illegal_argument_exception = RawClass::new(
        access!(public native),
        "java/lang/IllegalArgumentException".into(),
        runtime_exception.this.clone(),
    );

    let illegal_argument_exception_init = RawMethod {
        access_flags: access!(public native),
        name: "<init>".into(),
        descriptor: method!(() -> void),
        code: RawCode::native(NativeNoop),
        ..Default::default()
    };
    let illegal_argument_exception_to_string = RawMethod {
        access_flags: access!(public native),
        name: "toString".into(),
        descriptor: method!(() -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(|_: &mut _, _: [_; 0], _| {
            Ok(Some("java.lang.IllegalArgumentException".into()))
        })),
        ..Default::default()
    };

    illegal_argument_exception.register_methods(
        [
            illegal_argument_exception_init,
            illegal_argument_exception_to_string,
        ],
        method_area,
    );

    class_area.extend([
        throwable,
        exception,
        runtime_exception,
        illegal_argument_exception,
    ]);
}
