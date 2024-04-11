use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::{BitAnd, BitOr};
use std::sync::{Arc, Mutex, Once};

use crate::access;
use crate::class_loader::MethodName;
use crate::data::NULL;

pub use self::code::Code;
use self::code::NativeTodo;

pub mod code;

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
    /// names of all methods
    pub methods: Vec<MethodName>,
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
            methods: Vec::new(),
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
            .field("methods", &self.methods)
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

#[macro_export]
macro_rules! access {
    ($($acc:ident)*) => {
        $crate::class::AccessFlags(0) $(|$crate::access!(@$acc))*
    };

    (@public) => {
        $crate::class::AccessFlags::ACC_PUBLIC
    };

    (@static) => {
        $crate::class::AccessFlags::ACC_STATIC
    };

    (@native) => {
        $crate::class::AccessFlags::ACC_NATIVE
    };

    (@abstract) => {
        $crate::class::AccessFlags::ACC_ABSTRACT
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
/// Flag Name           Value   Interpretation
/// `ACC_PUBLIC`          0x0001  Declared public; may be accessed from outside its package.
/// `ACC_PRIVATE`         0x0002  Declared private; usable only within the defining class.
/// `ACC_PROTECTED`       0x0004  Declared protected; may be accessed within subclasses.
/// `ACC_STATIC`          0x0008  Declared static.
/// `ACC_FINAL`           0x0010  Declared final; never directly assigned to after object construction (JLS §17.5).
/// `ACC_SYNCHRONIZED`    0x0020  Declared synchronized; invocation is wrapped by a monitor use.
/// `ACC_VOLATILE`        0x0040  Declared volatile; cannot be cached.
/// `ACC_TRANSIENT`       0x0080  Declared transient; not written or read by a persistent object manager.
/// `ACC_NATIVE`          0x0100  Declared native; implemented in a language other than Java.
/// `ACC_ABSTRACT`        0x0400  Declared abstract; no implementation is provided.
/// `ACC_STRICT`          0x0800  Declared strictfp; floating-point mode is FP-strict.
/// `ACC_SYNTHETIC`       0x1000  Declared synthetic; not present in the source code.
/// `ACC_ENUM`            0x4000  Declared as an element of an enum.
pub struct AccessFlags(pub u16);

impl Debug for AccessFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AccessFlags({self})")
    }
}

impl Display for AccessFlags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 & 1 != 0 {
            write!(f, "public ")?;
        }
        if self.0 & 2 != 0 {
            write!(f, "private ")?;
        }
        if self.0 & 4 != 0 {
            write!(f, "protected ")?;
        }
        if self.0 & 8 != 0 {
            write!(f, "static ")?;
        }
        if self.0 & 0x10 != 0 {
            write!(f, "final ")?;
        }
        if self.0 & 0x20 != 0 {
            write!(f, "synchronized ")?;
        }
        if self.0 & 0x40 != 0 {
            write!(f, "volatile ")?;
        }
        if self.0 & 0x80 != 0 {
            write!(f, "transient ")?;
        }
        if self.0 & 0x100 != 0 {
            write!(f, "native ")?;
        }
        if self.0 & 0x400 != 0 {
            write!(f, "abstract ")?;
        }
        if self.0 & 0x800 != 0 {
            write!(f, "fp-strict ")?;
        }
        if self.0 & 0x1000 != 0 {
            write!(f, "synthetic ")?;
        }
        if self.0 & 0x4000 != 0 {
            write!(f, "enum ")?;
        }
        Ok(())
    }
}

impl AccessFlags {
    #[must_use]
    pub const fn is_static(self) -> bool {
        self.0 & Self::ACC_STATIC.0 != 0
    }
    #[must_use]
    pub const fn is_native(self) -> bool {
        self.0 & Self::ACC_NATIVE.0 != 0
    }
    #[must_use]
    pub const fn is_abstract(self) -> bool {
        self.0 & Self::ACC_ABSTRACT.0 != 0
    }

    // pub const ZERO: Self = Self(0);
    pub const ACC_PUBLIC: Self = Self(0x0001);
    // pub const ACC_PRIVATE: u16 = 0x0002;
    // pub const ACC_PROTECTED: u16 = 0x0004;
    pub const ACC_STATIC: Self = Self(0x0008);
    // pub const ACC_FINAL: u16 = 0x0010;
    // pub const ACC_SYNCHRONIZED: u16 = 0x0020;
    // pub const ACC_VOLATILE: u16 = 0x0040;
    // pub const ACC_TRANSIENT: u16 = 0x0080;
    pub const ACC_NATIVE: Self = Self(0x0100);
    // pub const ACC_???: u16 = 0x0200;
    pub const ACC_ABSTRACT: Self = Self(0x0400);
    // pub const ACC_STRICT: u16 = 0x0800;
    // pub const ACC_SYNTHETIC: u16 = 0x1000;
    // pub const ACC_???: u16 = 0x2000;
    // pub const ACC_ENUM: u16 = 0x4000;
    // pub const ACC_???: u16 = 0x8000;
}

impl BitOr for AccessFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitAnd for AccessFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
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
