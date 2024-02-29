pub mod instruction;
mod native;
pub mod object;
pub mod thread;

use std::sync::Arc;

use crate::class::{Class, FieldType, Method, MethodDescriptor};
use crate::data::{SharedClassArea, SharedHeap, SharedMethodArea};

pub use self::native::add_native_methods;

pub use self::thread::Thread;

pub use self::instruction::{hydrate_code, Cmp, Instruction, Op};

#[derive(Debug)]
pub struct StackFrame {
    pub locals: Vec<u32>,
    pub operand_stack: Vec<u32>,
    pub method: Arc<Method>,
    pub class: Arc<Class>,
}

impl StackFrame {
    pub fn from_method(method: Arc<Method>, class: Arc<Class>) -> Self {
        Self {
            locals: (0..=method.max_locals).map(|_| 0).collect(),
            operand_stack: Vec::new(),
            class,
            method,
        }
    }
}

/// # Panics
pub fn start_vm(
    class: &str,
    method_area: SharedMethodArea,
    class_area: SharedClassArea,
    heap: SharedHeap,
    verbose: bool,
) {
    // set the static classes
    unsafe {
        native::ARRAY_CLASS = class_area.search("java/lang/Array");
        native::OBJECT_CLASS = class_area.search("java/lang/Object");
        native::RANDOM_CLASS = class_area.search("java/util/Random");
        native::STRING_BUILDER_CLASS = class_area.search("java/lang/StringBuilder");
        native::STRING_CLASS = class_area.search("java/lang/String");
    }

    let (class, method) = method_area
        .search(
            class,
            "main",
            &MethodDescriptor {
                parameter_size: 1,
                parameters: vec![FieldType::Array(Box::new(FieldType::Object(
                    "java/lang/String".into(),
                )))],
                return_type: None,
            },
        )
        .expect("No `main` function found");
    let mut primary_thread = Thread {
        pc_register: 0,
        stack: Vec::new(),
        method_area,
        class_area,
        heap,
    };
    primary_thread.invoke_method(method, class);
    loop {
        // println!(
        //     "{:?}",
        //     primary_thread.stack.last().unwrap().lock().unwrap().operand_stack
        // );
        // println!("{}", primary_thread.pc_register);
        if primary_thread.stack.is_empty() {
            return;
        }
        primary_thread.tick(verbose).unwrap();
    }
}
