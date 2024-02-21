use std::{
    fmt::Debug,
    sync::{Arc, Mutex},
};

use crate::virtual_machine::{
    object::StringObj, thread::heap_allocate, Instruction, StackFrame, Thread,
};

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

pub enum Code {
    Code(ByteCode),
    Native(Box<dyn NativeMethod>),
}

impl Code {
    #[must_use]
    pub const fn as_ref(&self) -> Option<&ByteCode> {
        match self {
            Self::Code(bt) => Some(bt),
            Self::Native(_) => None,
        }
    }

    #[must_use]
    pub const fn as_native(&self) -> Option<&dyn NativeMethod> {
        match self {
            Self::Code(_) => None,
            Self::Native(nm) => Some(&**nm),
        }
    }

    pub fn native(func: impl NativeMethod + 'static) -> Self {
        Self::Native(Box::new(func))
    }
}

impl Debug for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Code(byte_code) => byte_code.fmt(f),
            Self::Native(_) => write!(f, "<<native code>>"),
        }
    }
}

pub trait NativeMethod: Send + Sync {
    /// # Errors
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String>;
}

pub struct NativeTodo;

impl NativeMethod for NativeTodo {
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String> {
        let method = stackframe.lock().unwrap().method.clone();
        let class = stackframe.lock().unwrap().class.clone();
        todo!(
            "Unimplemented Native Method {:?} {}.{}",
            method.descriptor,
            class.this,
            method.name
        )
    }
}

#[derive(Clone, Copy)]
pub struct NativeSingleMethod<
    T: Fn(&mut Thread, &Mutex<StackFrame>, bool) -> Result<u32, String> + Send + Sync,
>(pub T);

impl<T: Fn(&mut Thread, &Mutex<StackFrame>, bool) -> Result<u32, String> + Send + Sync> NativeMethod
    for NativeSingleMethod<T>
{
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String> {
        let single = self.0(thread, stackframe, is_verbose)?;
        stackframe.lock().unwrap().operand_stack.push(single);
        thread.return_one(is_verbose);
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct NativeStringMethod<
    T: Fn(&mut Thread, &Mutex<StackFrame>, bool) -> Result<Arc<str>, String> + Send + Sync,
>(pub T);

impl<T: Fn(&mut Thread, &Mutex<StackFrame>, bool) -> Result<Arc<str>, String> + Send + Sync>
    NativeMethod for NativeStringMethod<T>
{
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String> {
        let str = self.0(thread, stackframe, is_verbose)?;
        let string_object = StringObj::new(&thread.class_area, str);
        let heap_allocation = heap_allocate(&mut thread.heap.lock().unwrap(), string_object);
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
pub struct NativeVoid<
    T: Fn(&mut Thread, &Mutex<StackFrame>, bool) -> Result<(), String> + Send + Sync,
>(pub T);

impl<T: Fn(&mut Thread, &Mutex<StackFrame>, bool) -> Result<(), String> + Send + Sync> NativeMethod
    for NativeVoid<T>
{
    fn run(
        &self,
        thread: &mut Thread,
        stackframe: &Mutex<StackFrame>,
        is_verbose: bool,
    ) -> Result<(), String> {
        self.0(thread, stackframe, is_verbose)?;
        thread.return_void();
        Ok(())
    }
}

pub struct ByteCode {
    pub max_stack: u16,
    pub code: Vec<Instruction>,
    pub exception_table: Vec<(u16, u16, u16, Option<Arc<str>>)>,
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

#[derive(Debug)]
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

#[derive(Debug)]
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
