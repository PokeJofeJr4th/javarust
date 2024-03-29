use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use crate::virtual_machine::{
    object::ObjectFinder, thread::push_long, Instruction, StackFrame, Thread,
};

use super::{Attribute, FieldType};

#[derive(Clone, Copy)]
pub struct LineTableEntry {
    /// the line of source code
    pub line: u16,
    /// the first instruction index of the line
    pub pc: u16,
}

impl Debug for LineTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {} => PC {}", self.line, self.pc)
    }
}

#[derive(Clone, PartialEq, Eq)]
/// A local variable defined within a range of instruction indices
pub struct LocalVarTypeEntry {
    /// the start of the range
    pub pc: u16,
    /// the end of the range
    pub length: u16,
    /// the name of the variable
    pub name: Arc<str>,
    /// the type of the variable, including generics
    pub ty: Arc<str>,
    /// the index into locals of the variable
    pub index: u16,
}

impl Debug for LocalVarTypeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} ({}..{})@{}",
            self.ty,
            self.name,
            self.pc,
            self.pc + self.length,
            self.index,
        )
    }
}

#[derive(Clone, PartialEq, Eq)]
/// A local variable defined within a range of instruction indices
pub struct LocalVarEntry {
    /// the start of the range
    pub pc: u16,
    /// the end of the range
    pub length: u16,
    /// the name of the variable
    pub name: Arc<str>,
    /// the type of the variable, excluding generics
    pub ty: FieldType,
    /// the index into locals of the variable
    pub index: u16,
}

impl Debug for LocalVarEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} ({}..{})@{}",
            self.ty,
            self.name,
            self.pc,
            self.pc + self.length,
            self.index,
        )
    }
}

#[derive(Clone)]
/// Code contained in a method implementation
pub enum Code {
    /// JVM Bytecode
    Code(ByteCode),
    /// Native (rust) code
    Native(Arc<Box<dyn NativeMethod>>),
    /// No code -- method is defined as abstract
    Abstract,
}

impl Code {
    #[must_use]
    pub fn as_bytecode_mut(&mut self) -> Option<&mut ByteCode> {
        match self {
            Self::Code(bt) => Some(bt),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_bytecode(&self) -> Option<&ByteCode> {
        match self {
            Self::Code(bt) => Some(bt),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_native(&self) -> Option<&dyn NativeMethod> {
        match self {
            Self::Native(nm) => Some(&***nm),
            _ => None,
        }
    }

    #[must_use]
    pub const fn is_abstract(&self) -> bool {
        matches!(self, Self::Abstract)
    }

    pub fn native(func: impl NativeMethod + 'static) -> Self {
        Self::Native(Arc::new(Box::new(func)))
    }
}

impl Debug for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code(byte_code) => byte_code.fmt(f),
            Self::Native(_) => write!(f, "<<native code>>"),
            Self::Abstract => write!(f, "<<abstract method>>"),
        }
    }
}

/// The return type for a native method implementation
/// Cases:
///  - Ok(Some(T))  The method should return
///  - Ok(None)     The method should yield for this tick
///  - Err("...")   The JVM should exit with the given error message
pub type NativeReturn<T> = Result<Option<T>, String>;

/// A rust method that can interface with the JVM
///
/// This interface should mostly be used indirectly through one of the following types:
///  - `NativeTodo`
///  - `NativeSingleMethod`
///  - `NativeDoubleMethod`
///  - `NativeStringMethod`
///  - `NativeVoid`
///  - `NativeNoop`
///  - `native_property`
pub trait NativeMethod: Send + Sync + 'static {
    /// # Native Method
    /// Called each tick while the native method is in the current stackframe
    /// # Errors
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String>;

    fn args(&self) -> u16;
}

#[derive(Clone, Copy)]
/// A native method that panics with a TODO macro
pub struct NativeTodo;

impl NativeMethod for NativeTodo {
    fn run(
        &self,
        _thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        _is_verbose: bool,
    ) -> Result<(), String> {
        let method = stackframe.lock().unwrap().method.clone();
        let class = stackframe.lock().unwrap().class.clone();
        Err(format!(
            "Unimplemented Native Method {:?} {}.{}",
            method.descriptor, class.this, method.name
        ))
    }

    fn args(&self) -> u16 {
        0
    }
}

#[derive(Clone, Copy)]
/// A native method that returns a 32-bit value
pub struct NativeSingleMethod<T, const N: usize = 1>(pub T);

impl<
        T: Fn(&mut Thread, &Mutex<StackFrame>, [u32; N], bool) -> NativeReturn<u32>
            + Send
            + Sync
            + 'static,
        const N: usize,
    > NativeMethod for NativeSingleMethod<T, N>
{
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String> {
        let Ok(values) = <[u32; N]>::try_from(stackframe.lock().unwrap().locals[..N].to_vec())
        else {
            panic!("Function does not have enough local variables for its signature");
        };
        if let Some(single) = self.0(thread, stackframe, values, is_verbose)? {
            stackframe.lock().unwrap().operand_stack.push(single);
            thread.return_one(is_verbose);
        }
        Ok(())
    }

    fn args(&self) -> u16 {
        N as u16
    }
}

#[derive(Clone, Copy)]
/// A native method that returns a 64-bit value
pub struct NativeDoubleMethod<T, const N: usize = 1>(pub T);

impl<
        T: Fn(&mut Thread, &Mutex<StackFrame>, [u32; N], bool) -> NativeReturn<u64>
            + Send
            + Sync
            + 'static,
        const N: usize,
    > NativeMethod for NativeDoubleMethod<T, N>
{
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String> {
        let Ok(values) = <[u32; N]>::try_from(stackframe.lock().unwrap().locals[..N].to_vec())
        else {
            panic!("Function does not have enough local variables for its signature");
        };
        if let Some(double) = self.0(thread, stackframe, values, is_verbose)? {
            push_long(&mut stackframe.lock().unwrap().operand_stack, double);
            thread.return_two(is_verbose);
        }
        Ok(())
    }

    fn args(&self) -> u16 {
        N as u16
    }
}

#[derive(Clone, Copy)]
/// A native method that returns a string
pub struct NativeStringMethod<T, const N: usize = 1>(pub T);

impl<
        T: Fn(&mut Thread, &Mutex<StackFrame>, [u32; N], bool) -> NativeReturn<Arc<str>>
            + Send
            + Sync
            + 'static,
        const N: usize,
    > NativeMethod for NativeStringMethod<T, N>
{
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String> {
        let Ok(values) = <[u32; N]>::try_from(stackframe.lock().unwrap().locals[..N].to_vec())
        else {
            panic!("Function does not have enough local variables for its signature");
        };
        if let Some(str) = self.0(thread, stackframe, values, is_verbose)? {
            let heap_allocation = thread.heap.lock().unwrap().allocate_str(str);
            stackframe
                .lock()
                .unwrap()
                .operand_stack
                .push(heap_allocation);
            thread.return_one(is_verbose);
        }
        Ok(())
    }

    fn args(&self) -> u16 {
        N as u16
    }
}

#[derive(Clone, Copy)]
/// A native method that returns void
pub struct NativeVoid<T, const ARGS: usize = 1>(pub T);

impl<
        T: Fn(&mut Thread, &Mutex<StackFrame>, [u32; ARGS], bool) -> NativeReturn<()>
            + Send
            + Sync
            + 'static,
        const ARGS: usize,
    > NativeMethod for NativeVoid<T, ARGS>
{
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String> {
        let Ok(values) =
            <[u32; ARGS]>::try_from(stackframe.lock().unwrap().locals[..ARGS].to_vec())
        else {
            panic!("Function does not have enough local variables for its signature");
        };
        if self.0(thread, stackframe, values, is_verbose)?.is_some() {
            thread.return_void();
        }
        Ok(())
    }

    fn args(&self) -> u16 {
        ARGS as u16
    }
}

/// A native method that does nothing
pub struct NativeNoop;

impl NativeMethod for NativeNoop {
    fn run(
        &self,
        thread: &mut Thread,
        _stackframe: &Mutex<StackFrame>,
        _is_verbose: bool,
    ) -> Result<(), String> {
        thread.return_void();
        Ok(())
    }
    fn args(&self) -> u16 {
        1
    }
}

/// # Panics
pub fn native_property<F: ObjectFinder, O>(
    finder: F,
    func: impl Fn(F::Target<'_>) -> O,
) -> impl Fn(&mut Thread, &Mutex<StackFrame>, [u32; 1], bool) -> Result<Option<O>, String> {
    move |thread: &mut Thread, _: &_, [ptr]: [u32; 1], _| {
        finder
            .get(
                &thread.heap.lock().unwrap(),
                ptr as usize,
                |obj: F::Target<'_>| func(obj),
            )
            .map(Option::Some)
    }
}

#[derive(Clone)]
/// An entry into a bytecode method's exception-handling table
pub struct ExceptionTableEntry {
    /// start of the "try" block
    pub start_pc: u16,
    /// end of the "try" block
    pub end_pc: u16,
    /// start of the "catch" block
    pub handler_pc: u16,
    /// type of the exception
    pub catch_type: Option<Arc<str>>,
}

impl Debug for ExceptionTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.catch_type {
            Some(catch) => write!(
                f,
                "{{{}..{}: catch {catch} => {}}}",
                self.start_pc, self.end_pc, self.handler_pc
            ),
            None => write!(
                f,
                "{{{}..{}: catch => {}}}",
                self.start_pc, self.end_pc, self.handler_pc
            ),
        }
    }
}

#[derive(Clone, Default)]
/// JVM bytecode
pub struct ByteCode {
    /// highest number of locals on the stack
    pub max_stack: u16,
    /// List of bytecode instructions
    pub code: Vec<Instruction>,
    /// List of exception handlers
    pub exception_table: Vec<ExceptionTableEntry>,
    /// List of line numbers (unused)
    pub line_number_table: Vec<LineTableEntry>,
    /// List of local variables, including generics (unused)
    pub local_type_table: Vec<LocalVarTypeEntry>,
    /// List of local variables (unused)
    pub local_var_table: Vec<LocalVarEntry>,
    /// Stack frame verification information (unused)
    pub stack_map: Vec<StackMapFrame>,
    /// Miscellaneous attributes (unused)
    pub attributes: Vec<Attribute>,
}

impl Debug for ByteCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("ByteCode");
        s.field("max_stack", &self.max_stack)
            .field("code", &self.code)
            .field("stack_map", &self.stack_map);
        if !self.exception_table.is_empty() {
            s.field("exception_table", &self.exception_table);
        }
        if !self.line_number_table.is_empty() {
            s.field("line_number_table", &self.line_number_table);
        }
        if !self.local_type_table.is_empty() {
            s.field("local_variable_type_table", &self.local_type_table);
        }
        if !self.local_var_table.is_empty() {
            s.field("local_variable_table", &self.local_var_table);
        }
        for Attribute { name, data } in &self.attributes {
            s.field(name, data);
        }
        s.finish()
    }
}

#[derive(Debug, Clone)]
pub enum StackMapFrame {
    Same {
        offset_delta: u8,
    },
    SameLocals1Stack {
        offset_delta: u8,
        verification: VerificationTypeInfo,
    },
    SameLocals1StackExtended {
        offset_delta: u16,
        verification: VerificationTypeInfo,
    },
    Chop {
        chop: u8,
        offset_delta: u16,
    },
    SameExtended {
        offset_delta: u16,
    },
    Append {
        offset_delta: u16,
        locals: Vec<VerificationTypeInfo>,
    },
    Full {
        offset_delta: u16,
        locals: Vec<VerificationTypeInfo>,
        stack: Vec<VerificationTypeInfo>,
    },
}

#[derive(Debug, Clone)]
pub enum VerificationTypeInfo {
    Top,
    Integer,
    Float,
    Null,
    UninitializedThis,
    Object { class_name: Arc<str> },
    Uninitialized { offset: u16 },
    Long,
    Double,
}
