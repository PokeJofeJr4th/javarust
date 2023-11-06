use crate::class::{
    AccessFlags, Attribute, Class, ClassVersion, Code, Constant, Field, Method, StackMapFrame,
};

pub fn load_class(bytes: &mut impl Iterator<Item = u8>) -> Result<Class, String> {
    let 0xCAFEBABE = get_u32(bytes)? else { return Err(String::from("Invalid header")) };
    let version = ClassVersion {
        minor_version: get_u16(bytes)?,
        major_version: get_u16(bytes)?,
    };
    let const_count = get_u16(bytes)?;

    let mut constants = Vec::new();
    for _ in 1..const_count {
        match bytes.next() {
            Some(1) => {
                let strlen = get_u16(bytes)?;
                let string = bytes.take(strlen as usize).collect::<Vec<u8>>();
                let string = parse_java_string(string)?;
                constants.push(Constant::String(string));
            }
            Some(3) => {
                let bits = get_u32(bytes)? as i32;
                constants.push(Constant::Int(bits));
            }
            Some(4) => {
                let bits = f32::from_bits(get_u32(bytes)?);
                constants.push(Constant::Float(bits));
            }
            Some(5) => {
                let bits = get_u64(bytes)? as i64;
                constants.push(Constant::Long(bits));
            }
            Some(6) => {
                let bits = f64::from_bits(get_u64(bytes)?);
                constants.push(Constant::Double(bits));
            }
            Some(7) => {
                let string_addr = get_u16(bytes)?;
                constants.push(Constant::ClassRef { string_addr });
            }
            Some(8) => {
                let string_addr = get_u16(bytes)?;
                constants.push(Constant::StringRef { string_addr });
            }
            Some(9) => {
                let class_ref_addr = get_u16(bytes)?;
                let name_type_addr = get_u16(bytes)?;
                constants.push(Constant::FieldRef {
                    class_ref_addr,
                    name_type_addr,
                })
            }
            Some(10) => {
                let class_ref_addr = get_u16(bytes)?;
                let name_type_addr = get_u16(bytes)?;
                constants.push(Constant::MethodRef {
                    class_ref_addr,
                    name_type_addr,
                });
            }
            Some(11) => {
                let class_ref_addr = get_u16(bytes)?;
                let name_type_addr = get_u16(bytes)?;
                constants.push(Constant::InterfaceRef {
                    class_ref_addr,
                    name_type_addr,
                });
            }
            Some(12) => {
                let name_desc_addr = get_u16(bytes)?;
                let type_addr = get_u16(bytes)?;
                constants.push(Constant::NameTypeDescriptor {
                    name_desc_addr,
                    type_addr,
                });
            }
            Some(15) => {
                let Some(descriptor) = bytes.next() else { return Err(String::from("Unexpected EOF"))};
                let index = get_u16(bytes)?;
                constants.push(Constant::MethodHandle { descriptor, index });
            }
            Some(16) => {
                let index = get_u16(bytes)?;
                constants.push(Constant::MethodType { index });
            }
            Some(18) => {
                let bootstrap_index = get_u16(bytes)?;
                let name_type_index = get_u16(bytes)?;
                constants.push(Constant::InvokeDynamic {
                    bootstrap_index,
                    name_type_index,
                });
            }
            other => return Err(format!("Ugh, {other:?}")),
        }
        // println!("{constants:?}");
    }

    let access = AccessFlags(get_u16(bytes)?);
    let this_class = get_u16(bytes)?;
    let this_class = class_index(&constants, this_class as usize)?;

    let super_class = get_u16(bytes)?;
    let super_class = class_index(&constants, super_class as usize)?;

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
        let name = str_index(&constants, name_idx as usize)?;
        let descriptor_idx = get_u16(bytes)?;
        let descriptor = str_index(&constants, descriptor_idx as usize)?;
        let attrs_count = get_u16(bytes)?;
        let mut attributes = Vec::new();
        for _ in 0..attrs_count {
            attributes.push(get_attribute(&constants, bytes)?);
        }

        let constant_value = if access_flags.is_static() {
            let [const_idx] = attributes.iter().filter(|attr| attr.name == "ConstantValue").collect::<Vec<_>>()[..] else {
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
        let name = str_index(&constants, name_idx as usize)?;
        let descriptor_idx = get_u16(bytes)?;
        let descriptor = str_index(&constants, descriptor_idx as usize)?;
        let attrs_count = get_u16(bytes)?;
        let mut attributes = Vec::new();
        for _ in 0..attrs_count {
            attributes.push(get_attribute(&constants, bytes)?);
        }

        let code_attributes = attributes
            .iter()
            .filter(|attr| attr.name == "Code")
            .collect::<Vec<_>>();
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
                let code = parse_code_attribute(&constants, bytes)?;
                Some(code)
            }
            (true, [_]) => return Err(String::from("Method marked as native or abstract")),
            // (false, []) => return Err(String::from("Method must contain code")),
            (false, []) => None,
            _ => return Err(String::from("Method must only have one code attribute")),
        };
        methods.push(Method {
            access_flags,
            name,
            descriptor,
            attributes,
            code,
        })
    }

    let attribute_count = get_u16(bytes)?;
    let mut attributes = Vec::new();
    for _ in 0..attribute_count {
        attributes.push(get_attribute(&constants, bytes)?);
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
    constants: &[Constant],
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

fn parse_code_attribute(constants: &[Constant], bytes: Vec<u8>) -> Result<Code, String> {
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

    let stack_map_attrs = attributes
        .iter()
        .filter(|attr| attr.name == "StackMapTable")
        .collect::<Vec<_>>();
    let stack_map = match stack_map_attrs[..] {
        [attr] => {
            let mut stack_map = Vec::new();
            let mut bytes = attr.data.iter().copied();
            let frame_count = get_u16(&mut bytes)?;
            for _ in 0..frame_count {
                // stack_map.push(match bytes.next() {
                //     Some(offset_delta @ 0..=63) => {StackMapFrame::Same { offset_delta }}
                // })
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

fn str_index(constants: &[Constant], idx: usize) -> Result<String, String> {
    match constants.get(idx - 1) {
        Some(Constant::String(str)) => Ok(str.clone()),
        Some(other) => Err(format!("Expected a string; got `{other:?}`")),
        None => Err(String::from("Unexpected EOF")),
    }
}

fn class_index(constants: &[Constant], idx: usize) -> Result<String, String> {
    match constants.get(idx - 1) {
        Some(Constant::ClassRef { string_addr }) => str_index(constants, *string_addr as usize),
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
