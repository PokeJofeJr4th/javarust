#[derive(Debug)]
pub struct Class {
    pub constants: Vec<Constant>,
    pub access: AccessFlags,
    pub this: String,
    pub super_class: String,
    pub interfaces: Vec<u16>,
    pub fields: Vec<()>,
    pub methods: Vec<()>,
    pub attributes: ClassAttributes,
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
        idk: u32,
    },
    Module {
        identity: u16,
    },
    Package {
        identity: u16,
    },
}

#[derive(Debug)]
pub struct AccessFlags(pub u16);

#[derive(Debug)]
pub struct ClassAttributes {
    pub minor_version: u16,
    pub major_version: u16,
}
