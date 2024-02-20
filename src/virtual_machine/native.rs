use std::sync::{Arc, Mutex};

use crate::class::{AccessFlags, Class, Field, FieldType, Method, MethodDescriptor};

use super::{object::Object, thread::heap_allocate};

pub mod arrays;
pub mod string_builder;

pub static mut OBJECT_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_CLASS: Option<Arc<Class>> = None;
pub static mut STRING_BUILDER_CLASS: Option<Arc<Class>> = None;
pub static mut ARRAY_CLASS: Option<Arc<Class>> = None;

#[allow(clippy::too_many_lines)]
pub(super) fn add_native_methods(
    method_area: &mut Vec<(Arc<Class>, Arc<Method>)>,
    class_area: &mut Vec<Arc<Class>>,
    heap: &mut Vec<Arc<Mutex<Object>>>,
) {
    let object_name: Arc<str> = Arc::from("java/lang/Object");
    let init = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });

    let mut object = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        object_name.clone(),
        object_name.clone(),
    );
    object.methods.push(init.clone());
    let object = Arc::new(object);

    let array = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/Array".into(),
        object_name.clone(),
    );
    let array = Arc::new(array);

    let arrays_to_string = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Array(Box::new(FieldType::Int))],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        signature: None,
        code: None,
        attributes: Vec::new(),
    });
    let deep_to_string = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "deepToString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Array(Box::new(FieldType::Object(
                object_name.clone(),
            )))],
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let mut arrays = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/util/Arrays".into(),
        object_name.clone(),
    );
    arrays
        .methods
        .extend([arrays_to_string.clone(), deep_to_string.clone()]);
    let arrays = Arc::new(arrays);

    let string_length = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "length".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Int),
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let char_at = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "charAt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: Some(FieldType::Char),
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let mut string = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/String".into(),
        object_name.clone(),
    );
    string
        .methods
        .extend([string_length.clone(), char_at.clone()]);
    let string = Arc::new(string);

    let builder_init = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/String".into())],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let set_char_at = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "setCharAt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Int, FieldType::Char],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let to_string = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let mut string_builder = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/StringBuilder".into(),
        object_name.clone(),
    );
    string_builder
        .methods
        .extend([builder_init.clone(), set_char_at.clone(), to_string.clone()]);
    let string_builder = Arc::new(string_builder);

    let random_init = Arc::new(Method {
        max_locals: 0,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let next_int = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "nextInt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: Some(FieldType::Int),
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let mut random = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/util/Random".into(),
        object_name.clone(),
    );
    random
        .methods
        .extend([random_init.clone(), next_int.clone()]);
    let random = Arc::new(random);

    let println = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/String".into())],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let println_int = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let println_float = Arc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Float],
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let println_empty = Arc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let mut printstream = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/io/PrintStream".into(),
        object_name.clone(),
    );
    printstream.methods.extend([
        println.clone(),
        println_empty.clone(),
        println_float.clone(),
        println_int.clone(),
    ]);

    let printstream = Arc::new(printstream);

    let mut system_out = Object::new();
    system_out.class_mut_or_insert(&printstream);
    let system_out_idx = heap_allocate(heap, system_out);

    let mut system = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/System".into(),
        object_name.clone(),
    );
    system.static_data.lock().unwrap().push(system_out_idx);
    system.statics.push((
        Field {
            access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
            name: "out".into(),
            descriptor: FieldType::Object("java/io/PrintStream".into()),
            attributes: Vec::new(),
            signature: None,
            constant_value: None,
        },
        0,
    ));
    let system = Arc::new(system);

    let mut method_handle = Class::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/invoke/MethodHandle".into(),
        object_name.clone(),
    );
    method_handle.field_size = 1;
    method_handle.fields.push((
        Field {
            access_flags: AccessFlags::ACC_PUBLIC,
            name: "location".into(),
            descriptor: FieldType::Int,
            attributes: Vec::new(),
            signature: None,
            constant_value: None,
        },
        0,
    ));
    let method_handle = Arc::new(method_handle);

    let mut method_type = Class::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/invoke/MethodType".into(),
        object_name.clone(),
    );
    method_type.field_size = 1;
    method_type.fields.push((
        Field {
            access_flags: AccessFlags::ACC_PUBLIC,
            name: "location".into(),
            descriptor: FieldType::Int,
            attributes: Vec::new(),
            signature: None,
            constant_value: None,
        },
        0,
    ));
    let method_type = Arc::new(method_type);

    let call_site = Class::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/invoke/CallSite".into(),
        object_name.clone(),
    );
    let call_site = Arc::new(call_site);

    let make_concat_with_constants = Arc::new(Method {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        max_locals: 5,
        name: "makeConcatWithConstants".into(),
        descriptor: MethodDescriptor {
            parameter_size: 5,
            parameters: vec![
                FieldType::Object("java/lang/invoke/MethodHandles$Lookup".into()),
                FieldType::Object("java/lang/String".into()),
                FieldType::Object("java/lang/invoke/MethodType".into()),
                FieldType::Object("java/lang/String".into()),
                FieldType::Array(Box::new(FieldType::Object("java/lang/Object".into()))),
            ],
            return_type: Some(FieldType::Object("java/lang/invoke/CallSite".into())),
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });

    let mut string_concat_factory = Class::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/invoke/StringConcatFactory".into(),
        object_name.clone(),
    );
    string_concat_factory
        .methods
        .push(make_concat_with_constants.clone());
    let string_concat_factory = Arc::new(string_concat_factory);

    let sqrt_double = Arc::new(Method {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "sqrt".into(),
        max_locals: 2,
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Double],
            return_type: Some(FieldType::Double),
        },
        signature: None,
        attributes: Vec::new(),
        code: None,
    });
    let mut math = Class::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/Math".into(),
        object_name,
    );
    math.methods.push(sqrt_double.clone());
    let math = Arc::new(math);

    unsafe {
        OBJECT_CLASS = Some(object.clone());
        STRING_CLASS = Some(string.clone());
        STRING_BUILDER_CLASS = Some(string_builder.clone());
        ARRAY_CLASS = Some(array.clone());
    }
    method_area.extend([
        (object.clone(), init),
        (arrays.clone(), arrays_to_string),
        (arrays.clone(), deep_to_string),
        (string.clone(), string_length),
        (string.clone(), char_at),
        (string_builder.clone(), builder_init),
        (string_builder.clone(), to_string),
        (string_builder.clone(), set_char_at),
        (random.clone(), random_init),
        (random.clone(), next_int),
        (printstream.clone(), println),
        (printstream.clone(), println_float),
        (printstream.clone(), println_int),
        (printstream.clone(), println_empty),
        (string_concat_factory.clone(), make_concat_with_constants),
        (math.clone(), sqrt_double),
    ]);
    class_area.extend([
        object,
        array,
        arrays,
        string,
        string_builder,
        random,
        system,
        printstream,
        method_handle,
        method_type,
        call_site,
        string_concat_factory,
        math,
    ]);
}
