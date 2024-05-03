use std::sync::Arc;

use jvmrs_lib::access;

use crate::{
    class::{code::NativeNoop, Field, FieldType},
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

    throwable.register_method(noop_init.clone(), method_area);

    let mut exception = RawClass::new(
        access!(public native),
        "java/lang/Exception".into(),
        throwable.this.clone(),
    );

    exception.register_method(noop_init.clone(), method_area);

    let mut runtime_exception = RawClass::new(
        access!(public native),
        "java/lang/RuntimeException".into(),
        exception.this.clone(),
    );

    runtime_exception.register_method(noop_init.clone(), method_area);

    let mut illegal_argument_exception = RawClass::new(
        access!(public native),
        "java/lang/IllegalArgumentException".into(),
        runtime_exception.this.clone(),
    );
    let illegal_argument_exception_to_string = RawMethod::to_string(|_: &mut _, _: [_; 0], _| {
        Ok(Some("java.lang.IllegalArgumentException".into()))
    });

    illegal_argument_exception.register_methods(
        [noop_init.clone(), illegal_argument_exception_to_string],
        method_area,
    );

    let mut arithmetic_exception = RawClass::new(
        access!(public native),
        "java/lang/ArithmeticException".into(),
        runtime_exception.this.clone(),
    );

    let arith_to_string = RawMethod::to_string(|_: &mut _, _: [_; 0], _| {
        Ok(Some("java.lang.ArithmeticException".into()))
    });
    arithmetic_exception.register_methods([arith_to_string, noop_init], method_area);

    class_area.extend([
        throwable,
        exception,
        runtime_exception,
        illegal_argument_exception,
        arithmetic_exception,
    ]);
}
