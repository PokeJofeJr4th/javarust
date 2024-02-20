use std::fmt::{Debug, Display};
use std::ops::{BitAnd, BitOr};
use std::sync::{Arc, Mutex};

use crate::virtual_machine::Instruction;

pub struct Class {
    pub version: ClassVersion,
    pub constants: Vec<Constant>,
    pub access: AccessFlags,
    pub this: Arc<str>,
    pub super_class: Arc<str>,
    pub interfaces: Vec<Arc<str>>,
    pub field_size: usize,
    pub fields: Vec<(Field, usize)>,
    pub static_data: Mutex<Vec<u32>>,
    pub statics: Vec<(Field, usize)>,
    pub methods: Vec<Arc<Method>>,
    pub bootstrap_methods: Vec<BootstrapMethod>,
    pub source_file: Option<Arc<str>>,
    pub signature: Option<Arc<str>>,
    pub inner_classes: Vec<InnerClass>,
    pub attributes: Vec<Attribute>,
}

impl Class {
    pub fn new(access: AccessFlags, this: Arc<str>, super_class: Arc<str>) -> Self {
        Self {
            version: ClassVersion {
                minor_version: 0,
                major_version: 0,
            },
            constants: Vec::new(),
            access,
            this,
            super_class,
            interfaces: Vec::new(),
            field_size: 0,
            fields: Vec::new(),
            static_data: Mutex::new(Vec::new()),
            statics: Vec::new(),
            methods: Vec::new(),
            bootstrap_methods: Vec::new(),
            signature: None,
            source_file: None,
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
        let mut s = f.debug_struct("class");
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
        s.finish()
    }
}

#[derive(Debug, Clone)]
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
#[derive(Clone)]
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
    MethodType {
        index: u16,
    },
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
    pub fn bytes(&self) -> Vec<u32> {
        match self {
            Self::Int(i) => vec![*i as u32],
            Self::Float(f) => vec![f.to_bits()],
            Self::Long(l) => vec![*l as u64 as u32, (*l as u64 >> 32) as u32],
            Self::Double(f) => {
                let bits = f.to_bits();
                vec![bits as u32, (bits >> 32) as u32]
            }
            _ => vec![u32::MAX],
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
            Self::MethodType { index } => write!(f, "MethodType(#{index})"),
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
            write!(f, "strict ")?;
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
    pub const fn is_static(self) -> bool {
        self.0 & Self::ACC_STATIC.0 != 0
    }
    pub const fn is_native(self) -> bool {
        self.0 & Self::ACC_NATIVE.0 != 0
    }
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
    // pub const ACC_UNDEFINED: u16 = 0x0200;
    pub const ACC_ABSTRACT: Self = Self(0x0400);
    // pub const ACC_STRICT: u16 = 0x0800;
    // pub const ACC_SYNTHETIC: u16 = 0x1000;
    // pub const ACC_UNDEFINED: u16 = 0x2000;
    // pub const ACC_ENUM: u16 = 0x4000;
    // pub const ACC_UNDEFINED: u16 = 0x8000;
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

#[derive(Debug)]
pub struct ClassVersion {
    pub minor_version: u16,
    pub major_version: u16,
}

pub struct Field {
    pub access_flags: AccessFlags,
    pub name: Arc<str>,
    pub descriptor: FieldType,
    pub constant_value: Option<Constant>,
    pub signature: Option<Arc<str>>,
    pub attributes: Vec<Attribute>,
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
    pub descriptor: MethodDescriptor,
    pub code: Option<Code>,
    pub signature: Option<Arc<str>>,
    pub attributes: Vec<Attribute>,
}

impl Debug for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{:?} ", self.access_flags, self.descriptor)?;
        let mut s = f.debug_struct(&self.name);
        s.field("max_locals", &self.max_locals);
        if let Some(signature) = &self.signature {
            s.field("signature", signature);
        }
        if let Some(code) = &self.code {
            s.field("Code", code);
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
    pub parameters: Vec<FieldType>,
    pub return_type: Option<FieldType>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
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
    pub const fn get_size(&self) -> usize {
        match self {
            Self::Double | Self::Long => 2,
            _ => 1,
        }
    }
}

#[derive(Debug)]
pub struct Attribute {
    pub name: Arc<str>,
    pub data: Vec<u8>,
}

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

pub struct Code {
    pub max_stack: u16,
    pub code: Vec<Instruction>,
    pub exception_table: Vec<(u16, u16, u16, Option<Arc<str>>)>,
    pub line_number_table: Vec<LineTableEntry>,
    pub local_type_table: Vec<LocalVarTypeEntry>,
    pub local_var_table: Vec<LocalVarEntry>,
    pub stack_map: Vec<StackMapFrame>,
    pub attributes: Vec<Attribute>,
}

impl Debug for Code {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct("Code");
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
