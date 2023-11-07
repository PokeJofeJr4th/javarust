mod thread;

use std::{cell::RefCell, rc::Rc};

use crate::class::{AccessFlags, Class, ClassVersion, Field, FieldType, Method, MethodDescriptor};

use self::thread::{heap_allocate, Thread};

fn search_method_area(
    method_area: &[(Rc<Class>, Rc<Method>)],
    class: Rc<str>,
    method: Rc<str>,
    method_type: &MethodDescriptor,
) -> Option<(Rc<Class>, Rc<Method>)> {
    for (possible_class, possible_method) in method_area {
        if possible_class.this == class
            && possible_method.name == method
            && &possible_method.descriptor == method_type
        {
            return Some((possible_class.clone(), possible_method.clone()));
        }
    }
    None
}

#[derive(Debug)]
struct StackFrame {
    locals: Vec<u32>,
    operand_stack: Vec<u32>,
    method: Rc<Method>,
    class: Rc<Class>,
}

impl StackFrame {
    pub fn from_method(method: Rc<Method>, class: Rc<Class>) -> Self {
        let locals = match method.code.as_ref() {
            Some(code) => code.max_locals,
            _ => 0,
        };
        Self {
            locals: (0..=locals).map(|_| 0).collect(),
            operand_stack: Vec::new(),
            class,
            method,
        }
    }
}

enum HeapElement {
    Object(Object),
    String(String),
    Array(Vec<u32>),
    Class(Rc<Class>),
}

struct Object {
    fields: Vec<(Rc<str>, Vec<u32>)>,
}

impl Object {
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    pub fn class_mut_or_insert(&mut self, class: Rc<Class>) -> &mut Vec<u32> {
        let name = class.this.clone();
        &mut if self
            .fields
            .iter_mut()
            .any(|(class_name, _)| class_name == &name)
        {
            self.fields
                .iter_mut()
                .find(|(class_name, _)| class_name == &name)
                .unwrap()
        } else {
            let vec = vec![0; class.field_size];
            self.fields.push((class.this.clone(), vec));
            self.fields.last_mut().unwrap()
        }
        .1
    }
}

pub fn start_vm(src: Class) {
    let class = Rc::new(src);
    let mut method_area = class
        .methods
        .iter()
        .cloned()
        .map(|method| (class.clone(), method))
        .collect::<Vec<_>>();
    let mut class_area = vec![class.clone()];
    let heap = Rc::new(RefCell::new(Vec::new()));
    add_native_methods(&mut method_area, &mut class_area, &mut heap.borrow_mut());
    let mut method = None;
    for methods in &class.methods {
        if &*methods.name == "main" {
            method = Some(methods.clone());
            break;
        }
    }
    let method = method.expect("No `main` function found");
    let mut primary_thread = Thread {
        pc_register: 0,
        stack: Vec::new(),
        method_area: Rc::new(method_area),
        class_area: Rc::new(class_area),
        heap,
    };
    primary_thread.invoke_method(method, class);
    loop {
        // println!(
        //     "{:?}",
        //     primary_thread.stack.last().unwrap().borrow().operand_stack
        // );
        // println!("{}", primary_thread.pc_register);
        primary_thread.tick().unwrap();
    }
}

fn add_native_methods(
    method_area: &mut Vec<(Rc<Class>, Rc<Method>)>,
    class_area: &mut Vec<Rc<Class>>,
    heap: &mut Vec<Rc<RefCell<HeapElement>>>,
) {
    let init = Rc::new(Method {
        access_flags: AccessFlags(AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC),
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });

    let object = Rc::new(Class {
        version: ClassVersion {
            minor_version: 0,
            major_version: 0,
        },
        constants: Vec::new(),
        access: AccessFlags(AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC),
        this: "java/lang/Object".into(),
        super_class: "java/lang/Object".into(),
        interfaces: Vec::new(),
        field_size: 0,
        fields: Vec::new(),
        methods: vec![init.clone()],
        static_data: RefCell::new(Vec::new()),
        statics: Vec::new(),
        attributes: Vec::new(),
    });

    let println = Rc::new(Method {
        access_flags: AccessFlags(AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC),
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
        access_flags: AccessFlags(AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC),
        name: "println".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });

    let printstream = Rc::new(Class {
        version: ClassVersion {
            minor_version: 0,
            major_version: 0,
        },
        constants: Vec::new(),
        access: AccessFlags(AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC),
        this: "java/io/PrintStream".into(),
        super_class: "java/lang/Object".into(),
        interfaces: Vec::new(),
        field_size: 0,
        fields: Vec::new(),
        methods: vec![println.clone()],
        static_data: RefCell::new(Vec::new()),
        statics: Vec::new(),
        attributes: Vec::new(),
    });

    let mut system_out = Object::new();
    system_out.class_mut_or_insert(printstream.clone());
    let system_out_idx = heap_allocate(heap, HeapElement::Object(system_out));

    let system = Rc::new(Class {
        version: ClassVersion {
            minor_version: 0,
            major_version: 0,
        },
        constants: Vec::new(),
        access: AccessFlags(AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC),
        this: "java/lang/System".into(),
        super_class: "java/lang/Object".into(),
        interfaces: Vec::new(),
        field_size: 0,
        fields: Vec::new(),
        methods: Vec::new(),
        static_data: RefCell::new(vec![system_out_idx]),
        statics: vec![(
            Field {
                access_flags: AccessFlags(AccessFlags::ACC_PUBLIC | AccessFlags::ACC_STATIC),
                name: "out".into(),
                descriptor: FieldType::Object("java/io/PrintStream".into()),
                attributes: Vec::new(),
                constant_value: None,
            },
            0,
        )],
        attributes: Vec::new(),
    });

    method_area.extend([
        (object.clone(), init),
        (printstream.clone(), println),
        (printstream.clone(), println_empty),
    ]);
    class_area.extend([object, system, printstream]);
}
