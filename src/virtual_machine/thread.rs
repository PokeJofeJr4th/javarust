use rand::{rngs::ThreadRng, thread_rng, Rng};
use std::{
    cmp::Ordering,
    fmt::Write,
    sync::{Arc, Mutex},
};

use crate::{
    class::{BootstrapMethod, Class, Constant, FieldType, Method, MethodDescriptor, MethodHandle},
    virtual_machine::search_method_area,
};

use super::{
    instruction::Type,
    native,
    object::{AnyObj, Array1, Array2, ArrayFields, Object, ObjectFinder, StringObj},
    Cmp, Instruction, Op, StackFrame,
};

pub struct Thread {
    pub pc_register: usize,
    pub stack: Vec<Arc<Mutex<StackFrame>>>,
    pub method_area: Arc<[(Arc<Class>, Arc<Method>)]>,
    pub class_area: Arc<[Arc<Class>]>,
    pub heap: Arc<Mutex<Vec<Arc<Mutex<Object>>>>>,
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
                // nop
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
                let str_ptr = heap_allocate(
                    &mut self.heap.lock().unwrap(),
                    StringObj::new(&self.class_area, str),
                );
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
            Instruction::IfCmp(cmp, branch) => {
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
            Instruction::GetStatic(class, name, field_type) => {
                // getstatic
                // get a static field from a class
                let Some(class) = search_class_area(&self.class_area, &class) else {
                    return Err(format!("Couldn't resolve class {class}"));
                };

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
                    stackframe.lock().unwrap().operand_stack.push(value);
                } else {
                    let upper = static_fields[staticindex];
                    let lower = static_fields[staticindex + 1];
                    stackframe
                        .lock()
                        .unwrap()
                        .operand_stack
                        .extend([upper, lower]);
                }
            }
            Instruction::GetField(class, name, field_type) => {
                // get a field from an object
                let Some(class) = search_class_area(&self.class_area, &class) else {
                    return Err(format!("Couldn't resolve class {class}"));
                };

                let field_index = class
                    .fields
                    .iter()
                    .find(|(field, _)| field.name == name)
                    .ok_or_else(|| {
                        format!("Couldn't find field `{name}` on class `{}`", class.this)
                    })?
                    .1;
                let object_index = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                AnyObj
                    .get_mut(
                        &self.heap.lock().unwrap(),
                        object_index as usize,
                        |object_borrow| {
                            if verbose {
                                println!(
                                    "Getting Field {name} of {} at {object_index}",
                                    class.this
                                );
                            }
                            let object_fields = object_borrow.class_mut(&class.this).unwrap();

                            if field_type.get_size() == 1 {
                                let value = object_fields.fields[field_index];
                                stackframe.lock().unwrap().operand_stack.push(value);
                            } else {
                                let upper = object_fields.fields[field_index];
                                let lower = object_fields.fields[field_index + 1];
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
            Instruction::PutField(class, name, field_type) => {
                // putfield
                // set a field in an object

                let Some(class) = search_class_area(&self.class_area, &class) else {
                    return Err(format!("Couldn't resolve class {class}"));
                };

                let field_index = class
                    .fields
                    .iter()
                    .find(|(field, _)| field.name == name)
                    .ok_or_else(|| {
                        format!("Couldn't find field `{name}` on class `{}`", class.this)
                    })?
                    .1;
                let value = if field_type.get_size() == 1 {
                    stackframe.lock().unwrap().operand_stack.pop().unwrap() as u64
                } else {
                    pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap()
                };
                let object_index = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                AnyObj
                    .get_mut(
                        &self.heap.lock().unwrap(),
                        object_index as usize,
                        |object_borrow| {
                            let object_fields = object_borrow.class_mut(&class.this).unwrap();

                            if field_type.get_size() == 1 {
                                object_fields.fields[field_index] = value as u32;
                            } else {
                                object_fields.fields[field_index] = (value >> 32) as u32;
                                object_fields.fields[field_index + 1] = value as u32;
                            }
                        },
                    )
                    .unwrap();
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
                        obj.resolve_method(&self.method_area, &self.class_area, &name, &method_type)
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
                        "Invoking Virtual Method {} on {}",
                        resolved_method.name, resolved_class.this
                    );
                }
                self.invoke_method(resolved_method, resolved_class);
                self.pc_register = 0;

                let new_stackframe = self.stack.last().unwrap().clone();

                let new_locals = &mut new_stackframe.lock().unwrap().locals;
                for (index, value) in stack_iter.enumerate() {
                    if verbose {
                        println!("new_locals[{index}]={value}");
                    }
                    new_locals[index] = value;
                }
                if verbose {
                    println!("new locals: {new_locals:?}");
                }
            }
            Instruction::InvokeSpecial(class, name, method_type) => {
                // invoke an instance method
                let (class_ref, method_ref) =
                    search_method_area(&self.method_area, &class, &name, &method_type)
                        .ok_or_else(|| format!("Error during InvokeSpecial; {class}.{name}"))?;
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

                let new_locals = &mut new_stackframe.lock().unwrap().locals;
                for (index, value) in stack_iter.rev().enumerate() {
                    if verbose {
                        println!("new_locals[{index}]={value}");
                    }
                    new_locals[index] = value;
                }
                if verbose {
                    println!("new locals: {new_locals:?}");
                }
            }
            Instruction::InvokeStatic(class, name, method_type) => {
                // make a static method
                let (class_ref, method_ref) =
                    search_method_area(&self.method_area, &class, &name, &method_type).ok_or_else(
                        || format!("Error during InvokeStatic; {class}.{name}: {method_type:?}"),
                    )?;
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

                let new_locals = &mut new_stackframe.lock().unwrap().locals;
                for (index, value) in stack_iter.enumerate() {
                    if verbose {
                        println!("new_locals[{index}]={value}");
                    }
                    new_locals[index] = value;
                }
                if verbose {
                    println!("new locals: {new_locals:?}");
                }
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
                let Some(class) = search_class_area(&self.class_area, &class) else {
                    return Err(format!("Couldn't find class {class}"));
                };
                let objectref = heap_allocate(
                    &mut self.heap.lock().unwrap(),
                    Object::from_class(&self.class_area, &class),
                );
                stackframe.lock().unwrap().operand_stack.push(objectref);
            }
            Instruction::IfNull(is_rev, branch) => {
                // ifnull | ifnonnull
                let ptr = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                if (ptr == u32::MAX) ^ (is_rev) {
                    self.pc_register += branch as usize;
                }
            }
            Instruction::NewArray1(field_type) => {
                // {ty}newarray
                let count = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let new_array = Array1::new(&self.class_area, count as usize, field_type);
                let objectref = heap_allocate(&mut self.heap.lock().unwrap(), new_array);
                stackframe.lock().unwrap().operand_stack.push(objectref);
            }
            Instruction::NewArray2(field_type) => {
                // {ty}newarray
                let count = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let new_array = Array2::new(&self.class_area, count as usize, field_type);
                let objectref = heap_allocate(&mut self.heap.lock().unwrap(), new_array);
                stackframe.lock().unwrap().operand_stack.push(objectref);
            }
            Instruction::ArrayStore1 => {
                // store 1 value into an array
                let value = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let index = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let array_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                Array1.get_mut(&self.heap.lock().unwrap(), array_ref as usize, |arr| {
                    arr.contents[index as usize] = value;
                })?;
            }
            Instruction::ArrayStore2 => {
                // store 2 values into an array
                let value = pop_long(&mut stackframe.lock().unwrap().operand_stack).unwrap();
                let index = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let array_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                Array2.get_mut(&self.heap.lock().unwrap(), array_ref as usize, |arr| {
                    arr.contents[index as usize] = value;
                })?;
            }
            Instruction::ArrayLoad1 => {
                // load 1 value from an array
                let index = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let array_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                let value =
                    Array1.get_mut(&self.heap.lock().unwrap(), array_ref as usize, |arr| {
                        arr.contents[index as usize]
                    })?;
                stackframe.lock().unwrap().operand_stack.push(value);
            }
            Instruction::ArrayLoad2 => {
                // load 2 values from an array
                let index = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let array_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                let value =
                    Array2.get_mut(&self.heap.lock().unwrap(), array_ref as usize, |arr| {
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
                    &self.class_area,
                    &mut self.heap.lock().unwrap(),
                    &dimension_sizes,
                    arr_type,
                )?;
                stackframe.lock().unwrap().operand_stack.push(allocation);
            }
            other => return Err(format!("Invalid Opcode: {other:?}")),
        }
        Ok(())
    }

    fn get_code(stackframe: &Mutex<StackFrame>, idx: usize) -> Instruction {
        stackframe
            .lock()
            .unwrap()
            .method
            .code
            .as_ref()
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
                StringObj.get(&heap_borrow, value as usize, |str| {
                    write!(output, "{str}").map_err(|err| format!("{err:?}"))
                }).unwrap()?;
                drop(heap_borrow);
                            }
                            other => return Err(format!("Unsupported item for java/lang/invoke/StringConcatFactory.makeConcatWithConstants: {other:?}")),
                        }
                    }
                }
                let heap_pointer = heap_allocate(
                    &mut self.heap.lock().unwrap(),
                    StringObj::new(&self.class_area, Arc::from(&*output)),
                );
                stackframe.lock().unwrap().operand_stack.push(heap_pointer);
                if verbose {
                    println!("makeConcatWithConstants: {heap_pointer}");
                }
            }
            (n, h, d) => return Err(format!("Error during InvokeDynamic: {n}: {d:?}; {h:?}")),
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn invoke_native(
        &mut self,
        stackframe: &Mutex<StackFrame>,
        verbose: bool,
    ) -> Result<(), String> {
        let name = stackframe.lock().unwrap().method.name.clone();
        let method_type = stackframe.lock().unwrap().method.descriptor.clone();
        let class = stackframe.lock().unwrap().class.this.clone();
        match (&*class, &*name, method_type) {
            (
                "java/util/Arrays",
                "toString",
                MethodDescriptor {
                    parameter_size: _,
                    parameters,
                    return_type: _,
                },
            ) => {
                let arr_ref = stackframe.lock().unwrap().locals[0];

                let [field_type] = &parameters[..] else {
                    return Err(format!(
                        "java/util/Arrays.toString Expected one parameter; got {parameters:?}"
                    ));
                };
                let value = if field_type.get_size() == 2 {
                    Array2.get(
                        &self.heap.lock().unwrap(),
                        arr_ref as usize,
                        match field_type {
                            FieldType::Double => |arr: ArrayFields<'_, u64>| {
                                format!("{:?}", unsafe {
                                    &*std::ptr::addr_of!(arr.contents).cast::<Vec<f64>>()
                                })
                            },
                            FieldType::Long => |arr: ArrayFields<'_, u64>| {
                                format!("{:?}", unsafe {
                                    &*std::ptr::addr_of!(arr.contents).cast::<Vec<i64>>()
                                })
                            },
                            _ => unreachable!(),
                        },
                    )?
                } else {
                    Array1.get(
                        &self.heap.lock().unwrap(),
                        arr_ref as usize,
                        match field_type {
                            FieldType::Int => |arr: ArrayFields<'_, u32>| {
                                format!("{:?}", unsafe {
                                    &*std::ptr::addr_of!(arr.contents).cast::<Vec<i32>>()
                                })
                            },
                            FieldType::Float => |arr: ArrayFields<'_, u32>| {
                                format!("{:?}", unsafe {
                                    &*std::ptr::addr_of!(arr.contents).cast::<Vec<f32>>()
                                })
                            },
                            _ => |arr: ArrayFields<'_, u32>| {
                                format!(
                                    "{:?}",
                                    arr.contents
                                        .iter()
                                        .map(|item| format!("&{item}"))
                                        .collect::<Vec<_>>()
                                )
                            },
                        },
                    )?
                };
                let string_ref = heap_allocate(
                    &mut self.heap.lock().unwrap(),
                    StringObj::new(&self.class_area, Arc::from(&*value)),
                );
                stackframe.lock().unwrap().operand_stack.push(string_ref);
                self.return_one(verbose);
            }
            ("java/util/Arrays", "deepToString", _) => {
                let arr_ref = stackframe.lock().unwrap().operand_stack.pop().unwrap();

                let string_value =
                    native::arrays::deep_to_string(&self.heap.lock().unwrap(), arr_ref as usize)?;

                let str_pointer = heap_allocate(
                    &mut self.heap.lock().unwrap(),
                    StringObj::new(&self.class_area, Arc::from(string_value)),
                );
                stackframe.lock().unwrap().operand_stack.push(str_pointer);
                self.return_one(verbose);
            }
            ("java/lang/String", "length", _) => {
                let string_ref = stackframe.lock().unwrap().locals[0];
                StringObj
                    .get(&self.heap.lock().unwrap(), string_ref as usize, |str| {
                        stackframe
                            .lock()
                            .unwrap()
                            .operand_stack
                            .push(str.len() as u32);
                    })
                    .unwrap();
                self.return_one(verbose);
            }
            ("java/lang/String", "charAt", _) => {
                let string_ref = stackframe.lock().unwrap().locals[0];
                let char = stackframe.lock().unwrap().locals[1];
                StringObj
                    .get(&self.heap.lock().unwrap(), string_ref as usize, |str| {
                        stackframe
                            .lock()
                            .unwrap()
                            .operand_stack
                            .push(str.chars().nth(char as usize).unwrap_or(0 as char) as u32);
                    })
                    .unwrap();
                self.return_one(verbose);
            }
            (
                "java/util/Random",
                "nextInt",
                MethodDescriptor {
                    parameter_size: 1, ..
                },
            ) => {
                let obj_ref = stackframe.lock().unwrap().locals[0];
                if verbose {
                    println!("java/util/Random.nextInt(int): obj_ref={obj_ref}");
                }
                let right_bound = stackframe.lock().unwrap().locals[1];
                if verbose {
                    println!("java/util/Random.nextInt(int): right_bound={right_bound}");
                }
                let result = stackframe
                    .lock()
                    .unwrap()
                    .class
                    .as_ref()
                    .get_mut(&self.heap.lock().unwrap(), obj_ref as usize, |random_obj| {
                        random_obj.native_fields[0]
                            .downcast_mut::<ThreadRng>()
                            .unwrap()
                            .gen_range(0..right_bound)
                    })
                    .unwrap();
                stackframe.lock().unwrap().operand_stack.push(result);
                self.return_one(verbose);
            }
            ("java/lang/Math", "sqrt", _) => {
                let arg_type = stackframe.lock().unwrap().method.descriptor.parameters[0].clone();
                match arg_type {
                    FieldType::Double => {
                        let mut stackframe = stackframe.lock().unwrap();
                        let param = f64::from_bits(
                            (stackframe.locals[0] as u64) << 32 | (stackframe.locals[1] as u64),
                        );
                        push_long(&mut stackframe.operand_stack, param.sqrt().to_bits());
                        drop(stackframe);
                        self.return_two(verbose);
                    }
                    other => return Err(format!("java/lang/Math.sqrt({other:?}) is not defined")),
                }
            }
            (
                "java/lang/StringBuilder",
                "<init>",
                MethodDescriptor {
                    parameter_size: 1,
                    parameters: _,
                    return_type: None,
                },
            ) => {
                // println!("{stackframe:?}");
                native::string_builder::init(&self.heap.lock().unwrap(), stackframe)?;
                self.return_void();
            }
            ("java/lang/StringBuilder", "setCharAt", _) => {
                let builder_ref = stackframe.lock().unwrap().locals[0];
                let index = stackframe.lock().unwrap().locals[1];
                let char = stackframe.lock().unwrap().locals[2];
                native::string_builder::set_char_at(
                    &self.heap.lock().unwrap(),
                    builder_ref as usize,
                    index as usize,
                    char::from_u32(char).unwrap(),
                )?;
                self.return_void();
            }
            ("java/lang/StringBuilder", "toString", _) => {
                let builder_ref = stackframe.lock().unwrap().locals[0];
                let string = native::string_builder::to_string(
                    &self.heap.lock().unwrap(),
                    builder_ref as usize,
                )?;
                let string_ref = heap_allocate(
                    &mut self.heap.lock().unwrap(),
                    StringObj::new(&self.class_area, string),
                );

                stackframe.lock().unwrap().operand_stack.push(string_ref);
                self.return_one(verbose);
            }
            (
                "java/io/PrintStream",
                "println",
                MethodDescriptor {
                    parameter_size: 0, ..
                },
            ) => {
                println!();
                self.return_void();
            }
            (
                "java/io/PrintStream",
                "println",
                MethodDescriptor {
                    parameter_size: 1,
                    parameters,
                    ..
                },
            ) if matches!(&parameters[..], [FieldType::Object(_)]) => {
                // println!("{stackframe:?}");
                let arg = stackframe.lock().unwrap().locals[1];
                let heap_borrow = self.heap.lock().unwrap();
                let (to_string_class, to_string_method) =
                    AnyObj.get(&heap_borrow, arg as usize, |obj| {
                        obj.resolve_method(
                            &self.method_area,
                            &self.class_area,
                            "toString",
                            &MethodDescriptor {
                                parameter_size: 0,
                                parameters: Vec::new(),
                                return_type: Some(FieldType::Object(unsafe {
                                    native::STRING_CLASS.as_ref().unwrap().this.clone()
                                })),
                            },
                        )
                    })?;
                drop(heap_borrow);
                let stackframes = self.stack.len();
                // push a fake return address
                self.stack
                    .last_mut()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .operand_stack
                    .push(0);
                self.invoke_method(to_string_method, to_string_class);
                self.stack.last_mut().unwrap().lock().unwrap().locals[0] = arg;
                while self.stack.len() > stackframes {
                    self.tick(verbose)?;
                }
                let ret = stackframe.lock().unwrap().operand_stack.pop().unwrap();
                let str = StringObj.get(&self.heap.lock().unwrap(), ret as usize, Clone::clone)?;
                println!("{str}");
                self.return_void();
            }
            (
                "java/io/PrintStream",
                "println",
                MethodDescriptor {
                    parameter_size: 1,
                    parameters,
                    ..
                },
            ) => {
                // println!("{stackframe:?}");
                let arg = stackframe.lock().unwrap().locals[1];
                match parameters[0] {
                    FieldType::Float => {
                        println!("{}", f32::from_bits(arg));
                    }
                    FieldType::Int => {
                        println!("{}", arg as i32);
                    }
                    _ => {
                        return Err(format!(
                            "Unimplemented println argument type: {:?}",
                            parameters[0]
                        ))
                    }
                }
                self.return_void();
            }
            (class, name, method_type) => {
                return Err(format!(
                    "Error invoking native method; {class}.{name} {method_type:?}"
                ))
            }
        }
        Ok(())
    }

    /// # Panics
    pub fn return_void(&mut self) {
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
        let ret_value = old_stackframe.lock().unwrap().operand_stack.pop().unwrap();
        if verbose {
            println!("{ret_value}");
        }
        let stackframe = self.stack.last().unwrap();
        let ret_address = stackframe.lock().unwrap().operand_stack.pop().unwrap();
        self.pc_register = ret_address as usize;
        stackframe.lock().unwrap().operand_stack.push(ret_value);
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
    }
}

fn allocate_multi_array(
    class_area: &[Arc<Class>],
    heap: &mut Vec<Arc<Mutex<Object>>>,
    depth: &[u32],
    arr_type: FieldType,
) -> Result<u32, String> {
    match depth {
        [size] => Ok(heap_allocate(
            heap,
            if arr_type.get_size() == 2 {
                Array2::new(class_area, *size as usize, arr_type)
            } else {
                Array1::new(class_area, *size as usize, arr_type)
            },
        )),
        [size, rest @ ..] => {
            let FieldType::Array(inner_type) = arr_type else {
                return Err(format!("Expected an array type; got {arr_type:?}"));
            };
            let current_array = (0..*size)
                .map(|_| allocate_multi_array(class_area, heap, rest, *inner_type.clone()))
                .collect::<Result<Vec<_>, String>>()?;
            Ok(heap_allocate(
                heap,
                Array1::from_vec(class_area, current_array, *inner_type),
            ))
        }
        [] => Err(String::from("Can't create 0-dimensional array")),
    }
}

fn pop_long(stack: &mut Vec<u32>) -> Option<u64> {
    let lower = stack.pop()?;
    let upper = stack.pop()?;
    Some((upper as u64) << 32 | lower as u64)
}

fn push_long(stack: &mut Vec<u32>, l: u64) {
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

pub fn search_class_area(class_area: &[Arc<Class>], class: &str) -> Option<Arc<Class>> {
    for possible_class in class_area {
        if &*possible_class.this == class {
            return Some(possible_class.clone());
        }
    }
    None
}

pub fn heap_allocate(heap: &mut Vec<Arc<Mutex<Object>>>, element: Object) -> u32 {
    let length = heap.len();

    heap.push(Arc::new(Mutex::new(element)));

    length as u32
}
