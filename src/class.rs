#[derive(Debug)]
pub struct Class {
    pub version: ClassVersion,
    pub constants: Vec<Constant>,
    pub access: AccessFlags,
    pub this: String,
    pub super_class: String,
    pub interfaces: Vec<u16>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
}

/// A member of the constant pool
#[derive(Debug, Clone)]
pub enum Constant {
    String(String),
    Int(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    ClassRef {
        /// index in constant pool for a String value (internally-qualified class name)
        string_addr: u16,
    },
    StringRef {
        /// index in constant pool for a String value
        string_addr: u16,
    },
    FieldRef {
        /// index in constant pool for a ClassRef value
        class_ref_addr: u16,
        /// index in constant pool for a NameTypeDescriptor value
        name_type_addr: u16,
    },
    MethodRef {
        /// index in constant pool for a ClassRef value
        class_ref_addr: u16,
        /// index in constant pool for a NameTypeDescriptor value
        name_type_addr: u16,
    },
    InterfaceRef {
        /// index in constant pool for a ClassRef value
        class_ref_addr: u16,
        /// index in constant pool for a NameTypeDescriptor value
        name_type_addr: u16,
    },
    NameTypeDescriptor {
        /// index in constant pool for a String value for the name
        name_desc_addr: u16,
        /// index in constant pool for a String value - specially encoded type
        type_addr: u16,
    },
    MethodHandle {
        descriptor: u8,
        index: u16,
    },
    MethodType {
        index: u16,
    },
    Dynamic {
        constant: u32,
    },
    InvokeDynamic {
        bootstrap_index: u16,
        name_type_index: u16,
    },
    Module {
        identity: u16,
    },
    Package {
        identity: u16,
    },
}

#[derive(Debug, Clone, Copy)]
/// Flag Name 	        Value 	Interpretation
/// ACC_PUBLIC 	        0x0001 	Declared public; may be accessed from outside its package.
/// ACC_PRIVATE 	    0x0002 	Declared private; usable only within the defining class.
/// ACC_PROTECTED 	    0x0004 	Declared protected; may be accessed within subclasses.
/// ACC_STATIC 	        0x0008 	Declared static.
/// ACC_FINAL 	        0x0010 	Declared final; never directly assigned to after object construction (JLS §17.5).
/// ACC_SYNCHRONIZED 	0x0020 	Declared synchronized; invocation is wrapped by a monitor use.
/// ACC_VOLATILE 	    0x0040 	Declared volatile; cannot be cached.
/// ACC_TRANSIENT 	    0x0080 	Declared transient; not written or read by a persistent object manager.
/// ACC_NATIVE 	        0x0100 	Declared native; implemented in a language other than Java.
/// ACC_ABSTRACT 	    0x0400 	Declared abstract; no implementation is provided.
/// ACC_STRICT 	        0x0800 	Declared strictfp; floating-point mode is FP-strict.
/// ACC_SYNTHETIC 	    0x1000 	Declared synthetic; not present in the source code.
/// ACC_ENUM 	        0x4000 	Declared as an element of an enum.
pub struct AccessFlags(pub u16);

impl AccessFlags {
    pub const fn is_static(self) -> bool {
        self.0 & 0x0008 != 0
    }
    pub const fn is_native(self) -> bool {
        self.0 & 0x0100 != 0
    }
    pub const fn is_abstract(self) -> bool {
        self.0 & 0x0400 != 0
    }
}

#[derive(Debug)]
pub struct ClassVersion {
    pub minor_version: u16,
    pub major_version: u16,
}

#[derive(Debug)]
pub struct Field {
    pub access_flags: AccessFlags,
    pub name: String,
    pub descriptor: String,
    pub attributes: Vec<Attribute>,
    pub constant_value: Option<Constant>,
}

#[derive(Debug)]
pub struct Method {
    pub access_flags: AccessFlags,
    pub name: String,
    pub descriptor: String,
    pub attributes: Vec<Attribute>,
    pub code: Option<Code>,
}

#[derive(Debug)]
pub struct Attribute {
    pub name: String,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub exception_table: Vec<(u16, u16, u16, Option<String>)>,
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
        stack: Vec<()>,
    },
}

#[derive(Debug)]
pub enum VerificationTypeInfo {
    Top,
    Integer,
    Float,
    Null,
    UninitializedThis,
    Object { class_name: String },
    Uninitialized { offset: u16 },
    Long,
    Double,
}
