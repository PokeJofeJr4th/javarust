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

    let println = Rc::new(Method {
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

    let println_empty = Rc::new(Method {
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
    printstream
        .methods
        .extend([println.clone(), println_empty.clone()]);

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
        object_name,
    );
    string_concat_factory
        .methods
        .push(make_concat_with_constants.clone());
    let string_concat_factory = Rc::new(string_concat_factory);

    method_area.extend([
        (object.clone(), init),
        (printstream.clone(), println),
        (printstream.clone(), println_empty),
        (string_concat_factory.clone(), make_concat_with_constants),
    ]);
    class_area.extend([
        object,
        system,
        printstream,
        method_handle,
        method_type,
        call_site,
        string_concat_factory,
    ]);
}
