use std::{iter::Peekable, rc::Rc, sync::Arc};

use crate::class::{
    AccessFlags, Attribute, Class, ClassVersion, Code, Constant, Field, FieldType, Method,
    MethodDescriptor, StackMapFrame, VerificationTypeInfo,
};

/// A member of the constant pool
#[derive(Debug, Clone)]
pub enum RawConstant {
    String(Rc<str>),
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

pub fn load_class(bytes: &mut impl Iterator<Item = u8>) -> Result<Class, String> {
    let 0xCAFEBABE = get_u32(bytes)? else { return Err(String::from("Invalid header")) };
    let version = ClassVersion {
        minor_version: get_u16(bytes)?,
        major_version: get_u16(bytes)?,
    };
    let const_count = get_u16(bytes)?;

    let mut raw_constants = Vec::new();
    for _ in 1..const_count {
        match bytes.next() {
            Some(1) => {
                let strlen = get_u16(bytes)?;
                let string = bytes.take(strlen as usize).collect::<Vec<u8>>();
                let string = parse_java_string(string)?;
                raw_constants.push(RawConstant::String(string.into()));
            }
            Some(3) => {
                let bits = get_u32(bytes)? as i32;
                raw_constants.push(RawConstant::Int(bits));
            }
            Some(4) => {
                let bits = f32::from_bits(get_u32(bytes)?);
                raw_constants.push(RawConstant::Float(bits));
            }
            Some(5) => {
                let bits = get_u64(bytes)? as i64;
                raw_constants.push(RawConstant::Long(bits));
            }
            Some(6) => {
                let bits = f64::from_bits(get_u64(bytes)?);
                raw_constants.push(RawConstant::Double(bits));
            }
            Some(7) => {
                let string_addr = get_u16(bytes)?;
                raw_constants.push(RawConstant::ClassRef { string_addr });
            }
            Some(8) => {
                let string_addr = get_u16(bytes)?;
                raw_constants.push(RawConstant::StringRef { string_addr });
            }
            Some(9) => {
                let class_ref_addr = get_u16(bytes)?;
                let name_type_addr = get_u16(bytes)?;
                raw_constants.push(RawConstant::FieldRef {
                    class_ref_addr,
                    name_type_addr,
                })
            }
            Some(10) => {
                let class_ref_addr = get_u16(bytes)?;
                let name_type_addr = get_u16(bytes)?;
                raw_constants.push(RawConstant::MethodRef {
                    class_ref_addr,
                    name_type_addr,
                });
            }
            Some(11) => {
                let class_ref_addr = get_u16(bytes)?;
                let name_type_addr = get_u16(bytes)?;
                raw_constants.push(RawConstant::InterfaceRef {
                    class_ref_addr,
                    name_type_addr,
                });
            }
            Some(12) => {
                let name_desc_addr = get_u16(bytes)?;
                let type_addr = get_u16(bytes)?;
                raw_constants.push(RawConstant::NameTypeDescriptor {
                    name_desc_addr,
                    type_addr,
                });
            }
            Some(15) => {
                let Some(descriptor) = bytes.next() else { return Err(String::from("Unexpected EOF"))};
                let index = get_u16(bytes)?;
                raw_constants.push(RawConstant::MethodHandle { descriptor, index });
            }
            Some(16) => {
                let index = get_u16(bytes)?;
                raw_constants.push(RawConstant::MethodType { index });
            }
            Some(18) => {
                let bootstrap_index = get_u16(bytes)?;
                let name_type_index = get_u16(bytes)?;
                raw_constants.push(RawConstant::InvokeDynamic {
                    bootstrap_index,
                    name_type_index,
                });
            }
            other => return Err(format!("Ugh, {other:?}")),
        }
        // println!("{constants:?}");
    }

    let constants = raw_constants
        .iter()
        .map(|constant| cook_constant(&raw_constants, constant))
        .collect::<Result<Vec<_>, _>>()?;

    let access = AccessFlags(get_u16(bytes)?);
    let this_class = get_u16(bytes)?;
    let this_class = class_index(&raw_constants, this_class as usize)?;

    let super_class = get_u16(bytes)?;
    let super_class = class_index(&raw_constants, super_class as usize)?;

    let interface_count = get_u16(bytes)?;
    let mut interfaces = Vec::new();
    for _ in 0..interface_count {
        interfaces.push(get_u16(bytes)?);
    }

    let field_count = get_u16(bytes)?;
    let mut fields = Vec::new();
    for _ in 0..field_count {
        let access_flags = AccessFlags(get_u16(bytes)?);
        let name_idx = get_u16(bytes)?;
        let name = str_index(&raw_constants, name_idx as usize)?;
        let descriptor_idx = get_u16(bytes)?;
        let descriptor = str_index(&raw_constants, descriptor_idx as usize)?;
        let descriptor = parse_field_type(&mut descriptor.chars().peekable())?;
        let attrs_count = get_u16(bytes)?;
        let mut attributes = Vec::new();
        for _ in 0..attrs_count {
            attributes.push(get_attribute(&raw_constants, bytes)?);
        }

        let constant_value = if access_flags.is_static() {
            let [const_idx] = attributes.iter().filter(|attr| &*attr.name == "ConstantValue").collect::<Vec<_>>()[..] else {
                return Err(String::from("Static field must have exactly one `ConstantValue` attribute"))
            };
            let [b0, b1] = const_idx.data[..] else {
                return Err(String::from("`ConstantValue` attribute must have exactly two bytes"))
            };
            let Some(constant) = constants.get((b0 as usize) << 8 | b1 as usize) else {
                return Err(String::from("`ConstantValue` attribute has invalid constant index"))
            };
            Some(constant.clone())
        } else {
            None
        };

        fields.push(Field {
            access_flags,
            name,
            descriptor,
            attributes,
            constant_value,
        })
    }

    let method_count: u16 = get_u16(bytes)?;
    let mut methods = Vec::new();
    // println!("{method_count} methods");
    for _ in 0..method_count {
        let access_flags = AccessFlags(get_u16(bytes)?);
        let name_idx = get_u16(bytes)?;
        let name = str_index(&raw_constants, name_idx as usize)?;
        let descriptor_idx = get_u16(bytes)?;
        let descriptor = str_index(&raw_constants, descriptor_idx as usize)?;
        let descriptor = parse_method_descriptor(&descriptor)?;
        let attrs_count = get_u16(bytes)?;
        let mut attributes = Vec::new();
        for _ in 0..attrs_count {
            attributes.push(get_attribute(&raw_constants, bytes)?);
        }

        let (code_attributes, attributes): (Vec<_>, Vec<_>) = attributes
            .into_iter()
            .partition(|attr| &*attr.name == "Code");
        // println!("Method {name}: {descriptor}; {attrs_count} attrs");
        // println!("{attributes:?}");
        // println!("{access:?}");
        let code = match (
            access.is_native() || access.is_abstract(),
            &code_attributes[..],
        ) {
            (true, []) => None,
            (false, [code]) => {
                let bytes = code.data.clone();
                let code = parse_code_attribute(&raw_constants, bytes)?;
                Some(code)
            }
            (true, [_]) => return Err(String::from("Method marked as native or abstract")),
            // (false, []) => return Err(String::from("Method must contain code")),
            (false, []) => None,
            _ => return Err(String::from("Method must only have one code attribute")),
        };
        methods.push(Arc::new(Method {
            access_flags,
            name,
            descriptor,
            attributes,
            code,
        }));
    }

    let attribute_count = get_u16(bytes)?;
    let mut attributes = Vec::new();
    for _ in 0..attribute_count {
        attributes.push(get_attribute(&raw_constants, bytes)?);
    }

    Ok(Class {
        constants,
        access,
        this: this_class,
        super_class,
        interfaces,
        fields,
        methods,
        version,
        attributes,
    })
}

fn get_attribute(
    constants: &[RawConstant],
    bytes: &mut impl Iterator<Item = u8>,
) -> Result<Attribute, String> {
    let name_idx = get_u16(bytes)?;
    let name = str_index(constants, name_idx as usize)?;
    let attr_length = get_u32(bytes)?;
    Ok(Attribute {
        name,
        data: bytes.take(attr_length as usize).collect::<Vec<_>>(),
    })
}

fn parse_code_attribute(constants: &[RawConstant], bytes: Vec<u8>) -> Result<Code, String> {
    let mut bytes = bytes.into_iter();
    let max_stack = get_u16(&mut bytes)?;
    let max_locals = get_u16(&mut bytes)?;
    let code_length = get_u32(&mut bytes)?;
    let code = (&mut bytes).take(code_length as usize).collect::<Vec<_>>();

    let exception_table_length = get_u16(&mut bytes)?;
    let mut exception_table = Vec::new();
    for _ in 0..exception_table_length {
        let start_pc = get_u16(&mut bytes)?;
        let end_pc = get_u16(&mut bytes)?;
        let handler_pc = get_u16(&mut bytes)?;
        let catch_type = get_u16(&mut bytes)?;
        let catch_type = if catch_type == 0 {
            None
        } else {
            Some(str_index(constants, catch_type as usize)?)
        };
        exception_table.push((start_pc, end_pc, handler_pc, catch_type));
    }

    let attrs_count = get_u16(&mut bytes)?;
    let mut attributes = Vec::new();
    for _ in 0..attrs_count {
        attributes.push(get_attribute(constants, &mut bytes)?);
    }

    let (stack_map_attrs, attributes) = {
        let split: (Vec<_>, Vec<_>) = attributes
            .into_iter()
            .partition(|attr| &*attr.name == "StackMapTable");
        split
    };
    let stack_map = match &stack_map_attrs[..] {
        [attr] => {
            let mut stack_map = Vec::new();
            let mut bytes = attr.data.iter().copied();
            let frame_count = get_u16(&mut bytes)?;
            for _ in 0..frame_count {
                stack_map.push(match bytes.next() {
                    Some(offset_delta @ 0..=63) => StackMapFrame::Same { offset_delta },
                    Some(offset_delta @ 64..=127) => StackMapFrame::SameLocals1Stack {
                        offset_delta: offset_delta & 0x3F,
                        verification: parse_verification_type(constants, &mut bytes)?,
                    },
                    Some(247) => {
                        let offset_delta = get_u16(&mut bytes)?;
                        StackMapFrame::SameLocals1StackExtended {
                            offset_delta,
                            verification: parse_verification_type(constants, &mut bytes)?,
                        }
                    }
                    Some(chop @ 248..=250) => {
                        let chop = 251 - chop;
                        let offset_delta = get_u16(&mut bytes)?;
                        StackMapFrame::Chop { chop, offset_delta }
                    }
                    Some(251) => {
                        let offset_delta = get_u16(&mut bytes)?;
                        StackMapFrame::SameExtended { offset_delta }
                    }
                    Some(append @ 252..=254) => {
                        let append = append - 251;
                        let offset_delta = get_u16(&mut bytes)?;
                        let mut locals = Vec::new();
                        for _ in 0..append {
                            locals.push(parse_verification_type(constants, &mut bytes)?);
                        }
                        StackMapFrame::Append {
                            offset_delta,
                            locals,
                        }
                    }
                    Some(255) => {
                        let offset_delta = get_u16(&mut bytes)?;
                        let locals_count = get_u16(&mut bytes)?;
                        let mut locals = Vec::new();
                        for _ in 0..locals_count {
                            locals.push(parse_verification_type(constants, &mut bytes)?);
                        }
                        let stack_count = get_u16(&mut bytes)?;
                        let mut stack = Vec::new();
                        for _ in 0..stack_count {
                            stack.push(parse_verification_type(constants, &mut bytes)?);
                        }
                        StackMapFrame::Full {
                            offset_delta,
                            locals,
                            stack,
                        }
                    }
                    other => return Err(format!("Bad stackmap discriminator; {other:?}")),
                })
            }
            stack_map
        }
        [] => Vec::new(),
        _ => return Err(String::from("Only one `StackMapTable` attribute expected")),
    };

    Ok(Code {
        max_stack,
        max_locals,
        code,
        exception_table,
        attributes,
        stack_map,
    })
}

fn parse_verification_type(
    constants: &[RawConstant],
    bytes: &mut impl Iterator<Item = u8>,
) -> Result<VerificationTypeInfo, String> {
    match bytes.next() {
        Some(0) => Ok(VerificationTypeInfo::Top),
        Some(1) => Ok(VerificationTypeInfo::Integer),
        Some(2) => Ok(VerificationTypeInfo::Float),
        Some(5) => Ok(VerificationTypeInfo::Null),
        Some(6) => Ok(VerificationTypeInfo::UninitializedThis),
        Some(7) => {
            let index = get_u16(bytes)?;
            let class_name = class_index(constants, index as usize)?;
            Ok(VerificationTypeInfo::Object { class_name })
        }
        Some(8) => {
            let offset = get_u16(bytes)?;
            Ok(VerificationTypeInfo::Uninitialized { offset })
        }
        Some(4) => Ok(VerificationTypeInfo::Long),
        Some(3) => Ok(VerificationTypeInfo::Double),
        other => Err(format!("Invalid verification type info: `{other:?}`")),
    }
}

fn get_bytes<const N: usize>(bytes: &mut impl Iterator<Item = u8>) -> Result<[u8; N], String> {
    <[u8; N]>::try_from(bytes.take(N).collect::<Vec<_>>())
        .map_err(|_| String::from("Unexpected EOF"))
}

fn get_u16(bytes: &mut impl Iterator<Item = u8>) -> Result<u16, String> {
    let bytes = get_bytes::<2>(bytes)?;
    Ok(u16::from_be_bytes(bytes))
}

fn get_u32(bytes: &mut impl Iterator<Item = u8>) -> Result<u32, String> {
    let bytes = get_bytes::<4>(bytes)?;
    Ok(u32::from_be_bytes(bytes))
}

fn get_u64(bytes: &mut impl Iterator<Item = u8>) -> Result<u64, String> {
    let bytes = get_bytes::<8>(bytes)?;
    Ok(u64::from_be_bytes(bytes))
}

fn str_index(constants: &[RawConstant], idx: usize) -> Result<Rc<str>, String> {
    match constants.get(idx - 1) {
        Some(RawConstant::String(str)) => Ok(str.clone()),
        Some(other) => Err(format!("Expected a string; got `{other:?}`")),
        None => Err(String::from("Unexpected EOF")),
    }
}

fn class_index(constants: &[RawConstant], idx: usize) -> Result<Rc<str>, String> {
    match constants.get(idx - 1) {
        Some(RawConstant::ClassRef { string_addr }) => str_index(constants, *string_addr as usize),
        Some(other) => Err(format!("Expected a string; got `{other:?}`")),
        None => Err(String::from("Unexpected EOF")),
    }
}

fn parse_java_string(bytes: Vec<u8>) -> Result<String, String> {
    let mut bytes = bytes.into_iter();
    let bytes = &mut bytes;
    let mut str = String::new();
    while let Some(b) = bytes.next() {
        if b == 0 {
            return Err(String::from("No byte can have the value zero"));
        } else if b < 128 {
            str.push(b as char);
        } else if b & 0b1110_0000 == 0b1100_0000 {
            let Some(y) = bytes.next() else { return Err(String::from("Unexpected end of string"))};
            let chr = ((b as u16 & 0x1f) << 6) | (y as u16 & 0x3f);
            str.push(
                char::from_u32(chr as u32).ok_or_else(|| String::from("Invalid character code"))?,
            );
        } else if b == 0b1110_1101 {
            let [v, w, x, y, z] = get_bytes(bytes)?;
            let chr = 0x10000
                | ((v as u32 & 0x0f) << 16)
                | ((w as u32 & 0x3f) << 10)
                | ((y as u32 & 0x0f) << 6)
                | (z as u32 & 0x3f);
            let chr = char::from_u32(chr).ok_or_else(|| String::from("Invalid character code"))?;
            str.push(chr);
        } else if b & 0b1111_0000 == 0b1110_0000 {
            let Some(y) = bytes.next() else { return Err(String::from("Unexpected end of string"))};
            let Some(z) = bytes.next() else { return Err(String::from("Unexpected end of string"))};
            let chr = ((b as u32 & 0xf) << 12) | ((y as u32 & 0x3f) << 6) | (z as u32 & 0x3f);
            let chr = char::from_u32(chr).ok_or_else(|| String::from("Invalid character code"))?;
            str.push(chr);
        }
    }
    Ok(str)
}

fn cook_constant(constants: &[RawConstant], constant: &RawConstant) -> Result<Constant, String> {
    Ok(match constant {
        RawConstant::ClassRef { string_addr } => {
            Constant::ClassRef(str_index(constants, *string_addr as usize)?)
        }
        RawConstant::Double(d) => Constant::Double(*d),
        RawConstant::Dynamic { constant } => Constant::Dynamic {
            constant: *constant,
        },
        RawConstant::FieldRef {
            class_ref_addr,
            name_type_addr,
        } => {
            let class = class_index(constants, *class_ref_addr as usize)?;
            let (name, field_type) = name_type_index(constants, *name_type_addr as usize)?;
            let field_type = parse_field_type(&mut field_type.chars().peekable())?;
            Constant::FieldRef {
                class,
                name,
                field_type,
            }
        }
        RawConstant::Float(f) => Constant::Float(*f),
        RawConstant::Int(i) => Constant::Int(*i),
        RawConstant::InterfaceRef {
            class_ref_addr,
            name_type_addr,
        } => {
            let class = class_index(constants, *class_ref_addr as usize)?;
            let (name, interface_type) = name_type_index(constants, *name_type_addr as usize)?;
            Constant::InterfaceRef {
                class,
                name,
                interface_type,
            }
        }
        &RawConstant::InvokeDynamic {
            bootstrap_index,
            name_type_index,
        } => Constant::InvokeDynamic {
            bootstrap_index,
            name_type_index,
        },
        RawConstant::Long(l) => Constant::Long(*l),
        &RawConstant::MethodHandle { descriptor, index } => {
            Constant::MethodHandle { descriptor, index }
        }
        RawConstant::MethodRef {
            class_ref_addr,
            name_type_addr,
        } => {
            let class = class_index(constants, *class_ref_addr as usize)?;
            let (name, method_type) = name_type_index(constants, *name_type_addr as usize)?;
            let method_type = parse_method_descriptor(&method_type)?;
            Constant::MethodRef {
                class,
                name,
                method_type,
            }
        }
        &RawConstant::MethodType { index } => Constant::MethodType { index },
        &RawConstant::Module { identity } => Constant::Module { identity },
        RawConstant::NameTypeDescriptor {
            name_desc_addr,
            type_addr,
        } => {
            let name = str_index(constants, *name_desc_addr as usize)?;
            let type_descriptor = str_index(constants, *type_addr as usize)?;
            Constant::NameTypeDescriptor {
                name,
                type_descriptor,
            }
        }
        &RawConstant::Package { identity } => Constant::Package { identity },
        RawConstant::String(string) => Constant::String(string.clone()),
        RawConstant::StringRef { string_addr } => {
            let string = str_index(constants, *string_addr as usize)?;
            Constant::StringRef(string)
        }
    })
}

fn name_type_index(constants: &[RawConstant], idx: usize) -> Result<(Rc<str>, Rc<str>), String> {
    let Some(RawConstant::NameTypeDescriptor { name_desc_addr, type_addr }) = constants.get(idx - 1) else {
        return Err(String::from("Invalid NameTypeDescriptor for FieldRef"))
    };
    let name = str_index(constants, *name_desc_addr as usize)?;
    let type_name = str_index(constants, *type_addr as usize)?;
    Ok((name, type_name))
}

fn parse_method_descriptor(src: &str) -> Result<MethodDescriptor, String> {
    let mut chars = src.chars().peekable();
    let chars = &mut chars;
    let Some('(') = chars.next() else {
        return Err(String::from("Expected `(`"))
    };
    let mut parameters = Vec::new();
    while chars.peek() != Some(&')') {
        parameters.push(parse_field_type(chars)?);
    }
    chars.next();
    let return_type = match chars.peek() {
        Some('V') => None,
        _ => Some(parse_field_type(chars)?),
    };
    Ok(MethodDescriptor {
        parameters,
        return_type,
    })
}

fn parse_field_type(chars: &mut Peekable<impl Iterator<Item = char>>) -> Result<FieldType, String> {
    match chars.next() {
        Some('B') => Ok(FieldType::Byte),
        Some('C') => Ok(FieldType::Char),
        Some('D') => Ok(FieldType::Double),
        Some('F') => Ok(FieldType::Float),
        Some('I') => Ok(FieldType::Int),
        Some('J') => Ok(FieldType::Long),
        Some('L') => {
            let mut class_buf = String::new();
            while let Some(char) = chars.peek() {
                if char == &';' {
                    chars.next();
                    break;
                }
                class_buf.push(chars.next().unwrap());
            }
            Ok(FieldType::Object(class_buf.into()))
        }
        Some('S') => Ok(FieldType::Short),
        Some('Z') => Ok(FieldType::Boolean),
        Some('[') => Ok(FieldType::Array(Box::new(parse_field_type(chars)?))),
        other => Err(format!("bad field type {other:?}")),
    }
}
