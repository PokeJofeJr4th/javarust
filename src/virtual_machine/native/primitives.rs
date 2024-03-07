use std::sync::Arc;

use crate::{
    class::{code::NativeTodo, AccessFlags, Field, FieldType, MethodDescriptor},
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
};

pub(super) fn make_primitives(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    object_class: Arc<str>,
) -> Vec<RawMethod> {
    vec![make_primitive_class(
        method_area,
        class_area,
        object_class,
        FieldType::Int,
        "Integer".into(),
    )]
}

fn make_primitive_class(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    object_class: Arc<str>,
    primitive: FieldType,
    primitive_class: Arc<str>,
) -> RawMethod {
    let mut class = RawClass::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        primitive_class.clone(),
        object_class,
    );
    class.fields.push((
        Field {
            access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
            name: "value".into(),
            descriptor: primitive.clone(),
            constant_value: None,
            signature: None,
            attributes: Vec::new(),
        },
        class.field_size,
    ));
    class.field_size += primitive.get_size();

    let init = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1 + primitive.get_size(),
            parameters: vec![primitive.clone()],
            return_type: Some(FieldType::Object(primitive_class.clone())),
        },
        code: RawCode::native(NativeTodo),
        signature: None,
        attributes: Vec::new(),
        exceptions: Vec::new(),
    };

    let value_of = RawMethod {
        access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC | AccessFlags::ACC_NATIVE,
        name: "valueOf".into(),
        descriptor: MethodDescriptor {
            parameter_size: primitive.get_size(),
            parameters: vec![primitive],
            return_type: Some(FieldType::Object(primitive_class)),
        },
        code: RawCode::native(NativeTodo),
        signature: None,
        attributes: Vec::new(),
        exceptions: Vec::new(),
    };

    class.methods.extend([]);

    method_area.extend([(class.this.clone(), value_of), (class.this.clone(), init)]);
    class_area.push(class);
    RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: vec![],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: RawCode::native(NativeTodo),
        ..Default::default()
    }
}
