pub mod error;
pub mod instruction;
mod native;
pub mod object;
pub mod thread;

use std::sync::Arc;

use jvmrs_lib::{FieldType, MethodDescriptor};

use crate::class::{Class, Method};
use crate::data::{SharedClassArea, SharedHeap, SharedMethodArea};

pub use self::native::add_native_methods;

use self::object::Array1;
pub use self::thread::Thread;

pub use self::instruction::{hydrate_code, Cmp, Instruction, Op};

#[derive(Debug)]
pub struct StackFrame {
    /// local variables, aka registers
    pub locals: Vec<u32>,
    /// stack used for ephemeral operations
    pub operand_stack: Vec<u32>,
    /// list of pointers that should be collected at the end of execution
    pub garbage: Vec<u32>,
    /// current method
    pub method: Arc<Method>,
    /// current class
    pub class: Arc<Class>,
}

impl StackFrame {
    pub fn from_method(method: Arc<Method>, class: Arc<Class>) -> Self {
        Self {
            locals: vec![0; method.max_locals as usize],
            operand_stack: Vec::with_capacity(
                method
                    .code
                    .as_bytecode()
                    .map(|bc| bc.max_stack as usize)
                    .unwrap_or_default(),
            ),
            garbage: Vec::new(),
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
    argv: Vec<String>,
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

    let mut heap_borrow = heap.lock().unwrap();
    let arg_ptrs: Vec<u32> = argv
        .into_iter()
        .map(|arg| heap_borrow.allocate_str(arg.into()))
        .collect();
    let argv_ptr = heap_borrow.allocate(Array1::from_vec(
        arg_ptrs,
        FieldType::Object("java/lang/String".into()),
    ));
    drop(heap_borrow);

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
        stackframe: StackFrame::from_method(method, class),
        method_area,
        class_area,
        heap,
    };
    primary_thread.stackframe.locals[0] = argv_ptr;
    loop {
        // println!(
        //     "{:?}",
        //     primary_thread.stack.last().unwrap().lock().unwrap().operand_stack
        // );
        // println!("{}", primary_thread.pc_register);
        // if primary_thread.stack.is_empty() {
        //     return;
        // }
        match primary_thread.tick(verbose) {
            Ok(()) => {}
            Err(error::Error::ThreadKill) => break,
            Err(other) => {
                println!("Error in main thread: {other:?}");
                break;
            }
        }
    }
}
