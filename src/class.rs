use std::fmt::Debug;
use std::sync::{Arc, Mutex, Once, OnceLock};

use jvmrs_lib::{
    access, AccessFlags, ClassVersion, Constant, FieldType, MethodDescriptor, MethodHandle,
};

use crate::class_loader::MethodName;

pub use self::code::Code;
use self::code::NativeTodo;

pub mod code;

#[derive(Debug)]
pub struct VTableEntry {
    pub name: MethodName,
    pub value: OnceLock<(Arc<Class>, Arc<Method>)>,
}

pub struct Class {
    /// tracks if the <clinit> function has been run
    pub initialized: Once,
    /// unused
    pub version: ClassVersion,
    /// run-time constant pool
    pub constants: Vec<Constant>,
    pub access: AccessFlags,
    /// current class name
    pub this: Arc<str>,
    /// super class name
    pub super_class: Arc<str>,
    /// interface names
    pub interfaces: Vec<Arc<str>>,
    /// number of u32 in fields
    pub field_size: usize,
    /// type and index of fields
    pub fields: Vec<(Field, usize)>,
    /// fields at object initialization
    pub initial_fields: Vec<u32>,
    /// static fields
    pub static_data: Mutex<Vec<u32>>,
    /// static field descriptors
    pub statics: Vec<(Field, usize)>,
    /// content of all methods in the class
    pub vtable: Vec<VTableEntry>,
    pub bootstrap_methods: Vec<BootstrapMethod>,
    pub source_file: Option<Arc<str>>,
    /// signature including generics
    pub signature: Option<Arc<str>>,
    pub inner_classes: Vec<InnerClass>,
    pub attributes: Vec<Attribute>,
}

impl Default for Class {
    fn default() -> Self {
        Self {
            initialized: Once::new(),
            version: ClassVersion::default(),
            constants: Vec::new(),
            access: access!(public),
            this: "".into(),
            super_class: "".into(),
            interfaces: Vec::new(),
            field_size: 0,
            fields: Vec::new(),
            initial_fields: Vec::new(),
            static_data: Mutex::new(Vec::new()),
            statics: Vec::new(),
            vtable: Vec::new(),
            bootstrap_methods: Vec::new(),
            source_file: None,
            signature: None,
            inner_classes: Vec::new(),
            attributes: Vec::new(),
        }
    }
}

impl Debug for Class {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.access, self.this)?;
        if &*self.super_class != "java/lang/Object" {
            write!(f, " extends {}", self.super_class)?;
        }
        if !self.interfaces.is_empty() {
            write!(f, " implements {}", self.interfaces.join(", "))?;
        }
        let mut s = f.debug_struct("");
        s.field("initialized", &self.initialized.is_completed());
        if let Some(signature) = &self.signature {
            s.field("signature", signature);
        }
        s.field("version", &self.version)
            .field("constants", &self.constants)
            .field("field_size", &self.field_size)
            .field("fields", &self.fields)
            .field("static_data", &self.static_data.lock().unwrap())
            .field("statics", &self.statics)
            .field("vtable", &self.vtable)
            .field("bootstrap_methods", &self.bootstrap_methods);
        if !self.inner_classes.is_empty() {
            s.field("inner_classes", &self.inner_classes);
        }
        if let Some(source_file) = &self.source_file {
            s.field("source_file", source_file);
        }
        for Attribute { name, data } in &self.attributes {
            s.field(name, &data);
        }
        if false {
            s.field("initial fields", &self.initial_fields);
        }
        s.finish()
    }
}

#[derive(Clone, PartialEq)]
pub struct Field {
    pub access_flags: AccessFlags,
    pub name: Arc<str>,
    pub descriptor: FieldType,
    pub constant_value: Option<Constant>,
    pub signature: Option<Arc<str>>,
    pub attributes: Vec<Attribute>,
}

impl Default for Field {
    fn default() -> Self {
        Self {
            access_flags: AccessFlags(0),
            name: "<>".into(),
            descriptor: FieldType::Object("java/lang/Object".into()),
            constant_value: None,
            signature: None,
            attributes: Vec::new(),
        }
    }
}

impl Debug for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{} {}", self.access_flags, self.descriptor, self.name)?;
        let mut s = f.debug_struct("");
        if let Some(signature) = &self.signature {
            s.field("signature", signature);
        }
        if let Some(constant_value) = &self.constant_value {
            s.field("costant_value", constant_value);
        }
        for Attribute { name, data } in &self.attributes {
            s.field(name, data);
        }
        s.finish()
    }
}

pub struct Method {
    pub max_locals: u16,
    pub access_flags: AccessFlags,
    pub name: Arc<str>,
    /// exceptions thrown by the method
    pub exceptions: Vec<Arc<str>>,
    pub descriptor: MethodDescriptor,
    pub code: Code,
    /// method signature including generics
    pub signature: Option<Arc<str>>,
    pub attributes: Vec<Attribute>,
}

impl Default for Method {
    fn default() -> Self {
        Self {
            max_locals: 0,
            access_flags: AccessFlags::ACC_PUBLIC,
            name: "<>".into(),
            exceptions: Vec::new(),
            descriptor: MethodDescriptor {
                parameter_size: 0,
                parameters: Vec::new(),
                return_type: None,
            },
            code: Code::native(NativeTodo),
            signature: None,
            attributes: Vec::new(),
        }
    }
}

impl Debug for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}{:?} {}",
            self.access_flags, self.descriptor, self.name
        )?;
        if !self.exceptions.is_empty() {
            write!(f, " throws {}", self.exceptions.join(", "))?;
        }
        let mut s = f.debug_struct("");
        s.field("max_locals", &self.max_locals);
        if let Some(signature) = &self.signature {
            s.field("signature", signature);
        }
        if !self.code.is_abstract() {
            s.field("code", &self.code);
        }
        for Attribute { name, data } in &self.attributes {
            s.field(name, data);
        }
        s.finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: Arc<str>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct BootstrapMethod {
    pub method: MethodHandle,
    pub args: Vec<Constant>,
}

#[derive(Debug, Clone)]
pub struct InnerClass {
    pub this: Arc<str>,
    pub outer: Option<Arc<str>>,
    pub name: Option<Arc<str>>,
    pub flags: AccessFlags,
}
