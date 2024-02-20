use std::sync::Arc;

use crate::class::{AccessFlags, Class, FieldType, Method, MethodDescriptor};

fn make_primitive_class(
    method_area: &mut Vec<(Arc<Class>, Arc<Method>)>,
    class_area: &mut Vec<Arc<Class>>,
    object_class: Arc<str>,
    primitive: FieldType,
    primitive_class: Arc<str>,
) {
    let mut class = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        primitive_class.clone(),
        object_class,
    );

    let value_of = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC | AccessFlags::ACC_NATIVE,
        name: "valueOf".into(),
        descriptor: MethodDescriptor {
            parameter_size: primitive.get_size(),
            parameters: vec![primitive],
            return_type: Some(FieldType::Object(primitive_class)),
        },
        code: None,
        signature: None,
        attributes: Vec::new(),
    });

    class.methods.push(value_of.clone());
    let class = Arc::new(class);

    method_area.push((class.clone(), value_of));
    class_area.push(class);
}
