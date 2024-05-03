use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::sync::{Arc, Mutex, Once, OnceLock};

use jvmrs_lib::{access, AccessFlags};

use crate::class_loader::MethodName;
use crate::data::NULL;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodHandle {
    GetField {
        class: Arc<str>,
        name: Arc<str>,
        field_type: FieldType,
    },
    GetStatic {
        class: Arc<str>,
        name: Arc<str>,
        field_type: FieldType,
    },
    PutField {
        class: Arc<str>,
        name: Arc<str>,
        field_type: FieldType,
    },
    PutStatic {
        class: Arc<str>,
        name: Arc<str>,
        field_type: FieldType,
    },
    InvokeVirtual {
        class: Arc<str>,
        name: Arc<str>,
        method_type: MethodDescriptor,
    },
    InvokeStatic {
        class: Arc<str>,
        name: Arc<str>,
        method_type: MethodDescriptor,
    },
    InvokeSpecial {
        class: Arc<str>,
        name: Arc<str>,
        method_type: MethodDescriptor,
    },
    NewInvokeSpecial {
        class: Arc<str>,
        name: Arc<str>,
        method_type: MethodDescriptor,
    },
    InvokeInterface {
        class: Arc<str>,
        name: Arc<str>,
        method_type: MethodDescriptor,
    },
}

/// A member of the constant pool
#[derive(Clone, PartialEq)]
pub enum Constant {
    String(Arc<str>),
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    ClassRef(Arc<str>),
    StringRef(Arc<str>),
    FieldRef {
        class: Arc<str>,
        name: Arc<str>,
        field_type: FieldType,
    },
    MethodRef {
        class: Arc<str>,
        name: Arc<str>,
        method_type: MethodDescriptor,
    },
    InterfaceRef {
        class: Arc<str>,
        name: Arc<str>,
        interface_type: MethodDescriptor,
    },
    NameTypeDescriptor {
        name: Arc<str>,
        type_descriptor: Arc<str>,
    },
    MethodHandle(MethodHandle),
    MethodType(MethodDescriptor),
    // Dynamic {
    //     constant: u32,
    // },
    InvokeDynamic {
        bootstrap_index: u16,
        method_name: Arc<str>,
        method_type: MethodDescriptor,
    },
    // Module {
    //     identity: u16,
    // },
    // Package {
    //     identity: u16,
    // },
    Placeholder,
}

impl Constant {
    #[must_use]
    pub fn bytes(&self) -> Vec<u32> {
        match self {
            Self::Int(i) => vec![*i as u32],
            Self::Float(f) => vec![f.to_bits()],
            Self::Long(l) => vec![*l as u64 as u32, (*l as u64 >> 32) as u32],
            Self::Double(f) => {
                let bits = f.to_bits();
                vec![bits as u32, (bits >> 32) as u32]
            }
            _ => vec![NULL],
        }
    }
}

impl Debug for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::String(s) => write!(f, "{s:?}"),
            Self::Int(i) => write!(f, "{i}"),
            Self::Float(fl) => write!(f, "{fl}"),
            Self::Long(l) => write!(f, "{l}"),
            Self::Double(d) => write!(f, "{d}"),
            Self::ClassRef(c) => write!(f, "class {c}"),
            Self::StringRef(s) => write!(f, "&{s:?}"),
            Self::FieldRef {
                class,
                name,
                field_type,
            } => write!(f, "Field({field_type} {class}.{name})"),
            Self::MethodRef {
                class,
                name,
                method_type,
            } => write!(f, "Method({method_type:?} {class}.{name})"),
            Self::InterfaceRef {
                class,
                name,
                interface_type,
            } => write!(f, "InterfaceMethod({interface_type:?} {class}.{name})"),
            Self::NameTypeDescriptor {
                name,
                type_descriptor,
            } => write!(f, "NameTypeDescriptor({type_descriptor} {name})"),
            Self::MethodHandle(handle) => write!(f, "MethodHandle({handle:?})"),
            Self::MethodType(method) => write!(f, "MethodType({method:?})"),
            Self::InvokeDynamic {
                bootstrap_index,
                method_name,
                method_type,
            } => write!(
                f,
                "InvokeDynamic(#{bootstrap_index} {method_type:?} {method_name})"
            ),
            Self::Placeholder => write!(f, "Placeholder"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ClassVersion {
    pub minor_version: u16,
    pub major_version: u16,
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

#[derive(Clone, PartialEq, Eq)]
pub struct MethodDescriptor {
    pub parameter_size: usize,
    /// list of parameter types
    pub parameters: Vec<FieldType>,
    /// method return type; None => void
    pub return_type: Option<FieldType>,
}

impl MethodDescriptor {
    pub const EMPTY: Self = Self {
        parameter_size: 0,
        parameters: Vec::new(),
        return_type: None,
    };
}

#[macro_export]
macro_rules! method {
    (($($params:tt),*) -> void) => {{
        let parameters: Vec<$crate::class::FieldType> = vec![$($crate::field!($params)),*];
        $crate::class::MethodDescriptor {
            parameter_size: parameters.iter().map(|param| param.get_size()).sum(),
            parameters,
            return_type: None,
        }
    }};

    (($($params:tt),*) -> $($out:tt)*) => {{
        let parameters: Vec<$crate::class::FieldType> = vec![$($crate::field!($params)),*];
        $crate::class::MethodDescriptor {
            parameter_size: parameters.iter().map(|param| param.get_size()).sum(),
            parameters,
            return_type: Some($crate::field!($($out)*)),
        }
    }};
}

impl Hash for MethodDescriptor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.parameters.hash(state);
        self.return_type.hash(state);
    }
}

impl Debug for MethodDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.return_type {
            Some(t) => {
                write!(f, "{t} ")?;
            }
            None => {
                write!(f, "void ")?;
            }
        }
        write!(
            f,
            "{}({})",
            self.parameter_size,
            self.parameters
                .iter()
                .map(|par| format!("{par}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Object(Arc<str>),
    Short,
    Boolean,
    Array(Box<FieldType>),
}

#[macro_export]
macro_rules! field {
    (byte) => {
        $crate::class::FieldType::Byte
    };
    (short) => {
        $crate::class::FieldType::Short
    };
    (int) => {
        $crate::class::FieldType::Int
    };
    (long) => {
        $crate::class::FieldType::Long
    };
    (float) => {
        $crate::class::FieldType::Float
    };
    (double) => {
        $crate::class::FieldType::Double
    };
    (char) => {
        $crate::class::FieldType::Char
    };
    (boolean) => {
        $crate::class::FieldType::Boolean
    };
    ([]$($rest:tt)*) => {
        $crate::class::FieldType::Array(Box::new($crate::field!($($rest)*)))
    };
    (Object($id:expr)) => {
        $crate::class::FieldType::Object($id)
    };
    (($($t:tt)*)) => {
        $crate::field!($($t)*)
    }
}

impl Display for FieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Boolean => write!(f, "boolean"),
            Self::Byte => write!(f, "byte"),
            Self::Char => write!(f, "char"),
            Self::Double => write!(f, "double"),
            Self::Float => write!(f, "float"),
            Self::Int => write!(f, "int"),
            Self::Long => write!(f, "long"),
            Self::Short => write!(f, "short"),
            Self::Array(inner) => write!(f, "{inner}[]"),
            Self::Object(class) => write!(f, "{class}"),
        }
    }
}

impl FieldType {
    #[must_use]
    pub const fn get_size(&self) -> usize {
        match self {
            Self::Double | Self::Long => 2,
            _ => 1,
        }
    }

    #[must_use]
    pub const fn is_reference(&self) -> bool {
        matches!(self, Self::Array(_) | Self::Object(_))
    }

    #[must_use]
    pub const fn idx(&self) -> u64 {
        match self {
            Self::Byte => 0,
            Self::Char => 1,
            Self::Double => 2,
            Self::Float => 3,
            Self::Int => 4,
            Self::Long => 5,
            Self::Object(_) => 6,
            Self::Short => 7,
            Self::Boolean => 8,
            Self::Array(_) => 9,
        }
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
