use std::{fmt::Display, sync::Arc};

use crate::{
    class::{
        code::{NativeTodo, NativeVoid},
        AccessFlags, Field, FieldType, MethodDescriptor,
    },
    class_loader::{RawClass, RawCode, RawMethod},
    data::{WorkingClassArea, WorkingMethodArea},
    virtual_machine::thread::stacking::Stackable,
};

pub(super) fn make_primitives(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    object_class: Arc<str>,
) -> Vec<RawMethod> {
    vec![make_primitive_class::<i32, 1>(
        method_area,
        class_area,
        object_class,
        FieldType::Int,
        "Integer".into(),
        |i, _| i as i32,
    )]
}

fn make_primitive_class<T: Stackable<u32> + Display, const SIZE: usize>(
    method_area: &mut WorkingMethodArea,
    class_area: &mut WorkingClassArea,
    object_class: Arc<str>,
    primitive: FieldType,
    primitive_class: Arc<str>,
    from_parameter: impl Fn(u32, u32) -> T,
) -> RawMethod {
    let mut class = RawClass::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        primitive_class.clone(),
        object_class,
    );
    let primitive_size = primitive.get_size();
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
    class.field_size += primitive_size;

    let init = RawMethod {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1 + primitive_size,
            parameters: vec![primitive.clone()],
            return_type: Some(FieldType::Object(primitive_class.clone())),
        },
        code: RawCode::native(NativeVoid(
            |thread: &mut _, [this, upper, lower]: [u32; 3], _verbose| Ok(Some(())),
        )),
        signature: None,
        attributes: Vec::new(),
        exceptions: Vec::new(),
    };

    let value_of = RawMethod {
        access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC | AccessFlags::ACC_NATIVE,
        name: "valueOf".into(),
        descriptor: MethodDescriptor {
            parameter_size: primitive_size,
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
