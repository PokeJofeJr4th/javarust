use std::fmt::Debug;
use std::rc::Rc;

#[derive(Debug)]
pub struct Class {
    pub version: ClassVersion,
    pub constants: Vec<Constant>,
    pub access: AccessFlags,
    pub this: Rc<str>,
    pub super_class: Rc<str>,
    pub interfaces: Vec<u16>,
    pub field_size: usize,
    pub fields: Vec<(Field, usize)>,
    pub methods: Vec<Rc<Method>>,
    pub attributes: Vec<Attribute>,
}

/// A member of the constant pool
#[derive(Debug, Clone)]
pub enum Constant {
    String(Rc<str>),
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    ClassRef(Rc<str>),
    StringRef(Rc<str>),
    FieldRef {
        class: Rc<str>,
        name: Rc<str>,
        field_type: FieldType,
    },
    MethodRef {
        class: Rc<str>,
        name: Rc<str>,
        method_type: MethodDescriptor,
    },
    InterfaceRef {
        class: Rc<str>,
        name: Rc<str>,
        interface_type: Rc<str>,
    },
    NameTypeDescriptor {
        name: Rc<str>,
        type_descriptor: Rc<str>,
    },
    MethodHandle {
        descriptor: u8,
        index: u16,
    },
    MethodType {
        index: u16,
    },
    // Dynamic {
    //     constant: u32,
    // },
    InvokeDynamic {
        bootstrap_index: u16,
        name_type_index: u16,
    },
    // Module {
    //     identity: u16,
    // },
    // Package {
    //     identity: u16,
    // },
}

#[derive(Debug, Clone, Copy)]
/// Flag Name           Value   Interpretation
/// ACC_PUBLIC          0x0001  Declared public; may be accessed from outside its package.
/// ACC_PRIVATE         0x0002  Declared private; usable only within the defining class.
/// ACC_PROTECTED       0x0004  Declared protected; may be accessed within subclasses.
/// ACC_STATIC          0x0008  Declared static.
/// ACC_FINAL           0x0010  Declared final; never directly assigned to after object construction (JLS §17.5).
/// ACC_SYNCHRONIZED    0x0020  Declared synchronized; invocation is wrapped by a monitor use.
/// ACC_VOLATILE        0x0040  Declared volatile; cannot be cached.
/// ACC_TRANSIENT       0x0080  Declared transient; not written or read by a persistent object manager.
/// ACC_NATIVE          0x0100  Declared native; implemented in a language other than Java.
/// ACC_ABSTRACT        0x0400  Declared abstract; no implementation is provided.
/// ACC_STRICT          0x0800  Declared strictfp; floating-point mode is FP-strict.
/// ACC_SYNTHETIC       0x1000  Declared synthetic; not present in the source code.
/// ACC_ENUM            0x4000  Declared as an element of an enum.
pub struct AccessFlags(pub u16);

impl AccessFlags {
    pub const fn is_static(self) -> bool {
        self.0 & Self::ACC_STATIC != 0
    }
    pub const fn is_native(self) -> bool {
        self.0 & Self::ACC_NATIVE != 0
    }
    pub const fn is_abstract(self) -> bool {
        self.0 & Self::ACC_ABSTRACT != 0
    }

    pub const ACC_PUBLIC: u16 = 0x0001;
    // pub const ACC_PRIVATE: u16 = 0x0002;
    // pub const ACC_PROTECTED: u16 = 0x0004;
    pub const ACC_STATIC: u16 = 0x0008;
    // pub const ACC_FINAL: u16 = 0x0010;
    // pub const ACC_SYNCHRONIZED: u16 = 0x0020;
    // pub const ACC_VOLATILE: u16 = 0x0040;
    // pub const ACC_TRANSIENT: u16 = 0x0080;
    pub const ACC_NATIVE: u16 = 0x0100;
    // pub const ACC_UNDEFINED: u16 = 0x0200;
    pub const ACC_ABSTRACT: u16 = 0x0400;
    // pub const ACC_STRICT: u16 = 0x0800;
    // pub const ACC_SYNTHETIC: u16 = 0x1000;
    // pub const ACC_UNDEFINED: u16 = 0x2000;
    // pub const ACC_ENUM: u16 = 0x4000;
    // pub const ACC_UNDEFINED: u16 = 0x8000;
}

#[derive(Debug)]
pub struct ClassVersion {
    pub minor_version: u16,
    pub major_version: u16,
}

#[derive(Debug)]
pub struct Field {
    pub access_flags: AccessFlags,
    pub name: Rc<str>,
    pub descriptor: FieldType,
    pub attributes: Vec<Attribute>,
    pub constant_value: Option<Constant>,
}

#[derive(Debug)]
pub struct Method {
    pub access_flags: AccessFlags,
    pub name: Rc<str>,
    pub descriptor: MethodDescriptor,
    pub attributes: Vec<Attribute>,
    pub code: Option<Code>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodDescriptor {
    pub parameter_size: usize,
    pub parameters: Vec<FieldType>,
    pub return_type: Option<FieldType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Byte,
    Char,
    Double,
    Float,
    Int,
    Long,
    Object(Rc<str>),
    Short,
    Boolean,
    Array(Box<FieldType>),
}

impl FieldType {
    pub fn get_size(&self) -> usize {
        match self {
            Self::Double | Self::Long => 2,
            _ => 1,
        }
    }
}

#[derive(Debug)]
pub struct Attribute {
    pub name: Rc<str>,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub exception_table: Vec<(u16, u16, u16, Option<Rc<str>>)>,
    pub attributes: Vec<Attribute>,
    pub stack_map: Vec<StackMapFrame>,
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
    Object { class_name: Rc<str> },
    Uninitialized { offset: u16 },
    Long,
    Double,
}
