mod instruction;
mod native;
mod object;
mod thread;

use std::sync::{Arc, Mutex};

use crate::class::{Class, Method, MethodDescriptor};

use self::{native::add_native_methods, thread::Thread};

pub use self::instruction::{hydrate_code, Cmp, Instruction, Op};

fn search_method_area(
    method_area: &[(Arc<Class>, Arc<Method>)],
    class: &str,
    method: &str,
    method_type: &MethodDescriptor,
) -> Option<(Arc<Class>, Arc<Method>)> {
    for (possible_class, possible_method) in method_area {
        if &*possible_class.this == class
            && &*possible_method.name == method
            && &possible_method.descriptor == method_type
        {
            return Some((possible_class.clone(), possible_method.clone()));
        }
    }
    None
}

#[derive(Debug)]
pub struct StackFrame {
    locals: Vec<u32>,
    operand_stack: Vec<u32>,
    method: Arc<Method>,
    class: Arc<Class>,
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

pub fn start_vm(src: Class, verbose: bool) {
    let class = Arc::new(src);
    let mut method_area = class
        .methods
        .iter()
        .cloned()
        .map(|method| (class.clone(), method))
        .collect::<Vec<_>>();
    let mut class_area = vec![class.clone()];
    let heap = Arc::new(Mutex::new(Vec::new()));
    add_native_methods(&mut method_area, &mut class_area, &mut heap.lock().unwrap());
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
        method_area: Arc::from(&*method_area),
        class_area: Arc::from(&*class_area),
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
