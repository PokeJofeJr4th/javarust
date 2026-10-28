use std::sync::Arc;

use jvmrs_lib::{access, method};

use crate::{
    class::code::{native_property, NativeSingleMethod, NativeStringMethod},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    virtual_machine::object::ClassObj,
};

pub fn add_native_methods(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    java_lang_object: &Arc<str>,
    java_lang_string: &Arc<str>,
) {
    let mut class_class = RawClass::new(
        access!(public native),
        "java/lang/Class".into(),
        java_lang_object.clone(),
    );
    let class_name = RawMethod {
        name: "getName".into(),
        access_flags: access!(public native),
        descriptor: method!(() -> Object(java_lang_string.clone())),
        code: RawCode::native(NativeStringMethod(native_property(ClassObj::SELF, |cls| {
            cls.this.clone()
        }))),
        ..Default::default()
    };
    let class_assertion_status = RawMethod {
        name: "desiredAssertionStatus".into(),
        access_flags: access!(public native),
        descriptor: method!(() -> boolean),
        code: RawCode::native(NativeSingleMethod(native_property(
            ClassObj::SELF,
            |_cls| 1,
        ))),
        ..Default::default()
    };
    class_class.register_methods([class_name, class_assertion_status], method_area);

    class_area.extend([class_class]);
}
