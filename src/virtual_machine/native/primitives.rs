use std::sync::Arc;

use crate::{
    class::{AccessFlags, Class, Code, Field, FieldType, Method, MethodDescriptor, NativeTodo},
    data::{WorkingClassArea, MethodArea},
};

pub(super) fn make_primitives(
    method_area: &mut MethodArea,
    class_area: &mut WorkingClassArea,
    object_class: Arc<str>,
) -> Vec<Arc<Method>> {
    vec![make_primitive_class(
        method_area,
        class_area,
        object_class,
        FieldType::Int,
        "Integer".into(),
    )]
}

fn make_primitive_class(
    method_area: &mut MethodArea,
    class_area: &mut WorkingClassArea,
    object_class: Arc<str>,
    primitive: FieldType,
    primitive_class: Arc<str>,
) -> Arc<Method> {
    let mut class = Class::new(
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

    let init = Arc::new(Method {
        max_locals: 1 + primitive.get_size() as u16,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1 + primitive.get_size(),
            parameters: vec![primitive.clone()],
            return_type: Some(FieldType::Object(primitive_class.clone())),
        },
        code: Code::native(NativeTodo),
        signature: None,
        attributes: Vec::new(),
    });

    let value_of = Arc::new(Method {
        max_locals: 1 + primitive.get_size() as u16,
        access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC | AccessFlags::ACC_NATIVE,
        name: "valueOf".into(),
        descriptor: MethodDescriptor {
            parameter_size: primitive.get_size(),
            parameters: vec![primitive],
            return_type: Some(FieldType::Object(primitive_class)),
        },
        code: Code::native(NativeTodo),
        signature: None,
        attributes: Vec::new(),
    });

    class.methods.extend([value_of.clone(), init.clone()]);
    let class = Arc::new(class);

    method_area.extend([(class.clone(), value_of), (class.clone(), init)]);
    class_area.push(class);
    Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: vec![],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        code: Code::native(NativeTodo),
        signature: None,
        attributes: Vec::new(),
    })
}
