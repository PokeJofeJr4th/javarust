// the heap will be an uncollected set of Rc<Mutex<>>es

use std::{cell::RefCell, cmp::Ordering, rc::Rc};

use crate::class::{AccessFlags, Class, ClassVersion, Constant, Method, MethodDescriptor};

struct Thread {
    pc_register: usize,
    stack: Vec<Rc<RefCell<StackFrame>>>,
    method_area: Rc<Vec<(Rc<Class>, Rc<Method>)>>,
    heap: Rc<RefCell<Vec<Rc<RefCell<Object>>>>>,
}

impl Thread {
    pub fn tick(&mut self) -> Result<(), String> {
        // this way we can mutate the stack frame without angering the borrow checker
        let stackframe = self.stack.last().unwrap().clone();
        if stackframe.borrow().method.access_flags.is_native() {
            return self.invoke_native(stackframe);
        }
        let opcode = self.get_pc_byte(stackframe.clone());
        match opcode {
            0x0 => {
                // nop
            }
            0x01 => {
                // aconst_null
                // push a null pointer onto the operand stack
                stackframe.borrow_mut().operand_stack.push(u32::MAX);
            }
            iconst_i @ 0x02..=0x08 => {
                // iconst_<i>
                // push an integer constant onto the stack
                let iconst = iconst_i as i8 - 3;
                stackframe
                    .borrow_mut()
                    .operand_stack
                    .push(iconst as i32 as u32);
            }
            lconst_l @ 0x09..=0x0A => {
                // lconst_<l>
                // push a long constant onto the stack
                let lconst = lconst_l - 0x09;
                let long = lconst as i64;
                push_long(&mut stackframe.borrow_mut().operand_stack, long as u64);
            }
            fconst_f @ 0x0B..=0x0D => {
                // fconst_<f>
                // push a float constant onto the stack
                let fconst = fconst_f - 0x0B;
                let float = fconst as f32;
                stackframe.borrow_mut().operand_stack.push(float.to_bits());
            }
            dconst_d @ (0x0E | 0x0F) => {
                // dconst_<d>
                // push a double constant onto the stack
                let dconst = dconst_d - 0xE;
                let double = dconst as f64;
                push_long(&mut stackframe.borrow_mut().operand_stack, double.to_bits());
            }
            0x10 => {
                // bipush byte
                // push a byte onto the operand stack
                let byte = self.get_pc_byte(stackframe.clone()) as i8;
                // I think this will sign-extend it, not entirely sure tho
                let value = byte as i32 as u32;
                stackframe.borrow_mut().operand_stack.push(value);
            }
            0x11 => {
                // sipush b1 b2
                let upper = self.get_pc_byte(stackframe.clone());
                let lower = self.get_pc_byte(stackframe.clone());
                let short = u16::from_be_bytes([upper, lower]) as i16 as i32;
                stackframe.borrow_mut().operand_stack.push(short as u32);
            }
            0x12 => {
                todo!("ldc")
            }
            0x13 => {
                todo!("ldc_w")
            }
            0x14 => {
                todo!("ldc2_w")
            }
            0x18 | 0x16 => {
                // dload|lload index
                // load a double from locals to stack
                let index = self.get_pc_byte(stackframe.clone());
                long_load(stackframe, index as usize);
            }
            0x19 | 0x17 | 0x15 => {
                // aload|fload|iload index
                // load one item from locals to stack
                let index = self.get_pc_byte(stackframe.clone());
                value_load(stackframe, index as usize);
            }
            iload_n @ 0x1A..=0x1D => {
                // iload_<n>
                // load one item from locals to stack
                let index = iload_n - 0x1A;
                value_load(stackframe, index as usize);
            }
            lload_n @ 0x1E..=0x21 => {
                // lload_<n>
                // load one item from locals to stack
                let index = lload_n - 0x1E;
                long_load(stackframe, index as usize);
            }
            fload_n @ 0x22..=0x25 => {
                // fload_<n>
                // load one item from locals to stack
                let index = fload_n - 0x22;
                value_load(stackframe, index as usize);
            }
            dload_n @ 0x26..=0x29 => {
                // dload_<n>
                // load two items from locals to stack
                let index = dload_n - 0x26;
                long_load(stackframe, index as usize);
            }
            aload_n @ 0x2A..=0x2D => {
                // aload_<n>
                // load one item from locals to stack
                let index = aload_n - 0x29;
                value_load(stackframe, index as usize);
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
                let index = self.get_pc_byte(stackframe.clone());
                long_store(stackframe, index as usize);
            }
            0x3A | 0x38 | 0x36 => {
                // astore|fstore|istore index
                // put one reference into a local
                let index = self.get_pc_byte(stackframe.clone());
                value_store(stackframe, index as usize);
            }
            istore_n @ 0x3B..=0x3E => {
                // istore_<n>
                // store one item from stack into local
                let index = istore_n - 0x3B;
                value_store(stackframe, index as usize);
            }
            lstore_n @ 0x3F..=0x42 => {
                // lstore_<n>
                // store two items from stack into local
                let index = lstore_n - 0x3F;
                long_store(stackframe, index as usize);
            }
            fstore_n @ 0x43..=0x46 => {
                // fstore_<n>
                // store one item from stack into local
                let index = fstore_n - 0x43;
                value_store(stackframe, index as usize);
            }
            dstore_n @ 0x47..=0x4A => {
                // dstore_<n>
                // store two items from stack into locals
                let index = dstore_n - 0x47;
                long_store(stackframe, index as usize);
            }
            astore_n @ 0x4B..=0x4E => {
                // astore_<n>
                // store one item from stack into locals
                let index = astore_n - 0x4A;
                value_store(stackframe, index as usize);
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
                stackframe.borrow_mut().operand_stack.pop();
            }
            0x58 => {
                // pop2

                stackframe.borrow_mut().operand_stack.pop();
                stackframe.borrow_mut().operand_stack.pop();
            }
            0x59 => {
                // dup
                let value = *stackframe.borrow().operand_stack.last().unwrap();
                stackframe.borrow_mut().operand_stack.push(value);
            }
            0x5A => {
                // dup_x1
                // xy => yxy
                let y = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let x = stackframe.borrow_mut().operand_stack.pop().unwrap();
                stackframe.borrow_mut().operand_stack.extend([y, x, y]);
            }
            0x5B => {
                // dup_x1
                // xyz => zxyz
                let z = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let y = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let x = stackframe.borrow_mut().operand_stack.pop().unwrap();
                stackframe.borrow_mut().operand_stack.extend([z, x, y, z]);
            }
            0x5C => {
                // dup2
                // xy => xyxy
                let y = *stackframe.borrow().operand_stack.last().unwrap();
                let x = *stackframe.borrow().operand_stack.last().unwrap();
                stackframe.borrow_mut().operand_stack.extend([x, y]);
            }
            0x5D => {
                // dup2_x1
                // xyz => yzxyz
                let z = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let y = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let x = stackframe.borrow_mut().operand_stack.pop().unwrap();
                stackframe
                    .borrow_mut()
                    .operand_stack
                    .extend([y, z, x, y, z]);
            }
            0x5E => {
                // dup2_x2
                // wxyz => yzwxyz
                let z = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let y = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let x = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let w = stackframe.borrow_mut().operand_stack.pop().unwrap();
                stackframe
                    .borrow_mut()
                    .operand_stack
                    .extend([y, z, w, x, y, z]);
            }
            0x5F => {
                // swap
                // swap two values
                let x = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let y = stackframe.borrow_mut().operand_stack.pop().unwrap();
                stackframe.borrow_mut().operand_stack.push(x);
                stackframe.borrow_mut().operand_stack.push(y);
            }
            0x60 => {
                // iadd
                // int add
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs.wrapping_add(rhs);
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            0x61 => {
                // ladd
                // long add
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs.wrapping_add(rhs);
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            0x62 => {
                // fadd
                // float add
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs + rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            0x63 => {
                // dadd
                // double add
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let sum = rhs + lhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, sum.to_bits());
            }
            0x64 => {
                // isub
                // int subtract
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs.wrapping_sub(rhs);
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            0x65 => {
                // lsub
                // long subtract
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs.wrapping_sub(rhs);
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            0x66 => {
                // fsub
                // float sub
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs - rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            0x67 => {
                // dsub
                // double subtraction
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = lhs - rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            0x68 => {
                // imul
                // int multiply
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs.wrapping_mul(rhs);
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            0x69 => {
                // lmul
                // long multiply
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs.wrapping_mul(rhs);
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            0x6A => {
                // fmul
                // float mul
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs * rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            0x6B => {
                // dmul
                // double multiplication
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = lhs * rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            0x6C => {
                // idiv
                // int divide
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                // TODO: Check for zero division
                let result = lhs / rhs;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            0x6D => {
                // ldiv
                // long division

                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                // TODO: Check for zero division
                let result = lhs / rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            0x6E => {
                // fdiv
                // float div
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs / rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            0x6F => {
                // ddiv
                // double division
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = lhs / rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            0x70 => {
                // irem
                // int remainder
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                // TODO: Check for zero division
                let result = lhs % rhs;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            0x71 => {
                // lrem
                // long modulo

                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                // TODO: Check for zero division
                let result = lhs % rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            0x72 => {
                // frem
                // float rem
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs % rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            0x73 => {
                // drem
                // double remainder
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = lhs % rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            0x74 => {
                // ineg
                // negate int
                let f = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let result = -f;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            0x75 => {
                // lneg
                // negate long
                let l = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = -l;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            0x76 => {
                // fneg
                // negate float
                let f = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = -f;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            0x77 => {
                // dneg
                // negate double
                let d =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = -d;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            0x78 => {
                // ishl
                // int shift left
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) & 0x1F;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs << rhs;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            0x79 => {
                // lshl
                // long shift left
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap() & 0x3F;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs << rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            0x7A => {
                // ishr
                // int shift right
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) & 0x1F;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs >> rhs;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            0x7B => {
                // lshr
                // long shift right
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap() & 0x3F;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs >> rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            0x7C => {
                // iushr
                // int logical shift right
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) & 0x1F;
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let result = lhs >> rhs;
                stackframe.borrow_mut().operand_stack.push(result);
            }
            0x7D => {
                // lushr
                // long logical shift right
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap() & 0x3F;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let result = lhs >> rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result);
            }
            0x7E => {
                // iand
                // int boolean and
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let result = lhs & rhs;
                stackframe.borrow_mut().operand_stack.push(result);
            }
            0x7F => {
                // land
                // long boolean and
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let result = lhs & rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result);
            }
            0x80 => {
                // ior
                // int boolean or
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let result = lhs | rhs;
                stackframe.borrow_mut().operand_stack.push(result);
            }
            0x81 => {
                // lor
                // long boolean or
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let result = lhs | rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result);
            }
            0x82 => {
                // ixor
                // int boolean xor
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let result = lhs ^ rhs;
                stackframe.borrow_mut().operand_stack.push(result);
            }
            0x83 => {
                // lxor
                // long boolean xor
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let result = lhs ^ rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result);
            }
            0x84 => {
                // iinc
                // int increment
                let index = self.get_pc_byte(stackframe.clone());
                let inc = self.get_pc_byte(stackframe.clone()) as i32;
                let start = stackframe.borrow().locals[index as usize] as i32;
                stackframe.borrow_mut().locals[index as usize] = start.wrapping_add(inc) as u32;
            }
            0x85 => {
                // i2l
                // int to long
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let long = int as i64;
                push_long(&mut stackframe.borrow_mut().operand_stack, long as u64);
            }
            0x86 => {
                // i2f
                // int to float
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let float = int as f32;
                stackframe.borrow_mut().operand_stack.push(float.to_bits());
            }
            0x87 => {
                // i2d
                // int to double
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let double = int as f64;
                push_long(&mut stackframe.borrow_mut().operand_stack, double.to_bits());
            }
            0x88 => {
                // l2i
                // long to int
                let long = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let int = long as u32;
                stackframe.borrow_mut().operand_stack.push(int);
            }
            0x89 => {
                // l2f
                // long to float
                let long = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let float = long as f32;
                stackframe.borrow_mut().operand_stack.push(float.to_bits());
            }
            0x8A => {
                // l2d
                // long to double
                let long = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let double = long as f64;
                push_long(&mut stackframe.borrow_mut().operand_stack, double.to_bits());
            }
            0x8B => {
                // f2i
                // float to integer
                let float = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let int = float as i32;
                stackframe.borrow_mut().operand_stack.push(int as u32);
            }
            0x8C => {
                // f2l
                // float to long
                let float = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let long = float as u64;
                push_long(&mut stackframe.borrow_mut().operand_stack, long);
            }
            0x8D => {
                // f2d
                // float to double
                let float = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let double = float as f64;
                push_long(&mut stackframe.borrow_mut().operand_stack, double.to_bits());
            }
            0x8E => {
                // d2i
                // double to integer
                let double =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let int = double as u32;
                stackframe.borrow_mut().operand_stack.push(int);
            }
            0x8F => {
                // d2l
                // double to long
                let double =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let int = double as u64;
                push_long(&mut stackframe.borrow_mut().operand_stack, int);
            }
            0x90 => {
                // d2f
                // double to float
                let double =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let float = (double as f32).to_bits();
                stackframe.borrow_mut().operand_stack.push(float);
            }
            0x91 => {
                // i2b
                // int to byte
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let byte = int as i8 as i32;
                stackframe.borrow_mut().operand_stack.push(byte as u32);
            }
            0x92 => {
                // i2c
                // int to char
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let char = int as u8;
                stackframe.borrow_mut().operand_stack.push(char as u32);
            }
            0x93 => {
                // i2s
                // int to short
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let short = int as i16 as i32;
                stackframe.borrow_mut().operand_stack.push(short as u32);
            }
            0x94 => {
                // lcmp
                // long comparison
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let value = match lhs.cmp(&rhs) {
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                    Ordering::Less => -1,
                };
                stackframe.borrow_mut().operand_stack.push(value as u32);
            }
            fcmp_op @ 0x95..=0x96 => {
                // fcmp<op>
                // float comparison
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let value = if lhs > rhs {
                    1
                } else if lhs == rhs {
                    0
                } else if lhs < rhs || fcmp_op == 0x95 {
                    -1
                } else {
                    1
                } as u32;
                stackframe.borrow_mut().operand_stack.push(value);
            }
            dcmp_op @ 0x97..=0x98 => {
                // dcmp<op>
                // double comparison
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let value = if lhs > rhs {
                    1
                } else if lhs == rhs {
                    0
                } else if lhs < rhs || dcmp_op == 0x97 {
                    -1
                } else {
                    1
                } as u32;
                stackframe.borrow_mut().operand_stack.push(value);
            }
            if_cnd @ 0x99..=0x9E => {
                // if<cond>
                // integer comparison to zero
                let bb1 = self.get_pc_byte(stackframe.clone());
                let bb2 = self.get_pc_byte(stackframe.clone());
                let branch = u16::from_be_bytes([bb1, bb2]);

                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let cond = match if_cnd {
                    0x99 => lhs == 0,
                    0x9A => lhs != 0,
                    0x9B => lhs < 0,
                    0x9C => lhs <= 0,
                    0x9D => lhs > 0,
                    0x9E => lhs >= 0,
                    _ => unreachable!(),
                };
                if cond {
                    self.pc_register = branch as usize;
                }
            }
            if_icmp @ 0x9F..=0xA4 => {
                // if_icmp<cond>
                // comparison between integers
                let bb1 = self.get_pc_byte(stackframe.clone());
                let bb2 = self.get_pc_byte(stackframe.clone());
                let branch = u16::from_be_bytes([bb1, bb2]);

                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let cond = match if_icmp {
                    0x9F => lhs == rhs,
                    0xA0 => lhs != rhs,
                    0xA1 => lhs < rhs,
                    0xA2 => lhs <= rhs,
                    0xA3 => lhs > rhs,
                    0xA4 => lhs >= rhs,
                    _ => unreachable!(),
                };
                if cond {
                    self.pc_register = branch as usize;
                }
            }
            if_acmp @ 0xA5..=0xA6 => {
                todo!("if_acmp<cond>")
            }
            0xA7 => {
                // goto bb1 bb2
                let bb1 = self.get_pc_byte(stackframe.clone());
                let bb2 = self.get_pc_byte(stackframe);
                let branchoffset = u16::from_be_bytes([bb1, bb2]);
                self.pc_register = branchoffset as usize;
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
            0xAC => {
                todo!("ireturn")
            }
            0xAE => {
                todo!("freturn")
            }
            0xAF | 0xAD => {
                todo!("dreturn|lreturn")
            }
            0xB0 => {
                todo!("areturn")
            }
            0xB1 => {
                todo!("return")
                // return void
            }
            0xB2 => {
                todo!("getstatic")
            }
            0xB3 => {
                todo!("putstatic")
                // set a static field in a class
            }
            0xB4 => {
                todo!("getfield")
            }
            0xB5 => {
                todo!("putfield")
                // set a field in an object
            }
            0xB6 => {
                todo!("invokevirtual")
            }
            0xB7 => {
                // invokespecial
                // invoke an instance method
                let ib1 = self.get_pc_byte(stackframe.clone());
                let ib2 = self.get_pc_byte(stackframe.clone());
                let index = u16::from_be_bytes([ib1, ib2]);

                let Constant::MethodRef{name, class, method_type} = stackframe.borrow().class.constants[index as usize - 1].clone() else {
                    todo!("Error during InvokeSpecial")
                };

                let (class_ref, method_ref) = search_method_area(
                    &self.method_area,
                    class.clone(),
                    name.clone(),
                    &method_type,
                )
                .ok_or_else(|| format!("Error during InvokeSpecial; {}.{}", class, name))?;
                let args_start =
                    stackframe.borrow().operand_stack.len() - method_type.parameter_size;
                let stack = &mut stackframe.borrow_mut().operand_stack;
                let mut stack_iter = core::mem::take(stack).into_iter();
                stack.extend((&mut stack_iter).take(args_start));
                stack.push(self.pc_register as u32);

                let args = stack_iter.collect::<Vec<_>>();

                println!("Invoking Method {} on {}", method_ref.name, class_ref.this);
                self.invoke_method(method_ref, class_ref);
                self.pc_register = 0;

                let new_stackframe = self.stack.last().unwrap().clone();

                new_stackframe.borrow_mut().operand_stack.extend(args);
            }
            0xB8 => {
                todo!("invokestatic")
            }
            0xB9 => {
                todo!("invokeinterface")
            }
            0xBA => {
                todo!("invokedynamic")
            }
            0xBB => {
                // new
                // make a new object instance
                let ib1 = self.get_pc_byte(stackframe.clone());
                let ib2 = self.get_pc_byte(stackframe.clone());
                let index = u16::from_be_bytes([ib1, ib2]);

                let Constant::ClassRef(class) = stackframe.borrow().class.constants[index as usize - 1].clone() else {
                    todo!("Throw some sort of error")
                };

                let new_object = Object {
                    object_type: class.clone(),
                };

                let length = self.heap.borrow().len();

                self.heap
                    .borrow_mut()
                    .push(Rc::new(RefCell::new(new_object)));

                let objectref = length as u32;
                stackframe.borrow_mut().operand_stack.push(objectref);
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
                let bb1 = self.get_pc_byte(stackframe.clone());
                let bb2 = self.get_pc_byte(stackframe.clone());
                let branch = u16::from_be_bytes([bb1, bb2]);

                let ptr = stackframe.borrow_mut().operand_stack.pop().unwrap();
                if (ptr == u32::MAX) ^ (if_null == 0xC7) {
                    self.pc_register = branch as usize;
                }
            }
            0xC8 => {
                // goto_w bb1 bb2 bb3 bb4
                let bb1 = self.get_pc_byte(stackframe.clone());
                let bb2 = self.get_pc_byte(stackframe.clone());
                let bb3 = self.get_pc_byte(stackframe.clone());
                let bb4 = self.get_pc_byte(stackframe);
                let branchoffset = u32::from_be_bytes([bb1, bb2, bb3, bb4]);
                self.pc_register = branchoffset as usize;
            }
            0xC9 => {
                todo!("jsr_w")
                // jump subroutine wide
            }
            other => return Err(format!("Invalid Opcode: 0x{other:x}")),
        }
        Ok(())
    }

    fn get_code(&self, stackframe: Rc<RefCell<StackFrame>>, idx: usize) -> u8 {
        stackframe.borrow().method.code.as_ref().unwrap().code[idx]
    }

    fn get_pc_byte(&mut self, stackframe: Rc<RefCell<StackFrame>>) -> u8 {
        let b = self.get_code(stackframe, self.pc_register);
        self.pc_register += 1;
        b
    }

    fn invoke_method(&mut self, method: Rc<Method>, class: Rc<Class>) {
        let stackframe = StackFrame::from_method(method, class);
        self.stack.push(Rc::new(RefCell::new(stackframe)));
    }

    fn invoke_native(&mut self, stackframe: Rc<RefCell<StackFrame>>) -> Result<(), String> {
        let name = stackframe.borrow().method.name.clone();
        let class = stackframe.borrow().class.this.clone();
        match (&*class, &*name) {
            ("java/lang/Object", "<init>") => self.return_void(),
            (class, name) => return Err(format!("Error invoking native method; {class}.{name}")),
        }
        Ok(())
    }

    fn return_void(&mut self) {
        self.stack.pop();
        if self.stack.is_empty() {
            return;
        }
        let stackframe = self.stack.last().unwrap();
        let return_address = stackframe.borrow_mut().operand_stack.pop().unwrap();
        self.pc_register = return_address as usize;
    }
}

fn pop_long(stack: &mut Vec<u32>) -> Option<u64> {
    let lower = stack.pop()?;
    let upper = stack.pop()?;
    Some((upper as u64) << 32 | lower as u64)
}

fn push_long(stack: &mut Vec<u32>, l: u64) {
    let lower = (l & 0x0000_FFFF) as u32;
    let upper = (l >> 16) as u32;
    stack.push(upper);
    stack.push(lower);
}

fn value_store(stackframe: Rc<RefCell<StackFrame>>, index: usize) {
    let value = stackframe.borrow_mut().operand_stack.pop().unwrap();
    stackframe.borrow_mut().locals[index] = value;
}

fn value_load(stackframe: Rc<RefCell<StackFrame>>, index: usize) {
    let value = stackframe.borrow().locals[index];
    stackframe.borrow_mut().operand_stack.push(value);
}

fn long_store(stackframe: Rc<RefCell<StackFrame>>, index: usize) {
    let lower = stackframe.borrow_mut().operand_stack.pop().unwrap();
    let upper = stackframe.borrow_mut().operand_stack.pop().unwrap();
    stackframe.borrow_mut().locals[index] = upper;
    stackframe.borrow_mut().locals[index + 1] = lower;
}

fn long_load(stackframe: Rc<RefCell<StackFrame>>, index: usize) {
    let value_upper = stackframe.borrow().locals[index];
    let value_lower = stackframe.borrow().locals[index + 1];
    stackframe
        .borrow_mut()
        .operand_stack
        .extend([value_upper, value_lower]);
}

fn search_method_area(
    method_area: &[(Rc<Class>, Rc<Method>)],
    class: Rc<str>,
    method: Rc<str>,
    method_type: &MethodDescriptor,
) -> Option<(Rc<Class>, Rc<Method>)> {
    for (possible_class, possible_method) in method_area {
        if possible_class.this == class
            && possible_method.name == method
            && &possible_method.descriptor == method_type
        {
            return Some((possible_class.clone(), possible_method.clone()));
        }
    }
    None
}

struct StackFrame {
    locals: Vec<u32>,
    operand_stack: Vec<u32>,
    method: Rc<Method>,
    class: Rc<Class>,
}

impl StackFrame {
    pub fn from_method(method: Rc<Method>, class: Rc<Class>) -> Self {
        let locals = match method.code.as_ref() {
            Some(code) => code.max_locals,
            _ => 0,
        };
        Self {
            locals: (0..=locals).map(|_| 0).collect(),
            operand_stack: Vec::new(),
            class,
            method,
        }
    }
}

struct Object {
    object_type: Rc<str>,
}

pub fn start_vm(src: Class) {
    let class = Rc::new(src);
    let mut method_area = class
        .methods
        .iter()
        .cloned()
        .map(|method| (class.clone(), method))
        .collect::<Vec<_>>();
    add_native_methods(&mut method_area);
    let heap = Rc::new(RefCell::new(Vec::new()));
    let mut method = None;
    for methods in &class.methods {
        if &*methods.name == "main" {
            method = Some(methods.clone());
            break;
        }
    }
    let method = method.expect("No `Main` function found");
    let mut primary_thread = Thread {
        pc_register: 0,
        stack: Vec::new(),
        method_area: Rc::new(method_area),
        heap,
    };
    primary_thread.invoke_method(method, class);
    loop {
        println!("{}", primary_thread.pc_register);
        primary_thread.tick().unwrap();
    }
}

fn add_native_methods(method_area: &mut Vec<(Rc<Class>, Rc<Method>)>) {
    let init = Rc::new(Method {
        access_flags: AccessFlags(AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC),
        name: "<init>".into(),
        descriptor: MethodDescriptor {
            parameter_size: 0,
            parameters: Vec::new(),
            return_type: None,
        },
        attributes: Vec::new(),
        code: None,
    });

    let object = Rc::new(Class {
        version: ClassVersion {
            minor_version: 0,
            major_version: 0,
        },
        constants: Vec::new(),
        access: AccessFlags(AccessFlags::ACC_NATIVE | AccessFlags::ACC_PUBLIC),
        this: "java/lang/Object".into(),
        super_class: "java/lang/Object".into(),
        interfaces: Vec::new(),
        fields: Vec::new(),
        methods: vec![init.clone()],
        attributes: Vec::new(),
    });

    method_area.extend([(object, init)]);
}
