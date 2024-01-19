use std::{cell::RefCell, rc::Rc};

use crate::class::{AccessFlags, Class, Field, FieldType, Method, MethodDescriptor};

use super::{thread::heap_allocate, HeapElement, Object};

#[allow(clippy::too_many_lines)]
pub(super) fn add_native_methods(
    method_area: &mut Vec<(Rc<Class>, Rc<Method>)>,
    class_area: &mut Vec<Rc<Class>>,
    heap: &mut Vec<Rc<RefCell<HeapElement>>>,
) {
    let object_name: Rc<str> = Rc::from("java/lang/Object");
    let init = Rc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });

    let mut object = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        object_name.clone(),
        object_name.clone(),
    );
    object.methods.push(init.clone());
    let object = Rc::new(object);

    let string_length = Rc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "length".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Int),
        },
        attributes: Vec::new(),
        code: None,
    });
    let char_at = Rc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "charAt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: Some(FieldType::Char),
        },
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
    let string = Rc::new(string);

    let builder_init = Rc::new(Method {
        max_locals: 0,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/String".into())],
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });
    let set_char_at = Rc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "setCharAt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Int, FieldType::Char],
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });
    let to_string = Rc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "toString".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: Some(FieldType::Object("java/lang/String".into())),
        },
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
    let string_builder = Rc::new(string_builder);

    let random_init = Rc::new(Method {
        max_locals: 0,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });
    let next_int = Rc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "nextInt".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: Some(FieldType::Int),
        },
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
    let random = Rc::new(random);

    let println = Rc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Object("java/lang/String".into())],
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });
    let println_int = Rc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Int],
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });
    let println_float = Rc::new(Method {
        max_locals: 2,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 1,
            parameters: vec![FieldType::Float],
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });
    let println_empty = Rc::new(Method {
        max_locals: 1,
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
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

    let printstream = Rc::new(printstream);

    let mut system_out = Object::new();
    system_out.class_mut_or_insert(&printstream);
    let system_out_idx = heap_allocate(heap, HeapElement::Object(system_out));

    let mut system = Class::new(
        AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC,
        "java/lang/System".into(),
        object_name.clone(),
    );
    system.static_data.borrow_mut().push(system_out_idx);
    system.statics.push((
        Field {
            access_flags: AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
            name: "out".into(),
            descriptor: FieldType::Object("java/io/PrintStream".into()),
            attributes: Vec::new(),
            constant_value: None,
        },
        0,
    ));
    let system = Rc::new(system);

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
            constant_value: None,
        },
        0,
    ));
    let method_handle = Rc::new(method_handle);

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
            constant_value: None,
        },
        0,
    ));
    let method_type = Rc::new(method_type);

    let call_site = Class::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/invoke/CallSite".into(),
        object_name.clone(),
    );
    let call_site = Rc::new(call_site);

    let make_concat_with_constants = Rc::new(Method {
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
    let string_concat_factory = Rc::new(string_concat_factory);

    let sqrt_double = Rc::new(Method {
        access_flags: AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC,
        name: "sqrt".into(),
        max_locals: 2,
        descriptor: MethodDescriptor {
            parameter_size: 2,
            parameters: vec![FieldType::Double],
            return_type: Some(FieldType::Double),
        },
        attributes: Vec::new(),
        code: None,
    });
    let mut math = Class::new(
        AccessFlags::ACC_PUBLIC | AccessFlags::ACC_NATIVE,
        "java/lang/Math".into(),
        object_name,
    );
    math.methods.push(sqrt_double.clone());
    let math = Rc::new(math);

    method_area.extend([
        (object.clone(), init),
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
