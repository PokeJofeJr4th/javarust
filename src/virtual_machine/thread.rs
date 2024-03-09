use std::{
    cmp::Ordering,
    fmt::Write,
    sync::{Arc, Mutex},
};

use crate::{
    class::{BootstrapMethod, Class, Constant, FieldType, Method, MethodDescriptor, MethodHandle},
    data::{Heap, SharedClassArea, SharedHeap, SharedMethodArea, NULL},
};

use super::{
    instruction::Type,
    object::{AnyObj, Array1, Array2, ArrayType, Object, ObjectFinder, StringObj},
    Cmp, Instruction, Op, StackFrame,
};

pub struct Thread {
    pub pc_register: usize,
    pub stack: Vec<Arc<Mutex<StackFrame>>>,
    pub method_area: SharedMethodArea,
    pub class_area: SharedClassArea,
    pub heap: SharedHeap,
}

impl Thread {
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    /// # Panics
    /// # Errors
    pub fn tick(&mut self, verbose: bool) -> Result<(), String> {
        // this way we can mutate the stack frame without angering the borrow checker
        let stackframe = self.stack.last().unwrap().clone();
        let method = stackframe.lock().unwrap().method.clone();
        if let Some(native_method) = method.code.as_native() {
            // return self.invoke_native(&stackframe, verbose);
            return native_method.run(self, &stackframe, verbose);
        }
        let opcode = self.get_pc_byte(&stackframe);
        if verbose {
            println!("{opcode:?}");
        }
        match opcode {
            Instruction::Noop => {
                // nope
            }
            Instruction::Push1(i) => {
                // push one item onto the operand stack
                stackframe.lock().unwrap().operand_stack.push(i);
            }
            Instruction::Push2(a, b) => {
                // push 2 items onto the operand stack
                let operand_stack = &mut stackframe.lock().unwrap().operand_stack;
                operand_stack.push(a);
                operand_stack.push(b);
            }
            Instruction::LoadString(str) => {
                let str_ptr = self.heap.lock().unwrap().allocate_str(str);
                // never forget a static string
                self.rember(str_ptr, verbose);
                stackframe.lock().unwrap().operand_stack.push(str_ptr);
            }
            Instruction::Load2(index) => {
                // load a double from locals to stack
                long_load(&stackframe, index);
            }
            Instruction::Load1(index) => {
                // load one item from locals to stack
                value_load(&stackframe, index);
                if verbose {
                    println!("stack {:?}", stackframe.lock().unwrap().operand_stack);
                }
            }
            Instruction::Store2(index) => {
                // put two values into a local
                long_store(&stackframe, index);
            }
            Instruction::Store1(index) => {
                // put one reference into a local
                value_store(&stackframe, index);
                if verbose {
                    println!("locals {:?}", stackframe.lock().unwrap().locals);
                }
            }
            Instruction::Pop => {
                stackframe.lock().unwrap().operand_stack.pop();
            }
            Instruction::Pop2 => {
                stackframe.lock().unwrap().operand_stack.pop();
                stackframe.lock().unwrap().operand_stack.pop();
            }
            Instruction::Dup => {
                // dup
                let value = *stackframe.lock().unwrap().operand_stack.last().unwrap();
                stackframe.lock().unwrap().operand_stack.push(value);
            }
            Instruction::Dupx1 => {
                // xy => yxy
                let y = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let x = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                stackframe.lock().unwrap().operand_stack.extend([y, x, y]);
            }
            Instruction::Dupx2 => {
                // xyz => zxyz
                let z = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let y = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let x = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .extend([z, x, y, z]);
            }
            Instruction::Dup2 => {
                // xy => xyxy
                let y = *stackframe.lock().unwrap().operand_stack.last().unwrap();
                let x = *stackframe.lock().unwrap().operand_stack.last().unwrap();
                stackframe.lock().unwrap().operand_stack.extend([x, y]);
            }
            Instruction::Dup2x1 => {
                // xyz => yzxyz
                let z = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let y = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let x = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .extend([y, z, x, y, z]);
            }
            Instruction::Dup2x2 => {
                // wxyz => yzwxyz
                let z = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let y = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let x = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let w = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .extend([y, z, w, x, y, z]);
            }
            Instruction::Swap => {
                let x = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let y = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                stackframe.lock().unwrap().operand_stack.push(x);
                stackframe.lock().unwrap().operand_stack.push(y);
            }
            Instruction::IOp(Op::Add) => {
                // iadd
                // int add
                let rhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let result = lhs.wrapping_add(rhs);
                stackframe.lock().unwrap().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Add) => {
                // ladd
                // long add
                let rhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let result = lhs.wrapping_add(rhs);
                push_long(&mut stackframe.lock().unwrap().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Add) => {
                // fadd
                // float add
                let rhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let result = lhs + rhs;
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .push(result.to_bits());
            }
            Instruction::DOp(Op::Add) => {
                // dadd
                // double add
                let rhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let lhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let sum = rhs + lhs;
                push_long(&mut stackframe.lock().unwrap().operand_stack, sum.to_bits());
            }
            Instruction::IOp(Op::Sub) => {
                // isub
                // int subtract
                let rhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let result = lhs.wrapping_sub(rhs);
                stackframe.lock().unwrap().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Sub) => {
                // lsub
                // long subtract
                let rhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let result = lhs.wrapping_sub(rhs);
                push_long(&mut stackframe.lock().unwrap().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Sub) => {
                // fsub
                // float sub
                let rhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let result = lhs - rhs;
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .push(result.to_bits());
            }
            Instruction::DOp(Op::Sub) => {
                // dsub
                // double subtraction
                let rhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let lhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let result = lhs - rhs;
                push_long(
                    &mut stackframe.lock().unwrap().operand_stack,
                    result.to_bits(),
                );
            }
            Instruction::IOp(Op::Mul) => {
                // imul
                // int multiply
                let rhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let result = lhs.wrapping_mul(rhs);
                stackframe.lock().unwrap().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Mul) => {
                // lmul
                // long multiply
                let rhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let result = lhs.wrapping_mul(rhs);
                push_long(&mut stackframe.lock().unwrap().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Mul) => {
                // fmul
                // float mul
                let rhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let result = lhs * rhs;
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .push(result.to_bits());
            }
            Instruction::DOp(Op::Mul) => {
                // dmul
                // double multiplication
                let rhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let lhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let result = lhs * rhs;
                push_long(
                    &mut stackframe.lock().unwrap().operand_stack,
                    result.to_bits(),
                );
            }
            Instruction::IOp(Op::Div) => {
                // idiv
                // int divide
                let rhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                // TODO: Check for zero division
                let result = lhs / rhs;
                stackframe.lock().unwrap().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Div) => {
                // ldiv
                // long division

                let rhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                // TODO: Check for zero division
                let result = lhs / rhs;
                push_long(&mut stackframe.lock().unwrap().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Div) => {
                // fdiv
                // float div
                let rhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let result = lhs / rhs;
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .push(result.to_bits());
            }
            Instruction::DOp(Op::Div) => {
                // ddiv
                // double division
                let rhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let lhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let result = lhs / rhs;
                push_long(
                    &mut stackframe.lock().unwrap().operand_stack,
                    result.to_bits(),
                );
            }
            Instruction::IOp(Op::Mod) => {
                // irem
                // int remainder
                let rhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                // TODO: Check for zero division
                let result = lhs % rhs;
                stackframe.lock().unwrap().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Mod) => {
                // lrem
                // long modulo

                let rhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                // TODO: Check for zero division
                let result = lhs % rhs;
                push_long(&mut stackframe.lock().unwrap().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Mod) => {
                // frem
                // float rem
                let rhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let result = lhs % rhs;
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .push(result.to_bits());
            }
            Instruction::DOp(Op::Mod) => {
                // drem
                // double remainder
                let rhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let lhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let result = lhs % rhs;
                push_long(
                    &mut stackframe.lock().unwrap().operand_stack,
                    result.to_bits(),
                );
            }
            Instruction::IOp(Op::Neg) => {
                // ineg
                // negate int
                let f = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                let result = -f;
                stackframe.lock().unwrap().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Neg) => {
                // lneg
                // negate long
                let l = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let result = -l;
                push_long(&mut stackframe.lock().unwrap().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Neg) => {
                // fneg
                // negate float
                let f = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let result = -f;
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .push(result.to_bits());
            }
            Instruction::DOp(Op::Neg) => {
                // dneg
                // negate double
                let d = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let result = -d;
                push_long(
                    &mut stackframe.lock().unwrap().operand_stack,
                    result.to_bits(),
                );
            }
            Instruction::IOp(Op::Shl) => {
                // ishl
                // int shift left
                let rhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) & 0x1F;
                let lhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let result = lhs << rhs;
                stackframe.lock().unwrap().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Shl) => {
                // lshl
                // long shift left
                let rhs = stackframe.lock().unwrap().operand_stack.pop().unwrap() & 0x3F;
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let result = lhs << rhs;
                push_long(&mut stackframe.lock().unwrap().operand_stack, result as u64);
            }
            Instruction::IOp(Op::Shr) => {
                // ishr
                // int shift right
                let rhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) & 0x1F;
                let lhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) as i32;
                let result = lhs >> rhs;
                stackframe.lock().unwrap().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Shr) => {
                // lshr
                // long shift right
                let rhs = stackframe.lock().unwrap().operand_stack.pop().unwrap() & 0x3F;
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let result = lhs >> rhs;
                push_long(&mut stackframe.lock().unwrap().operand_stack, result as u64);
            }
            Instruction::IOp(Op::Ushr) => {
                // iushr
                // int logical shift right
                let rhs = (stackframe.lock().unwrap().operand_stack.pop().unwrap()) & 0x1F;
                let lhs = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let result = lhs >> rhs;
                stackframe.lock().unwrap().operand_stack.push(result);
            }
            Instruction::LOp(Op::Ushr) => {
                // lushr
                // long logical shift right
                let rhs = stackframe.lock().unwrap().operand_stack.pop().unwrap() & 0x3F;
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let result = lhs >> rhs;
                push_long(&mut stackframe.lock().unwrap().operand_stack, result);
            }
            Instruction::IOp(Op::And) => {
                // iand
                // int boolean and
                let rhs = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let lhs = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let result = lhs & rhs;
                stackframe.lock().unwrap().operand_stack.push(result);
            }
            Instruction::LOp(Op::And) => {
                // land
                // long boolean and
                let rhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let result = lhs & rhs;
                push_long(&mut stackframe.lock().unwrap().operand_stack, result);
            }
            Instruction::IOp(Op::Or) => {
                // ior
                // int boolean or
                let rhs = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let lhs = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let result = lhs | rhs;
                stackframe.lock().unwrap().operand_stack.push(result);
            }
            Instruction::LOp(Op::Or) => {
                // lor
                // long boolean or
                let rhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let result = lhs | rhs;
                push_long(&mut stackframe.lock().unwrap().operand_stack, result);
            }
            Instruction::IOp(Op::Xor) => {
                // ixor
                // int boolean xor
                let rhs = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let lhs = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let result = lhs ^ rhs;
                stackframe.lock().unwrap().operand_stack.push(result);
            }
            Instruction::LOp(Op::Xor) => {
                // lxor
                // long boolean xor
                let rhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let result = lhs ^ rhs;
                push_long(&mut stackframe.lock().unwrap().operand_stack, result);
            }
            Instruction::IInc(index, inc) => {
                // iinc
                // int increment
                let start = stackframe.lock().unwrap().locals[index] as i32;
                stackframe.lock().unwrap().locals[index] = start.wrapping_add(inc) as u32;
            }
            Instruction::Convert(Type::Int, Type::Long) => {
                // i2l
                // int to long
                let int = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                let long = int as i64;
                push_long(&mut stackframe.lock().unwrap().operand_stack, long as u64);
            }
            Instruction::Convert(Type::Int, Type::Float) => {
                // i2f
                // int to float
                let int = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                let float = int as f32;
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .push(float.to_bits());
            }
            Instruction::Convert(Type::Int, Type::Double) => {
                // i2d
                // int to double
                let int = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                let double = int as f64;
                push_long(
                    &mut stackframe.lock().unwrap().operand_stack,
                    double.to_bits(),
                );
            }
            Instruction::Convert(Type::Long, Type::Int) => {
                // l2i
                // long to int
                let long = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let int = long as u32;
                stackframe.lock().unwrap().operand_stack.push(int);
            }
            Instruction::Convert(Type::Long, Type::Float) => {
                // l2f
                // long to float
                let long = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let float = long as f32;
                stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .push(float.to_bits());
            }
            Instruction::Convert(Type::Long, Type::Double) => {
                // l2d
                // long to double
                let long = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let double = long as f64;
                push_long(
                    &mut stackframe.lock().unwrap().operand_stack,
                    double.to_bits(),
                );
            }
            Instruction::Convert(Type::Float, Type::Int) => {
                // f2i
                // float to integer
                let float = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let int = float as i32;
                stackframe.lock().unwrap().operand_stack.push(int as u32);
            }
            Instruction::Convert(Type::Float, Type::Long) => {
                // f2l
                // float to long
                let float = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let long = float as u64;
                push_long(&mut stackframe.lock().unwrap().operand_stack, long);
            }
            Instruction::Convert(Type::Float, Type::Double) => {
                // f2d
                // float to double
                let float = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let double = float as f64;
                push_long(
                    &mut stackframe.lock().unwrap().operand_stack,
                    double.to_bits(),
                );
            }
            Instruction::Convert(Type::Double, Type::Int) => {
                // d2i
                // double to integer
                let double = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let int = double as u32;
                stackframe.lock().unwrap().operand_stack.push(int);
            }
            Instruction::Convert(Type::Double, Type::Long) => {
                // d2l
                // double to long
                let double = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let int = double as u64;
                push_long(&mut stackframe.lock().unwrap().operand_stack, int);
            }
            Instruction::Convert(Type::Double, Type::Float) => {
                // d2f
                // double to float
                let double = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let float = (double as f32).to_bits();
                stackframe.lock().unwrap().operand_stack.push(float);
            }
            Instruction::Convert(Type::Int, Type::Byte) => {
                // i2b
                // int to byte
                let int = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                let byte = int as i8 as i32;
                stackframe.lock().unwrap().operand_stack.push(byte as u32);
            }
            Instruction::Convert(Type::Int, Type::Char) => {
                // i2c
                // int to char
                let int = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                let char = int as u8;
                stackframe.lock().unwrap().operand_stack.push(char as u32);
            }
            Instruction::Convert(Type::Int, Type::Short) => {
                // i2s
                // int to short
                let int = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                let short = int as i16 as i32;
                stackframe.lock().unwrap().operand_stack.push(short as u32);
            }
            Instruction::LCmp => {
                // lcmp
                // long comparison
                let rhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap() as i64;
                let value = match lhs.cmp(&rhs) {
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                    Ordering::Less => -1,
                };
                stackframe.lock().unwrap().operand_stack.push(value as u32);
            }
            Instruction::FCmp(is_rev) => {
                // fcmp<op>
                // float comparison
                let rhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.lock().unwrap().operand_stack.pop().unwrap());
                let value = if lhs > rhs {
                    1
                } else if (lhs - rhs).abs() < f32::EPSILON {
                    0
                } else if lhs < rhs || is_rev {
                    -1
                } else {
                    1
                } as u32;
                stackframe.lock().unwrap().operand_stack.push(value);
            }
            Instruction::DCmp(is_rev) => {
                // dcmp<op>
                // double comparison
                let rhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let lhs = f64::from_bits(
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap(),
                );
                let value = if lhs > rhs {
                    1
                } else if (lhs - rhs).abs() < f64::EPSILON {
                    0
                } else if lhs < rhs || is_rev {
                    -1
                } else {
                    1
                } as u32;
                stackframe.lock().unwrap().operand_stack.push(value);
            }
            Instruction::IfCmpZ(cmp, branch) => {
                // if<cond>
                // integer comparison to zero
                let lhs = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                let cond = match cmp {
                    Cmp::Eq => lhs == 0,
                    Cmp::Ne => lhs != 0,
                    Cmp::Lt => lhs < 0,
                    Cmp::Ge => lhs >= 0,
                    Cmp::Gt => lhs > 0,
                    Cmp::Le => lhs <= 0,
                };
                if cond {
                    self.pc_register = branch as usize;
                }
            }
            Instruction::ICmp(cnd, branch) => {
                // if_icmp<cond>
                // comparison between integers
                let rhs = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                let lhs = stackframe.lock().unwrap().operand_stack.pop().unwrap() as i32;
                if match cnd {
                    Cmp::Eq => lhs == rhs,
                    Cmp::Ne => lhs != rhs,
                    Cmp::Lt => lhs < rhs,
                    Cmp::Ge => lhs >= rhs,
                    Cmp::Gt => lhs > rhs,
                    Cmp::Le => lhs <= rhs,
                } {
                    self.pc_register = branch as usize;
                }
            }
            Instruction::Goto(goto) => {
                // goto bb1 bb2
                self.pc_register = goto as usize;
            }
            Instruction::Return0 => {
                // return void
                self.return_void();
            }
            Instruction::Return1 => {
                // return one thing
                self.return_one(verbose);
            }
            Instruction::PutStatic(class, name, field_type) => {
                // putstatic
                // put a static field to a class
                let Some(class) = self.class_area.search(&class) else {
                    return Err(format!("Couldn't resolve class {class}"));
                };

                if self.maybe_initialize_class(&class, &stackframe) {
                    return Ok(());
                }

                let &(ref static_ty, staticindex) = class
                    .statics
                    .iter()
                    .find(|(field, _)| field.name == name)
                    .ok_or_else(|| {
                        format!("Couldn't find static `{name}` on class `{}`", class.this)
                    })?;
                if verbose {
                    println!("Putting Static {name} of {}", class.this);
                }
                let mut static_fields = class.static_data.lock().unwrap();

                if field_type.get_size() == 1 {
                    let value = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                    if static_ty.descriptor.is_reference() {
                        self.forgor(static_fields[staticindex], verbose);
                        self.rember(value, verbose);
                    }
                    static_fields[staticindex] = value;
                    drop(static_fields);
                } else {
                    let mut stackframe = stackframe.lock().unwrap();
                    let lower = stackframe.operand_stack.pop().unwrap();
                    let upper = stackframe.operand_stack.pop().unwrap();
                    drop(stackframe);
                    static_fields[staticindex] = upper;
                    static_fields[staticindex + 1] = lower;
                }
            }
            Instruction::GetStatic(class, name, field_type) => {
                // getstatic
                // get a static field from a class
                let Some(class) = self.class_area.search(&class) else {
                    return Err(format!("Couldn't resolve class {class}"));
                };

                if self.maybe_initialize_class(&class, &stackframe) {
                    return Ok(());
                }

                let staticindex = class
                    .statics
                    .iter()
                    .find(|(field, _)| field.name == name)
                    .ok_or_else(|| {
                        format!("Couldn't find static `{name}` on class `{}`", class.this)
                    })?
                    .1;
                if verbose {
                    println!("Getting Static {name} of {}", class.this);
                }
                let static_fields = class.static_data.lock().unwrap();

                if field_type.get_size() == 1 {
                    let value = static_fields[staticindex];
                    drop(static_fields);
                    if field_type.is_reference() {
                        self.rember_temp(&stackframe, value, verbose);
                    }
                    stackframe.lock().unwrap().operand_stack.push(value);
                } else {
                    let upper = static_fields[staticindex];
                    let lower = static_fields[staticindex + 1];
                    drop(static_fields);
                    stackframe
                        .lock()
                        .unwrap()
                        .operand_stack
                        .extend([upper, lower]);
                }
            }
            Instruction::GetField(Some(idx), _class, _name, field_type) => {
                // get a field from an object
                let object_index = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                AnyObj
                    .get_mut(
                        &mut self.heap.lock().unwrap(),
                        object_index as usize,
                        |object_borrow| {
                            if field_type.get_size() == 1 {
                                let value = object_borrow.fields[idx];
                                if field_type.is_reference() {
                                    self.rember_temp(&stackframe, value, verbose);
                                }
                                stackframe.lock().unwrap().operand_stack.push(value);
                            } else {
                                let upper = object_borrow.fields[idx];
                                let lower = object_borrow.fields[idx + 1];
                                stackframe
                                    .lock()
                                    .unwrap()
                                    .operand_stack
                                    .extend([upper, lower]);
                            }
                        },
                    )
                    .unwrap();
            }
            Instruction::PutField(Some(idx), _class, _name, field_type) => {
                // putfield
                // set a field in an object

                let value = if field_type.get_size() == 1 {
                    stackframe.lock().unwrap().operand_stack.pop().unwrap() as u64
                } else {
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap()
                };
                let object_index = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                AnyObj.get_mut(
                    &mut self.heap.lock().unwrap(),
                    object_index as usize,
                    |object_borrow| -> Result<(), String> {
                        if verbose {
                            println!("Object class: {}", object_borrow.this_class());
                        }

                        if field_type.get_size() == 1 {
                            if field_type.is_reference() {
                                self.forgor(object_borrow.fields[idx], verbose);
                                self.rember(value as u32, verbose);
                            }
                            object_borrow.fields[idx] = value as u32;
                        } else {
                            object_borrow.fields[idx] = (value >> 32) as u32;
                            object_borrow.fields[idx + 1] = value as u32;
                        }
                        Ok(())
                    },
                )??;
            }
            Instruction::InvokeVirtual(_class, name, method_type)
            | Instruction::InvokeInterface(_class, name, method_type) => {
                // invokevirtual
                // invoke a method virtually I guess
                let arg_count = method_type.parameter_size;
                let obj_pointer = *stackframe
                    .lock()
                    .unwrap()
                    .operand_stack
                    .iter()
                    .rev()
                    .nth(arg_count)
                    .unwrap();
                let (resolved_class, resolved_method) =
                    AnyObj.get(&self.heap.lock().unwrap(), obj_pointer as usize, |obj| {
                        obj.resolve_method(
                            &self.method_area,
                            &self.class_area,
                            &name,
                            &method_type,
                            verbose,
                        )
                    })?;
                let args_start = stackframe.lock().unwrap().operand_stack.len() - arg_count - 1;
                if verbose {
                    println!(
                        "Args Start: {args_start}\nStack: {:?}",
                        stackframe.lock().unwrap().operand_stack
                    );
                }
                let stack = &mut stackframe.lock().unwrap().operand_stack;
                let mut stack_iter = core::mem::take(stack).into_iter();
                stack.extend((&mut stack_iter).take(args_start));
                stack.push(self.pc_register as u32);

                if verbose {
                    println!(
                        "Invoking Method {} on {}",
                        resolved_method.name, resolved_class.this
                    );
                }
                self.invoke_method(resolved_method, resolved_class);
                self.pc_register = 0;

                let new_stackframe = self.stack.last().unwrap().clone();

                let mut stackframe_lock = new_stackframe.lock().unwrap();
                let new_locals = &mut stackframe_lock.locals;
                for (index, value) in stack_iter.enumerate() {
                    if verbose {
                        println!("new_locals[{index}]={value}");
                    }
                    new_locals[index] = value;
                }
                if verbose {
                    println!("new locals: {new_locals:?}");
                }
                drop(stackframe_lock);
            }
            Instruction::InvokeSpecial(class, name, method_type) => {
                // invoke an instance method
                let current_class = if &*name == "<init>" {
                    class
                } else {
                    self.class_area
                        .search(&class)
                        .ok_or_else(|| format!("Can't find class {class}"))?
                        .super_class
                        .clone()
                };
                let (class_ref, method_ref) = self
                    .method_area
                    .search(&current_class, &name, &method_type)
                    .ok_or_else(|| format!("Error during InvokeSpecial; {current_class}.{name}"))?;
                let args_start =
                    stackframe.lock().unwrap().operand_stack.len() - method_type.parameter_size - 1;
                let stack = &mut stackframe.lock().unwrap().operand_stack;
                if verbose {
                    println!("arg start: {args_start} stack: {stack:?}");
                }
                let mut stack_iter = core::mem::take(stack).into_iter();
                stack.extend((&mut stack_iter).take(args_start));
                stack.push(self.pc_register as u32);

                if verbose {
                    println!(
                        "Invoking Special Method {} on {}",
                        method_ref.name, class_ref.this
                    );
                }
                self.invoke_method(method_ref, class_ref);
                self.pc_register = 0;

                let new_stackframe = self.stack.last().unwrap().clone();

                let mut stackframe_lock = new_stackframe.lock().unwrap();
                let new_locals = &mut stackframe_lock.locals;
                for (index, value) in stack_iter.enumerate() {
                    if verbose {
                        println!("new_locals[{index}]={value}");
                    }
                    new_locals[index] = value;
                }
                if verbose {
                    println!("new locals: {new_locals:?}");
                }
                drop(stackframe_lock);
            }
            Instruction::InvokeStatic(class, name, method_type) => {
                // make a static method
                let (class_ref, method_ref) = self
                    .method_area
                    .search(&class, &name, &method_type)
                    .ok_or_else(|| {
                        format!("Error during InvokeStatic; {class}.{name}: {method_type:?}")
                    })?;

                if self.maybe_initialize_class(&class_ref, &stackframe) {
                    return Ok(());
                }

                if verbose {
                    println!(
                        "Invoking Static Method {} on {}",
                        method_ref.name, class_ref.this,
                    );
                }
                let args_start =
                    stackframe.lock().unwrap().operand_stack.len() - method_type.parameter_size;
                let stack = &mut stackframe.lock().unwrap().operand_stack;
                let mut stack_iter = core::mem::take(stack).into_iter();
                stack.extend((&mut stack_iter).take(args_start));
                stack.push(self.pc_register as u32);

                self.invoke_method(method_ref, class_ref);
                self.pc_register = 0;

                let new_stackframe = self.stack.last().unwrap().clone();

                let mut stackframe_lock = new_stackframe.lock().unwrap();
                let new_locals = &mut stackframe_lock.locals;
                for (index, value) in stack_iter.enumerate() {
                    if verbose {
                        println!("new_locals[{index}]={value}");
                    }
                    new_locals[index] = value;
                }
                if verbose {
                    println!("new locals: {new_locals:?}");
                }
                drop(stackframe_lock);
            }
            Instruction::InvokeDynamic(bootstrap_index, method_name, method_type) => {
                let bootstrap_method = stackframe.lock().unwrap().class.bootstrap_methods
                    [bootstrap_index as usize]
                    .clone();
                self.invoke_dynamic(
                    &method_name,
                    bootstrap_method,
                    method_type,
                    &stackframe,
                    verbose,
                )?;
            }
            Instruction::New(class) => {
                // make a new object instance
                let Some(class) = self.class_area.search(&class) else {
                    return Err(format!("Couldn't find class {class}"));
                };
                if self.maybe_initialize_class(&class, &stackframe) {
                    return Ok(());
                }
                let objectref = self
                    .heap
                    .lock()
                    .unwrap()
                    .allocate(Object::from_class(&class));
                stackframe.lock().unwrap().operand_stack.push(objectref);
                self.rember_temp(&stackframe, objectref, verbose);
            }
            Instruction::IfNull(is_rev, branch) => {
                // ifnull | ifnonnull
                let ptr = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                if (ptr == NULL) ^ (is_rev) {
                    self.pc_register = branch as usize;
                }
            }
            Instruction::NewArray1(field_type) => {
                // {ty}newarray
                let count = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let new_array = Array1::new(count as usize, field_type);
                let objectref = self.heap.lock().unwrap().allocate(new_array);
                stackframe.lock().unwrap().operand_stack.push(objectref);
                self.rember_temp(&stackframe, objectref, verbose);
            }
            Instruction::NewArray2(field_type) => {
                // {ty}newarray
                let count = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let new_array = Array2::new(count as usize, field_type);
                let objectref = self.heap.lock().unwrap().allocate(new_array);
                stackframe.lock().unwrap().operand_stack.push(objectref);
                self.rember_temp(&stackframe, objectref, verbose);
            }
            Instruction::ArrayStore1 => {
                // store 1 value into an array
                let value = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let index = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let array_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                let old =
                    Array1.get_mut(&mut self.heap.lock().unwrap(), array_ref as usize, |arr| {
                        let old = arr.contents[index as usize];
                        arr.contents[index as usize] = value;
                        old
                    })?;
                if ArrayType::SELF.get(
                    &self.heap.lock().unwrap(),
                    array_ref as usize,
                    FieldType::is_reference,
                )? {
                    // if it's a reference type, increment the ref count
                    self.rember(value, verbose);
                    self.forgor(old, verbose);
                }
            }
            Instruction::ArrayStore2 => {
                // store 2 values into an array
                let value = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let index = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let array_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                Array2.get_mut(&mut self.heap.lock().unwrap(), array_ref as usize, |arr| {
                    arr.contents[index as usize] = value;
                })?;
            }
            Instruction::ArrayLoad1 => {
                // load 1 value from an array
                let index = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let array_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                let value =
                    Array1.get_mut(&mut self.heap.lock().unwrap(), array_ref as usize, |arr| {
                        arr.contents[index as usize]
                    })?;
                stackframe.lock().unwrap().operand_stack.push(value);
                if ArrayType::SELF.get(
                    &self.heap.lock().unwrap(),
                    array_ref as usize,
                    FieldType::is_reference,
                )? {
                    self.rember_temp(&stackframe, value, verbose);
                }
            }
            Instruction::ArrayLoad2 => {
                // load 2 values from an array
                let index = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let array_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                let value =
                    Array2.get_mut(&mut self.heap.lock().unwrap(), array_ref as usize, |arr| {
                        arr.contents[index as usize]
                    })?;
                push_long(&mut stackframe.lock().unwrap().operand_stack, value);
            }
            Instruction::NewMultiArray(dimensions, arr_type) => {
                let mut stackframe_ref = stackframe.lock().unwrap();
                let dimension_sizes = (0..dimensions)
                    .map(|_| stackframe_ref.operand_stack.pop().unwrap())
                    .collect::<Vec<_>>();
                drop(stackframe_ref);
                // make all the thingies
                let allocation = allocate_multi_array(
                    &mut self.heap.lock().unwrap(),
                    &dimension_sizes,
                    arr_type,
                )?;
                stackframe.lock().unwrap().operand_stack.push(allocation);
                self.rember_temp(&stackframe, allocation, verbose);
            }
            Instruction::ArrayLength => {
                let arr_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap() as usize;
                let length = Array1.get(&self.heap.lock().unwrap(), arr_ref, |arr| {
                    arr.contents.len()
                })? as u32;
                stackframe.lock().unwrap().operand_stack.push(length);
            }
            Instruction::CheckedCast(ty) => {
                let objref = *stackframe.lock().unwrap().operand_stack.last().unwrap();
                if objref != NULL {
                    // get the fields of the given class; if it works, we have a subclass
                    let class_name = &self.class_area.search(&ty).unwrap().this;
                    let obj_works = AnyObj
                        .get(&self.heap.lock().unwrap(), objref as usize, |o| {
                            o.isinstance(&self.class_area, class_name, verbose)
                        })
                        .is_ok();
                    if !obj_works {
                        let obj_type =
                            AnyObj.get(&self.heap.lock().unwrap(), objref as usize, |obj| {
                                obj.this_class()
                            })?;
                        return Err(format!(
                            "CheckedCast failed; expected a(n) {ty} but got a(n) {obj_type}"
                        ));
                    }
                }
            }
            Instruction::AThrow => {
                let objref = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                self.throw(stackframe, objref, verbose)?;
            }
            other => return Err(format!("Invalid Opcode: {other:?}")),
        }
        Ok(())
    }

    /// # Panics
    pub fn rember_temp(&self, stackframe: &Mutex<StackFrame>, value: u32, verbose: bool) {
        self.heap.lock().unwrap().inc_ref(value);
        stackframe.lock().unwrap().garbage.push(value);
        if verbose {
            println!("Rember (temporary) {value}");
        }
    }

    /// # Panics
    pub fn rember(&self, value: u32, verbose: bool) {
        self.heap.lock().unwrap().inc_ref(value);
        if verbose {
            println!("Rember {value}");
        }
    }

    /// # Panics
    pub fn forgor(&self, value: u32, verbose: bool) {
        self.heap.lock().unwrap().dec_ref(value);
        if verbose {
            println!("forgor {value}");
        }
    }

    /// # Panics
    pub fn maybe_initialize_class(
        &mut self,
        class: &Class,
        stackframe: &Mutex<StackFrame>,
    ) -> bool {
        if class.initialized.is_completed() {
            return false;
        }
        // mark the class as initialized
        class.initialized.call_once(|| ());
        let Some((class, method)) = self.method_area.search(
            &class.this,
            "<clinit>",
            &MethodDescriptor {
                parameter_size: 0,
                parameters: Vec::new(),
                return_type: None,
            },
        ) else {
            return false;
        };
        stackframe
            .lock()
            .unwrap()
            .operand_stack
            .push((self.pc_register - 1) as u32);
        self.invoke_method(method, class);
        true
    }

    fn get_code(stackframe: &Mutex<StackFrame>, idx: usize) -> Instruction {
        stackframe
            .lock()
            .unwrap()
            .method
            .code
            .as_bytecode()
            .unwrap()
            .code[idx]
            .clone()
    }

    fn get_pc_byte(&mut self, stackframe: &Mutex<StackFrame>) -> Instruction {
        let b = Self::get_code(stackframe, self.pc_register);
        self.pc_register += 1;
        b
    }

    pub fn invoke_method(&mut self, method: Arc<Method>, class: Arc<Class>) {
        let stackframe = StackFrame::from_method(method, class);
        self.stack.push(Arc::new(Mutex::new(stackframe)));
        self.pc_register = 0;
    }

    fn throw(
        &mut self,
        mut stackframe: Arc<Mutex<StackFrame>>,
        exception_ptr: u32,
        verbose: bool,
    ) -> Result<(), String> {
        loop {
            let mut stackframe_borrow = stackframe.lock().unwrap();
            for entry in &stackframe_borrow
                .method
                .code
                .as_bytecode()
                .unwrap()
                .exception_table
            {
                if !(entry.start_pc..=entry.end_pc).contains(&(self.pc_register as u16)) {
                    continue;
                }
                if entry.catch_type.is_none()
                    || entry.catch_type.as_ref().is_some_and(|catch_type| {
                        AnyObj
                            .get(&self.heap.lock().unwrap(), exception_ptr as usize, |obj| {
                                obj.isinstance(&self.class_area, catch_type, verbose)
                            })
                            .is_ok_and(|a| a)
                    })
                {
                    if verbose {
                        println!("Found an exception handler! {entry:?}");
                    }
                    self.pc_register = entry.handler_pc as usize;
                    stackframe_borrow.operand_stack.push(exception_ptr);
                    return Ok(());
                }
            }
            if verbose {
                println!(
                    "No exception handlers found : {:?}",
                    stackframe_borrow.method.name
                );
            }
            drop(stackframe_borrow);
            self.stack.pop();
            match self.stack.last() {
                Some(s) => stackframe = s.clone(),
                None => return Err(String::from("Exception propagated past main")),
            }
            self.pc_register = stackframe.lock().unwrap().operand_stack.pop().unwrap() as usize;
        }
    }

    #[allow(clippy::too_many_lines)]
    fn invoke_dynamic(
        &mut self,
        method_name: &str,
        method_handle: BootstrapMethod,
        method_descriptor: MethodDescriptor,
        stackframe: &Mutex<StackFrame>,
        verbose: bool,
    ) -> Result<(), String> {
        match (method_name, method_handle, method_descriptor) {
            (
                "makeConcatWithConstants",
                BootstrapMethod {
                    method:
                        MethodHandle::InvokeStatic {
                            class,
                            name,
                            method_type: _,
                        },
                    args,
                },
                MethodDescriptor {
                    parameter_size,
                    parameters,
                    return_type: Some(FieldType::Object(obj)),
                },
            ) if &*class == "java/lang/invoke/StringConcatFactory"
                && &*name == "makeConcatWithConstants"
                && &*obj == "java/lang/String" =>
            {
                if verbose {
                    println!("{:?}", self.heap);
                }
                let [Constant::String(str) | Constant::StringRef(str)] = &args[..] else {
                    return Err(format!("Expected a single template string; got {args:?}"));
                };
                let mut output = String::new();
                let mut parameters_iter = parameters.iter();

                let mut stackframe_borrow = stackframe.lock().unwrap();
                let mut args_iter = (0..parameter_size)
                    .map(|_| stackframe_borrow.operand_stack.pop().unwrap())
                    .collect::<Vec<_>>();
                drop(stackframe_borrow);

                for c in str.chars() {
                    if c != '\u{1}' {
                        output.push(c);
                        continue;
                    }
                    let Some(field_type) = parameters_iter.next() else {
                        return Err(format!("Not enough parameters for java/lang/invoke/StringConcatFactory.makeConcatWithConstants: {str:?} {parameters:?}"));
                    };
                    if field_type.get_size() == 2 {
                        let value = pop_long(&mut args_iter).unwrap();
                        // since the stack is reversed, this stuff is goofy
                        let value = value >> 32 | value << 32;
                        match field_type {
                            FieldType::Long => {
                                write!(output, "{}", value as i64)
                                    .map_err(|err| format!("{err:?}"))?;
                            }
                            FieldType::Double => {
                                write!(output, "{}", f64::from_bits(value))
                                    .map_err(|err| format!("{err:?}"))?;
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        let value = args_iter.pop().unwrap();
                        match field_type {
                            FieldType::Boolean => {
                                write!(output, "{}", value == 1)
                                    .map_err(|err| format!("{err:?}"))?;
                            }
                            FieldType::Int | FieldType::Short | FieldType::Byte => {
                                write!(output, "{}", value as i32)
                                    .map_err(|err| format!("{err:?}"))?;
                            }
                            FieldType::Char => {
                                output.push(char::from_u32(value).unwrap());
                            }
                            FieldType::Float => {
                                write!(output, "{}", f32::from_bits(value))
                                    .map_err(|err| format!("{err:?}"))?;
                            }
                            FieldType::Object(class) if &**class == "java/lang/String" => {
                let heap_borrow = self.heap.lock().unwrap();
                if verbose {
                    println!("{value}");
                }
                StringObj::SELF.get(&heap_borrow, value as usize, |str| {
                    write!(output, "{str}").map_err(|err| format!("{err:?}"))
                }).unwrap()?;
                drop(heap_borrow);
                            }
                            other => return Err(format!("Unsupported item for java/lang/invoke/StringConcatFactory.makeConcatWithConstants: {other:?}")),
                        }
                    }
                }
                let heap_pointer = self.heap.lock().unwrap().allocate_str(Arc::from(&*output));
                stackframe.lock().unwrap().operand_stack.push(heap_pointer);
                self.rember_temp(stackframe, heap_pointer, verbose);
                if verbose {
                    println!("makeConcatWithConstants: {heap_pointer}");
                }
            }
            (n, h, d) => return Err(format!("Error during InvokeDynamic: {n}: {d:?}; {h:?}")),
        }
        Ok(())
    }

    fn collect_garbage(&self, stackframe: &Mutex<StackFrame>) {
        let mut heap_borrow = self.heap.lock().unwrap();
        for ptr in core::mem::take(&mut stackframe.lock().unwrap().garbage) {
            // println!("Collecting Garbage {ptr}");
            heap_borrow.dec_ref(ptr);
        }
    }

    /// # Panics
    pub fn return_void(&mut self) {
        self.collect_garbage(self.stack.last().unwrap());
        self.stack.pop();
        if self.stack.is_empty() {
            return;
        }
        let stackframe = self.stack.last().unwrap();
        let return_address = stackframe.lock().unwrap().operand_stack.pop().unwrap();
        self.pc_register = return_address as usize;
    }

    /// # Panics
    pub fn return_one(&mut self, verbose: bool) {
        let old_stackframe = self.stack.pop().unwrap();
        let is_reference = old_stackframe
            .lock()
            .unwrap()
            .method
            .descriptor
            .return_type
            .as_ref()
            .unwrap()
            .is_reference();
        let ret_value = old_stackframe.lock().unwrap().operand_stack.pop().unwrap();
        if verbose {
            println!("{ret_value}");
        }
        let stackframe = self.stack.last().unwrap();
        if is_reference {
            self.rember_temp(stackframe, ret_value, verbose);
        }
        let ret_address = stackframe.lock().unwrap().operand_stack.pop().unwrap();
        self.pc_register = ret_address as usize;
        stackframe.lock().unwrap().operand_stack.push(ret_value);
        self.collect_garbage(&old_stackframe);
    }

    /// # Panics
    pub fn return_two(&mut self, verbose: bool) {
        let old_stackframe = self.stack.pop().unwrap();
        let ret_value = pop_long(&mut old_stackframe.lock().unwrap().operand_stack).unwrap();
        if verbose {
            println!("{ret_value}");
        }
        let stackframe = self.stack.last().unwrap();
        let ret_address = stackframe.lock().unwrap().operand_stack.pop().unwrap();
        self.pc_register = ret_address as usize;
        push_long(&mut stackframe.lock().unwrap().operand_stack, ret_value);
        self.collect_garbage(&old_stackframe);
    }
}

fn allocate_multi_array(
    heap: &mut Heap,
    depth: &[u32],
    arr_type: FieldType,
) -> Result<u32, String> {
    match depth {
        [size] => Ok(heap.allocate(if arr_type.get_size() == 2 {
            Array2::new(*size as usize, arr_type)
        } else {
            Array1::new(*size as usize, arr_type)
        })),
        [size, rest @ ..] => {
            let FieldType::Array(inner_type) = arr_type else {
                return Err(format!("Expected an array type; got {arr_type:?}"));
            };
            let current_array = (0..*size)
                .map(|_| {
                    let idx = allocate_multi_array(heap, rest, *inner_type.clone())?;
                    heap.inc_ref(idx);
                    Ok(idx)
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(heap.allocate(Array1::from_vec(current_array, *inner_type)))
        }
        [] => Err(String::from("Can't create 0-dimensional array")),
    }
}

pub fn pop_long(stack: &mut Vec<u32>) -> Option<u64> {
    let lower = stack.pop()?;
    let upper = stack.pop()?;
    Some((upper as u64) << 32 | lower as u64)
}

pub fn push_long(stack: &mut Vec<u32>, l: u64) {
    let lower = (l & 0xFFFF_FFFF) as u32;
    let upper = (l >> 32) as u32;
    stack.push(upper);
    stack.push(lower);
}

fn value_store(stackframe: &Mutex<StackFrame>, index: usize) {
    let value = stackframe.lock().unwrap().operand_stack.pop().unwrap();
    stackframe.lock().unwrap().locals[index] = value;
}

fn value_load(stackframe: &Mutex<StackFrame>, index: usize) {
    let value = stackframe.lock().unwrap().locals[index];
    stackframe.lock().unwrap().operand_stack.push(value);
}

fn long_store(stackframe: &Mutex<StackFrame>, index: usize) {
    let lower = stackframe.lock().unwrap().operand_stack.pop().unwrap();
    let upper = stackframe.lock().unwrap().operand_stack.pop().unwrap();
    stackframe.lock().unwrap().locals[index] = upper;
    stackframe.lock().unwrap().locals[index + 1] = lower;
}

fn long_load(stackframe: &Mutex<StackFrame>, index: usize) {
    let value_upper = stackframe.lock().unwrap().locals[index];
    let value_lower = stackframe.lock().unwrap().locals[index + 1];
    stackframe
        .lock()
        .unwrap()
        .operand_stack
        .extend([value_upper, value_lower]);
}

pub fn heap_allocate(heap: &mut Vec<Arc<Mutex<Object>>>, element: Object) -> u32 {
    let length = heap.len();

    heap.push(Arc::new(Mutex::new(element)));

    length as u32
}
