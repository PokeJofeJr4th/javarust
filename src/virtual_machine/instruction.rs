use std::{iter::Peekable, rc::Rc};

use crate::class::{BootstrapMethod, Constant, FieldType, MethodDescriptor};

#[derive(Clone, Debug)]
pub enum Instruction {
    Noop,
    Push1(u32),
    Push2(u32, u32),
    LoadString(Rc<str>),
    Load1(usize),
    Load2(usize),
    Store1(usize),
    Store2(usize),
    Pop,
    Pop2,
    Dup,
    Dupx1,
    Dupx2,
    Dup2,
    Dup2x1,
    Dup2x2,
    Swap,
    IOp(Op),
    IInc(usize, i32),
    LOp(Op),
    FOp(Op),
    DOp(Op),
    Convert(Type, Type),
    LCmp,
    DCmp(bool),
    FCmp(bool),
    ICmp(Cmp, i16),
    IfCmp(Cmp, i16),
    Goto(i32),
    Return0,
    Return1,
    Return2,
    GetStatic(Rc<str>, Rc<str>, FieldType),
    GetField(Rc<str>, Rc<str>, FieldType),
    PutField(Rc<str>, Rc<str>, FieldType),
    InvokeVirtual(Rc<str>, Rc<str>, MethodDescriptor),
    InvokeSpecial(Rc<str>, Rc<str>, MethodDescriptor),
    InvokeStatic(Rc<str>, Rc<str>, MethodDescriptor),
    InvokeDynamic(u16, Rc<str>, MethodDescriptor),
    New(Rc<str>),
    IfNull(bool, i16),
}

impl Instruction {
    pub const fn push_2(bytes: u64) -> Self {
        Self::Push2((bytes >> 32) as u32, (bytes & u32::MAX as u64) as u32)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Shl,
    Shr,
    Ushr,
    And,
    Or,
    Xor,
}

#[derive(Clone, Copy, Debug)]
pub enum Type {
    Int,
    Long,
    Float,
    Double,
    Byte,
    Char,
    Short,
}

#[derive(Clone, Copy, Debug)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

pub fn hydrate_code(
    constants: &[Constant],
    code: Vec<u8>,
    verbose: bool,
) -> Result<Vec<Instruction>, String> {
    if verbose {
        for byte in &code {
            print!("{byte:<02X} ");
        }
        println!();
    }
    let mut bytes = code.into_iter().enumerate().peekable();
    let mut code = Vec::new();
    while let Some(&(index, _)) = bytes.peek() {
        code.push((index, parse_instruction(constants, &mut bytes)?));
    }
    if verbose {
        println!("{code:?}");
    }
    Ok(code
        .iter()
        .cloned()
        .map(|(idx, instr)| match instr {
            Instruction::Goto(goto) => {
                let target = (idx as i32).wrapping_add(goto) as usize;
                let goto = code.iter().position(|(idx, _)| *idx == target).unwrap();
                Instruction::Goto(goto as i32)
            }
            Instruction::IfCmp(cmp, goto) => {
                let target = (idx as i16).wrapping_add(goto) as usize;
                let goto = code.iter().position(|(idx, _)| *idx == target).unwrap();
                Instruction::IfCmp(cmp, goto as i16)
            }
            Instruction::IfNull(cmp, goto) => {
                let target = (idx as i16).wrapping_add(goto) as usize;
                let goto = code.iter().position(|(idx, _)| *idx == target).unwrap();
                Instruction::IfNull(cmp, goto as i16)
            }
            Instruction::ICmp(cmp, goto) => {
                let target = (idx as i16).wrapping_add(goto) as usize;
                let goto = code.iter().position(|(idx, _)| *idx == target).unwrap();
                Instruction::ICmp(cmp, goto as i16)
            }
            other => other,
        })
        .collect())
}

#[allow(clippy::too_many_lines)]
pub fn parse_instruction(
    constants: &[Constant],
    bytes: &mut Peekable<impl Iterator<Item = (usize, u8)>>,
) -> Result<Instruction, String> {
    match bytes.next().unwrap().1 {
        0x0 => Ok(Instruction::Noop),
        0x01 => {
            // aconst_null
            // push a null pointer onto the operand stack
            Ok(Instruction::Push1(u32::MAX))
        }
        iconst_i @ 0x02..=0x08 => {
            // iconst_<i>
            // push an integer constant onto the stack
            let iconst = iconst_i as i8 - 3;
            Ok(Instruction::Push1(iconst as i32 as u32))
        }
        lconst_l @ 0x09..=0x0A => {
            // lconst_<l>
            // push a long constant onto the stack
            let lconst = lconst_l - 0x09;
            let long = lconst as i64 as u64;
            Ok(Instruction::Push2((long >> 32) as u32, long as u32))
        }
        fconst_f @ 0x0B..=0x0D => {
            // fconst_<f>
            // push a float constant onto the stack
            let fconst = fconst_f - 0x0B;
            let float = fconst as f32;
            Ok(Instruction::Push1(float.to_bits()))
        }
        dconst_d @ (0x0E | 0x0F) => {
            // dconst_<d>
            // push a double constant onto the stack
            let dconst = dconst_d - 0xE;
            let double = (dconst as f64).to_bits();
            Ok(Instruction::Push2((double >> 32) as u32, double as u32))
        }
        0x10 => {
            // bipush byte
            // push a byte onto the operand stack
            let byte = bytes.next().unwrap().1 as i8;
            // I think this will sign-extend it, not entirely sure tho
            let value = byte as i32 as u32;
            Ok(Instruction::Push1(value))
        }
        0x11 => {
            // sipush b1 b2
            let upper = bytes.next().unwrap().1;
            let lower = bytes.next().unwrap().1;
            let short = u16::from_be_bytes([upper, lower]) as i16 as i32 as u32;
            Ok(Instruction::Push1(short))
        }
        0x12 => {
            // ldc
            // push item from constant pool

            let index = bytes.next().unwrap().1;

            let constant = constants[index as usize - 1].clone();

            match constant {
                Constant::Int(i) => Ok(Instruction::Push1(i as u32)),
                Constant::Float(i) => Ok(Instruction::Push1(i.to_bits())),
                Constant::String(str) | Constant::StringRef(str) => {
                    Ok(Instruction::LoadString(str))
                }
                other => Err(format!("Error during ldc; can't load {other:?}")),
            }
        }
        0x13 => {
            todo!("ldc_w")
        }
        0x14 => {
            // ldc2_w
            // load a 2-wide constant
            let upper = bytes.next().unwrap().1;
            let lower = bytes.next().unwrap().1;

            let index = ((upper as u16) << 8) | lower as u16;

            let constant = constants[index as usize - 1].clone();
            match constant {
                Constant::Double(d) => Ok(Instruction::push_2(d.to_bits())),
                Constant::Long(l) => Ok(Instruction::push_2(l as u64)),
                other => Err(format!("Error during ldc2_w; can't load {other:?}")),
            }
        }
        0x18 | 0x16 => {
            // dload|lload index
            // load a double from locals to stack
            let index = bytes.next().unwrap().1;
            Ok(Instruction::Load2(index as usize))
        }
        0x19 | 0x17 | 0x15 => {
            // aload|fload|iload index
            // load one item from locals to stack
            let index = bytes.next().unwrap().1;
            Ok(Instruction::Load1(index as usize))
        }
        iload_n @ 0x1A..=0x1D => {
            // iload_<n>
            // load one item from locals to stack
            let index = iload_n - 0x1A;
            Ok(Instruction::Load1(index as usize))
        }
        lload_n @ 0x1E..=0x21 => {
            // lload_<n>
            // load one item from locals to stack
            let index = lload_n - 0x1E;
            Ok(Instruction::Load2(index as usize))
        }
        fload_n @ 0x22..=0x25 => {
            // fload_<n>
            // load one item from locals to stack
            let index = fload_n - 0x22;
            Ok(Instruction::Load1(index as usize))
        }
        dload_n @ 0x26..=0x29 => {
            // dload_<n>
            // load two items from locals to stack
            let index = dload_n - 0x26;
            Ok(Instruction::Load2(index as usize))
        }
        aload_n @ 0x2A..=0x2D => {
            // aload_<n>
            // load one item from locals to stack
            let index = aload_n - 0x2A;
            Ok(Instruction::Load1(index as usize))
        }
        0x31 | 0x2F => {
            todo!("daload | laload")
        }
        0x32 | 0x30 | 0x2E => {
            todo!("aaload | faload | iaload")
        }
        0x33 => {
            todo!("baload")
        }
        0x34 => {
            todo!("caload")
        }
        0x35 => {
            todo!("saload")
        }
        0x39 | 0x37 => {
            // dstore|lstore index
            // put two values into a local
            let index = bytes.next().unwrap().1;
            Ok(Instruction::Store2(index as usize))
        }
        0x3A | 0x38 | 0x36 => {
            // astore|fstore|istore index
            // put one reference into a local
            let index = bytes.next().unwrap().1;
            Ok(Instruction::Store1(index as usize))
        }
        istore_n @ 0x3B..=0x3E => {
            // istore_<n>
            // store one item from stack into local
            let index = istore_n - 0x3B;
            Ok(Instruction::Store1(index as usize))
        }
        lstore_n @ 0x3F..=0x42 => {
            // lstore_<n>
            // store two items from stack into local
            let index = lstore_n - 0x3F;
            Ok(Instruction::Store2(index as usize))
        }
        fstore_n @ 0x43..=0x46 => {
            // fstore_<n>
            // store one item from stack into local
            let index = fstore_n - 0x43;
            Ok(Instruction::Store1(index as usize))
        }
        dstore_n @ 0x47..=0x4A => {
            // dstore_<n>
            // store two items from stack into locals
            let index = dstore_n - 0x47;
            Ok(Instruction::Store2(index as usize))
        }
        astore_n @ 0x4B..=0x4E => {
            // astore_<n>
            // store one item from stack into locals
            let index = astore_n - 0x4B;
            Ok(Instruction::Store1(index as usize))
        }
        0x52 | 0x50 => {
            todo!("dastore | lastore")
        }
        0x53 | 0x51 | 0x4F => {
            todo!("aastore | fastore | iastore")
        }
        0x54 => {
            todo!("bastore")
        }
        0x55 => {
            todo!("castore")
        }
        0x56 => {
            todo!("sastore")
        }
        0x57 => {
            // pop
            Ok(Instruction::Pop)
        }
        0x58 => {
            // pop2
            Ok(Instruction::Pop2)
        }
        0x59 => {
            // dup
            Ok(Instruction::Dup)
        }
        0x5A => {
            // dup_x1
            // xy => yxy
            Ok(Instruction::Dupx1)
        }
        0x5B => {
            // dup_x2
            // xyz => zxyz
            Ok(Instruction::Dupx2)
        }
        0x5C => {
            // dup2
            // xy => xyxy
            Ok(Instruction::Dup2)
        }
        0x5D => {
            // dup2_x1
            // xyz => yzxyz
            Ok(Instruction::Dup2x1)
        }
        0x5E => {
            // dup2_x2
            // wxyz => yzwxyz
            Ok(Instruction::Dup2x2)
        }
        0x5F => {
            // swap
            // swap two values
            Ok(Instruction::Swap)
        }
        0x60 => {
            // iadd
            // int add
            Ok(Instruction::IOp(Op::Add))
        }
        0x61 => {
            // ladd
            // long add
            Ok(Instruction::LOp(Op::Add))
        }
        0x62 => {
            // fadd
            // float add
            Ok(Instruction::FOp(Op::Add))
        }
        0x63 => {
            // dadd
            // double add
            Ok(Instruction::DOp(Op::Add))
        }
        0x64 => {
            // isub
            // int subtract
            Ok(Instruction::IOp(Op::Sub))
        }
        0x65 => {
            // lsub
            // long subtract
            Ok(Instruction::LOp(Op::Sub))
        }
        0x66 => {
            // fsub
            // float sub
            Ok(Instruction::FOp(Op::Sub))
        }
        0x67 => {
            // dsub
            // double subtraction
            Ok(Instruction::DOp(Op::Sub))
        }
        0x68 => {
            // imul
            // int multiply
            Ok(Instruction::IOp(Op::Mul))
        }
        0x69 => {
            // lmul
            // long multiply
            Ok(Instruction::LOp(Op::Mul))
        }
        0x6A => {
            // fmul
            // float mul
            Ok(Instruction::FOp(Op::Mul))
        }
        0x6B => {
            // dmul
            // double multiplication
            Ok(Instruction::DOp(Op::Mul))
        }
        0x6C => {
            // idiv
            // int divide
            Ok(Instruction::IOp(Op::Div))
        }
        0x6D => {
            // ldiv
            // long division
            Ok(Instruction::LOp(Op::Div))
        }
        0x6E => {
            // fdiv
            // float div

            Ok(Instruction::FOp(Op::Div))
        }
        0x6F => {
            // ddiv
            // double division
            Ok(Instruction::DOp(Op::Div))
        }
        0x70 => {
            // irem
            // int remainder
            Ok(Instruction::IOp(Op::Mod))
        }
        0x71 => {
            // lrem
            // long modulo
            Ok(Instruction::LOp(Op::Mod))
        }
        0x72 => {
            // frem
            // float rem
            Ok(Instruction::FOp(Op::Mod))
        }
        0x73 => {
            // drem
            // double remainder
            Ok(Instruction::DOp(Op::Mod))
        }
        0x74 => {
            // ineg
            // negate int
            Ok(Instruction::IOp(Op::Neg))
        }
        0x75 => {
            // lneg
            // negate long
            Ok(Instruction::LOp(Op::Neg))
        }
        0x76 => {
            // fneg
            // negate float
            Ok(Instruction::FOp(Op::Neg))
        }
        0x77 => {
            // dneg
            // negate double
            Ok(Instruction::DOp(Op::Neg))
        }
        0x78 => {
            // ishl
            // int shift left
            Ok(Instruction::IOp(Op::Shl))
        }
        0x79 => {
            // lshl
            // long shift left
            Ok(Instruction::LOp(Op::Shl))
        }
        0x7A => {
            // ishr
            // int shift right
            Ok(Instruction::IOp(Op::Shr))
        }
        0x7B => {
            // lshr
            // long shift right
            Ok(Instruction::LOp(Op::Shr))
        }
        0x7C => {
            // iushr
            // int logical shift right
            Ok(Instruction::IOp(Op::Ushr))
        }
        0x7D => {
            // lushr
            // long logical shift right
            Ok(Instruction::LOp(Op::Ushr))
        }
        0x7E => {
            // iand
            // int boolean and
            Ok(Instruction::IOp(Op::And))
        }
        0x7F => {
            // land
            // long boolean and
            Ok(Instruction::LOp(Op::And))
        }
        0x80 => {
            // ior
            // int boolean or
            Ok(Instruction::IOp(Op::Or))
        }
        0x81 => {
            // lor
            // long boolean or
            Ok(Instruction::LOp(Op::Or))
        }
        0x82 => {
            // ixor
            // int boolean xor
            Ok(Instruction::IOp(Op::Xor))
        }
        0x83 => {
            // lxor
            // long boolean xor
            Ok(Instruction::LOp(Op::Xor))
        }
        0x84 => {
            // iinc
            // int increment
            let index = bytes.next().unwrap().1 as usize;
            let inc = bytes.next().unwrap().1 as i8 as i32;
            Ok(Instruction::IInc(index, inc))
        }
        0x85 => {
            // i2l
            // int to long
            Ok(Instruction::Convert(Type::Int, Type::Long))
        }
        0x86 => {
            // i2f
            // int to float
            Ok(Instruction::Convert(Type::Int, Type::Float))
        }
        0x87 => {
            // i2d
            // int to double
            Ok(Instruction::Convert(Type::Int, Type::Double))
        }
        0x88 => {
            // l2i
            // long to int
            Ok(Instruction::Convert(Type::Long, Type::Int))
        }
        0x89 => {
            // l2f
            // long to float
            Ok(Instruction::Convert(Type::Long, Type::Float))
        }
        0x8A => {
            // l2d
            // long to double
            Ok(Instruction::Convert(Type::Long, Type::Double))
        }
        0x8B => {
            // f2i
            // float to integer
            Ok(Instruction::Convert(Type::Float, Type::Int))
        }
        0x8C => {
            // f2l
            // float to long
            Ok(Instruction::Convert(Type::Float, Type::Long))
        }
        0x8D => {
            // f2d
            // float to double
            Ok(Instruction::Convert(Type::Float, Type::Double))
        }
        0x8E => {
            // d2i
            // double to integer
            Ok(Instruction::Convert(Type::Double, Type::Int))
        }
        0x8F => {
            // d2l
            // double to long
            Ok(Instruction::Convert(Type::Double, Type::Long))
        }
        0x90 => {
            // d2f
            // double to float
            Ok(Instruction::Convert(Type::Double, Type::Float))
        }
        0x91 => {
            // i2b
            // int to byte
            Ok(Instruction::Convert(Type::Int, Type::Byte))
        }
        0x92 => {
            // i2c
            // int to char
            Ok(Instruction::Convert(Type::Int, Type::Char))
        }
        0x93 => {
            // i2s
            // int to short
            Ok(Instruction::Convert(Type::Int, Type::Short))
        }
        0x94 => {
            // lcmp
            // long comparison
            Ok(Instruction::LCmp)
        }
        fcmp_op @ 0x95..=0x96 => {
            // fcmp<op>
            // float comparison
            Ok(Instruction::FCmp(fcmp_op == 0x95))
        }
        dcmp_op @ 0x97..=0x98 => {
            // dcmp<op>
            // double comparison
            Ok(Instruction::DCmp(dcmp_op == 0x97))
        }
        if_cnd @ 0x99..=0x9E => {
            // if<cond>
            // integer comparison to zero
            let bb1 = bytes.next().unwrap().1;
            let bb2 = bytes.next().unwrap().1;
            let branch = u16::from_be_bytes([bb1, bb2]) as i16;

            let cond = match if_cnd {
                0x99 => Cmp::Eq,
                0x9A => Cmp::Ne,
                0x9B => Cmp::Lt,
                0x9C => Cmp::Ge,
                0x9D => Cmp::Gt,
                0x9E => Cmp::Le,
                _ => unreachable!(),
            };
            Ok(Instruction::IfCmp(cond, branch))
        }
        if_icmp @ 0x9F..=0xA4 => {
            // if_icmp<cond>
            // comparison between integers
            let bb1 = bytes.next().unwrap().1;
            let bb2 = bytes.next().unwrap().1;
            let branch = u16::from_be_bytes([bb1, bb2]) as i16;

            let cond = match if_icmp {
                0x9F => Cmp::Eq,
                0xA0 => Cmp::Ne,
                0xA1 => Cmp::Lt,
                0xA2 => Cmp::Ge,
                0xA3 => Cmp::Gt,
                0xA4 => Cmp::Le,
                _ => unreachable!(),
            };
            Ok(Instruction::ICmp(cond, branch))
        }
        _if_acmp @ 0xA5..=0xA6 => {
            todo!("if_acmp<cond>")
        }
        0xA7 => {
            // goto bb1 bb2
            let bb1 = bytes.next().unwrap().1;
            let bb2 = bytes.next().unwrap().1;
            let branchoffset = u16::from_be_bytes([bb1, bb2]) as i16 as i32;
            Ok(Instruction::Goto(branchoffset))
        }
        0xA8 => {
            todo!("jsr")
            // jump subroutine
        }
        0xA9 => {
            todo!("ret")
            // return from subroutine
        }
        0xAA => {
            todo!("tableswitch")
        }
        0xAB => {
            todo!("lookupswitch")
        }
        0xAC | 0xAE | 0xB0 => Ok(Instruction::Return1),
        0xAF | 0xAD => Ok(Instruction::Return2),
        0xB1 => {
            // return
            // return void
            Ok(Instruction::Return0)
        }
        0xB2 => {
            // getstatic
            // get a static field from a class
            let ib1 = bytes.next().unwrap().1;
            let ib2 = bytes.next().unwrap().1;
            let index = u16::from_be_bytes([ib1, ib2]);

            let Constant::FieldRef { class, name, field_type } = constants[index as usize - 1].clone() else {
                    return Err(format!("Error invoking GetStatic at index {index}; {:?}", constants[index as usize - 1]))
            };
            Ok(Instruction::GetStatic(class, name, field_type))
        }
        0xB3 => {
            todo!("putstatic")
            // set a static field in a class
        }
        0xB4 => {
            // getfield
            // get a field from an object
            let ib1 = bytes.next().unwrap().1;
            let ib2 = bytes.next().unwrap().1;
            let index = u16::from_be_bytes([ib1, ib2]);

            let Constant::FieldRef { class, name, field_type } = constants[index as usize - 1].clone() else {
                    return Err(format!("Error invoking PutField at index {index}; {:?}", constants[index as usize - 1]))
                };

            Ok(Instruction::GetField(class, name, field_type))
        }
        0xB5 => {
            // putfield
            // set a field in an object
            let ib1 = bytes.next().unwrap().1;
            let ib2 = bytes.next().unwrap().1;
            let index = u16::from_be_bytes([ib1, ib2]);

            let Constant::FieldRef { class, name, field_type } = constants[index as usize - 1].clone() else {
                    return Err(format!("Error invoking PutField at index {index}; {:?}", constants[index as usize - 1]))
                };

            Ok(Instruction::PutField(class, name, field_type))
        }
        0xB6 => {
            // invokevirtual
            // invoke a method virtually I guess

            let ib1 = bytes.next().unwrap().1;
            let ib2 = bytes.next().unwrap().1;
            let index = u16::from_be_bytes([ib1, ib2]);

            let Constant::MethodRef { class, name, method_type } = constants[index as usize - 1].clone() else {
                    return Err(String::from("Error during InvokeVirtual"))
                };

            Ok(Instruction::InvokeVirtual(class, name, method_type))
        }
        0xB7 => {
            // invokespecial
            // invoke an instance method
            let ib1 = bytes.next().unwrap().1;
            let ib2 = bytes.next().unwrap().1;
            let index = u16::from_be_bytes([ib1, ib2]);

            let Constant::MethodRef{name, class, method_type} = constants[index as usize - 1].clone() else {
                    todo!("Error during InvokeSpecial")
                };

            Ok(Instruction::InvokeSpecial(class, name, method_type))
        }
        0xB8 => {
            // invokestatic
            // make a static method
            let ib1 = bytes.next().unwrap().1;
            let ib2 = bytes.next().unwrap().1;
            let index = u16::from_be_bytes([ib1, ib2]);

            let Constant::MethodRef{name, class, method_type} = constants[index as usize - 1].clone() else {
                    todo!("Throw some sort of error")
                };
            Ok(Instruction::InvokeStatic(class, name, method_type))
        }
        0xB9 => {
            todo!("invokeinterface")
        }
        0xBA => {
            // invokedynamic
            // dynamically figure out what to do
            let ib1 = bytes.next().unwrap().1;
            let ib2 = bytes.next().unwrap().1;
            let index = u16::from_be_bytes([ib1, ib2]);

            let 0 = bytes.next().unwrap().1 else { return Err(String::from("Expected a zero"))};
            let 0 =  bytes.next().unwrap().1 else { return Err(String::from("Expected a zero"))};

            let Constant::InvokeDynamic { bootstrap_index, method_name, method_type } =
                    constants[index as usize - 1].clone() else {
                        return Err(format!("Error running InvokeDynamic - {:?}", constants[index as usize - 1]))
                    };

            Ok(Instruction::InvokeDynamic(
                bootstrap_index,
                method_name,
                method_type,
            ))
        }
        0xBB => {
            // new
            // make a new object instance
            let ib1 = bytes.next().unwrap().1;
            let ib2 = bytes.next().unwrap().1;
            let index = u16::from_be_bytes([ib1, ib2]);

            let Constant::ClassRef(class) = constants[index as usize - 1].clone() else {
                    todo!("Throw some sort of error")
                };

            Ok(Instruction::New(class))
        }
        0xBC => {
            todo!("newarray")
            // make a new array
        }
        0xBD => {
            todo!("anewarray")
        }
        0xBE => {
            todo!("arraylength")
        }
        0xBF => {
            todo!("athrow")
        }
        0xC0 => {
            todo!("checkcast")
        }
        0xC1 => {
            todo!("instanceof")
        }
        0xC2 => {
            todo!("monitorenter")
        }
        0xC3 => {
            todo!("monitorexit")
        }
        0xC4 => {
            todo!("wide")
            // woosh this one's gonna be tough
        }
        0xC5 => {
            todo!("multianewarray")
            // make a new multi-dimensional array
        }
        if_null @ 0xC6..=0xC7 => {
            // ifnull | ifnonnull
            let bb1 = bytes.next().unwrap().1;
            let bb2 = bytes.next().unwrap().1;
            let branch = u16::from_be_bytes([bb1, bb2]) as i16;

            Ok(Instruction::IfNull(if_null == 0xC6, branch))
        }
        0xC8 => {
            // goto_w bb1 bb2 bb3 bb4
            let bb1 = bytes.next().unwrap().1;
            let bb2 = bytes.next().unwrap().1;
            let bb3 = bytes.next().unwrap().1;
            let bb4 = bytes.next().unwrap().1;
            let branchoffset = u32::from_be_bytes([bb1, bb2, bb3, bb4]) as i32;
            Ok(Instruction::Goto(branchoffset))
        }
        0xC9 => {
            todo!("jsr_w")
            // jump subroutine wide
        }
        other => Err(format!("Invalid Opcode: 0x{other:x}")),
    }
}
