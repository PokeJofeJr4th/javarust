use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use crate::virtual_machine::{thread::push_long, Instruction, StackFrame, Thread};

use super::{Attribute, FieldType};

#[derive(Clone, Copy)]
pub struct LineTableEntry {
    pub line: u16,
    pub pc: u16,
}

impl Debug for LineTableEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line {} => PC {}", self.line, self.pc)
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct LocalVarTypeEntry {
    pub pc: u16,
    pub length: u16,
    pub name: Arc<str>,
    pub ty: Arc<str>,
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
pub struct LocalVarEntry {
    pub pc: u16,
    pub length: u16,
    pub name: Arc<str>,
    pub ty: FieldType,
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
pub enum Code {
    Code(ByteCode),
    Native(Arc<Box<dyn NativeMethod>>),
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

pub trait NativeMethod: Send + Sync + 'static {
    /// # Errors
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String>;
}

#[derive(Clone, Copy)]
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
}

#[derive(Clone, Copy)]
pub struct NativeSingleMethod<T, const N: usize = 1>(pub T);

impl<
        T: Fn(&mut Thread, &Mutex<StackFrame>, [u32; N], bool) -> Result<u32, String>
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
        let single = self.0(thread, stackframe, values, is_verbose)?;
        stackframe.lock().unwrap().operand_stack.push(single);
        thread.return_one(is_verbose);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct NativeDoubleMethod<T, const N: usize = 1>(pub T);

impl<
        T: Fn(&mut Thread, &Mutex<StackFrame>, [u32; N], bool) -> Result<u64, String>
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
        let double = self.0(thread, stackframe, values, is_verbose)?;
        push_long(&mut stackframe.lock().unwrap().operand_stack, double);
        thread.return_two(is_verbose);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct NativeStringMethod<T, const N: usize = 1>(pub T);

impl<
        T: Fn(&mut Thread, &Mutex<StackFrame>, [u32; N], bool) -> Result<Arc<str>, String>
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
        let str = self.0(thread, stackframe, values, is_verbose)?;
        let heap_allocation = thread.heap.lock().unwrap().allocate_str(str);
        stackframe
            .lock()
            .unwrap()
            .operand_stack
            .push(heap_allocation);
        thread.return_one(is_verbose);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct NativeVoid<T, const N: usize = 1>(pub T);

impl<
        T: Fn(&mut Thread, &Mutex<StackFrame>, [u32; N], bool) -> Result<(), String>
            + Send
            + Sync
            + 'static,
        const N: usize,
    > NativeMethod for NativeVoid<T, N>
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
        self.0(thread, stackframe, values, is_verbose)?;
        thread.return_void();
        Ok(())
    }
}

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
}

#[derive(Clone)]
pub struct ExceptionTableEntry {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
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
pub struct ByteCode {
    pub max_stack: u16,
    pub code: Vec<Instruction>,
    pub exception_table: Vec<ExceptionTableEntry>,
    pub line_number_table: Vec<LineTableEntry>,
    pub local_type_table: Vec<LocalVarTypeEntry>,
    pub local_var_table: Vec<LocalVarEntry>,
    pub stack_map: Vec<StackMapFrame>,
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
