use std::{cell::RefCell, cmp::Ordering, rc::Rc};

use crate::{
    class::{Class, Method},
    virtual_machine::search_method_area,
};

use super::{instruction::Type, Cmp, HeapElement, Instruction, Object, Op, StackFrame};

pub(super) struct Thread {
    pub pc_register: usize,
    pub stack: Vec<Rc<RefCell<StackFrame>>>,
    pub method_area: Rc<Vec<(Rc<Class>, Rc<Method>)>>,
    pub class_area: Rc<Vec<Rc<Class>>>,
    pub heap: Rc<RefCell<Vec<Rc<RefCell<HeapElement>>>>>,
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
            Instruction::Noop => {
                // nop
            }
            Instruction::Push1(i) => {
                // push one item onto the operand stack
                stackframe.borrow_mut().operand_stack.push(i);
            }
            Instruction::Push2(a, b) => {
                // push 2 items onto the operand stack
                let operand_stack = &mut stackframe.borrow_mut().operand_stack;
                operand_stack.push(a);
                operand_stack.push(b);
            }
            Instruction::LoadString(str) => {
                let str_obj = String::from(&*str.clone());
                let str_ptr =
                    heap_allocate(&mut self.heap.borrow_mut(), HeapElement::String(str_obj));
                stackframe.borrow_mut().operand_stack.push(str_ptr);
            }
            Instruction::Load2(index) => {
                // load a double from locals to stack
                long_load(stackframe, index);
            }
            Instruction::Load1(index) => {
                // load one item from locals to stack
                value_load(stackframe, index);
            }
            Instruction::Store2(index) => {
                // put two values into a local
                long_store(stackframe, index);
            }
            Instruction::Store1(index) => {
                // put one reference into a local
                value_store(stackframe, index);
            }
            Instruction::Pop => {
                stackframe.borrow_mut().operand_stack.pop();
            }
            Instruction::Pop2 => {
                stackframe.borrow_mut().operand_stack.pop();
                stackframe.borrow_mut().operand_stack.pop();
            }
            Instruction::Dup => {
                // dup
                let value = *stackframe.borrow().operand_stack.last().unwrap();
                stackframe.borrow_mut().operand_stack.push(value);
            }
            Instruction::Dupx1 => {
                // xy => yxy
                let y = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let x = stackframe.borrow_mut().operand_stack.pop().unwrap();
                stackframe.borrow_mut().operand_stack.extend([y, x, y]);
            }
            Instruction::Dupx2 => {
                // xyz => zxyz
                let z = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let y = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let x = stackframe.borrow_mut().operand_stack.pop().unwrap();
                stackframe.borrow_mut().operand_stack.extend([z, x, y, z]);
            }
            Instruction::Dup2 => {
                // xy => xyxy
                let y = *stackframe.borrow().operand_stack.last().unwrap();
                let x = *stackframe.borrow().operand_stack.last().unwrap();
                stackframe.borrow_mut().operand_stack.extend([x, y]);
            }
            Instruction::Dup2x1 => {
                // xyz => yzxyz
                let z = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let y = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let x = stackframe.borrow_mut().operand_stack.pop().unwrap();
                stackframe
                    .borrow_mut()
                    .operand_stack
                    .extend([y, z, x, y, z]);
            }
            Instruction::Dup2x2 => {
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
            Instruction::Swap => {
                let x = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let y = stackframe.borrow_mut().operand_stack.pop().unwrap();
                stackframe.borrow_mut().operand_stack.push(x);
                stackframe.borrow_mut().operand_stack.push(y);
            }
            Instruction::IOp(Op::Add) => {
                // iadd
                // int add
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs.wrapping_add(rhs);
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Add) => {
                // ladd
                // long add
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs.wrapping_add(rhs);
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Add) => {
                // fadd
                // float add
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs + rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            Instruction::DOp(Op::Add) => {
                // dadd
                // double add
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let sum = rhs + lhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, sum.to_bits());
            }
            Instruction::IOp(Op::Sub) => {
                // isub
                // int subtract
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs.wrapping_sub(rhs);
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Sub) => {
                // lsub
                // long subtract
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs.wrapping_sub(rhs);
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Sub) => {
                // fsub
                // float sub
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs - rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            Instruction::DOp(Op::Sub) => {
                // dsub
                // double subtraction
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = lhs - rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            Instruction::IOp(Op::Mul) => {
                // imul
                // int multiply
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs.wrapping_mul(rhs);
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Mul) => {
                // lmul
                // long multiply
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs.wrapping_mul(rhs);
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Mul) => {
                // fmul
                // float mul
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs * rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            Instruction::DOp(Op::Mul) => {
                // dmul
                // double multiplication
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = lhs * rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            Instruction::IOp(Op::Div) => {
                // idiv
                // int divide
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                // TODO: Check for zero division
                let result = lhs / rhs;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Div) => {
                // ldiv
                // long division

                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                // TODO: Check for zero division
                let result = lhs / rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Div) => {
                // fdiv
                // float div
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs / rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            Instruction::DOp(Op::Div) => {
                // ddiv
                // double division
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = lhs / rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            Instruction::IOp(Op::Mod) => {
                // irem
                // int remainder
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                // TODO: Check for zero division
                let result = lhs % rhs;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Mod) => {
                // lrem
                // long modulo

                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                // TODO: Check for zero division
                let result = lhs % rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Mod) => {
                // frem
                // float rem
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = lhs % rhs;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            Instruction::DOp(Op::Mod) => {
                // drem
                // double remainder
                let rhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let lhs =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = lhs % rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            Instruction::IOp(Op::Neg) => {
                // ineg
                // negate int
                let f = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let result = -f;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Neg) => {
                // lneg
                // negate long
                let l = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = -l;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            Instruction::FOp(Op::Neg) => {
                // fneg
                // negate float
                let f = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let result = -f;
                stackframe.borrow_mut().operand_stack.push(result.to_bits());
            }
            Instruction::DOp(Op::Neg) => {
                // dneg
                // negate double
                let d =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let result = -d;
                push_long(&mut stackframe.borrow_mut().operand_stack, result.to_bits());
            }
            Instruction::IOp(Op::Shl) => {
                // ishl
                // int shift left
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) & 0x1F;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs << rhs;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Shl) => {
                // lshl
                // long shift left
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap() & 0x3F;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs << rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            Instruction::IOp(Op::Shr) => {
                // ishr
                // int shift right
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) & 0x1F;
                let lhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) as i32;
                let result = lhs >> rhs;
                stackframe.borrow_mut().operand_stack.push(result as u32);
            }
            Instruction::LOp(Op::Shr) => {
                // lshr
                // long shift right
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap() & 0x3F;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let result = lhs >> rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result as u64);
            }
            Instruction::IOp(Op::Ushr) => {
                // iushr
                // int logical shift right
                let rhs = (stackframe.borrow_mut().operand_stack.pop().unwrap()) & 0x1F;
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let result = lhs >> rhs;
                stackframe.borrow_mut().operand_stack.push(result);
            }
            Instruction::LOp(Op::Ushr) => {
                // lushr
                // long logical shift right
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap() & 0x3F;
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let result = lhs >> rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result);
            }
            Instruction::IOp(Op::And) => {
                // iand
                // int boolean and
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let result = lhs & rhs;
                stackframe.borrow_mut().operand_stack.push(result);
            }
            Instruction::LOp(Op::And) => {
                // land
                // long boolean and
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let result = lhs & rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result);
            }
            Instruction::IOp(Op::Or) => {
                // ior
                // int boolean or
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let result = lhs | rhs;
                stackframe.borrow_mut().operand_stack.push(result);
            }
            Instruction::LOp(Op::Or) => {
                // lor
                // long boolean or
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let result = lhs | rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result);
            }
            Instruction::IOp(Op::Xor) => {
                // ixor
                // int boolean xor
                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap();
                let result = lhs ^ rhs;
                stackframe.borrow_mut().operand_stack.push(result);
            }
            Instruction::LOp(Op::Xor) => {
                // lxor
                // long boolean xor
                let rhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let lhs = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let result = lhs ^ rhs;
                push_long(&mut stackframe.borrow_mut().operand_stack, result);
            }
            Instruction::IInc(index, inc) => {
                // iinc
                // int increment
                let start = stackframe.borrow().locals[index] as i32;
                stackframe.borrow_mut().locals[index] = start.wrapping_add(inc) as u32;
            }
            Instruction::Convert(Type::Int, Type::Long) => {
                // i2l
                // int to long
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let long = int as i64;
                push_long(&mut stackframe.borrow_mut().operand_stack, long as u64);
            }
            Instruction::Convert(Type::Int, Type::Float) => {
                // i2f
                // int to float
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let float = int as f32;
                stackframe.borrow_mut().operand_stack.push(float.to_bits());
            }
            Instruction::Convert(Type::Int, Type::Double) => {
                // i2d
                // int to double
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let double = int as f64;
                push_long(&mut stackframe.borrow_mut().operand_stack, double.to_bits());
            }
            Instruction::Convert(Type::Long, Type::Int) => {
                // l2i
                // long to int
                let long = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap();
                let int = long as u32;
                stackframe.borrow_mut().operand_stack.push(int);
            }
            Instruction::Convert(Type::Long, Type::Float) => {
                // l2f
                // long to float
                let long = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let float = long as f32;
                stackframe.borrow_mut().operand_stack.push(float.to_bits());
            }
            Instruction::Convert(Type::Long, Type::Double) => {
                // l2d
                // long to double
                let long = pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap() as i64;
                let double = long as f64;
                push_long(&mut stackframe.borrow_mut().operand_stack, double.to_bits());
            }
            Instruction::Convert(Type::Float, Type::Int) => {
                // f2i
                // float to integer
                let float = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let int = float as i32;
                stackframe.borrow_mut().operand_stack.push(int as u32);
            }
            Instruction::Convert(Type::Float, Type::Long) => {
                // f2l
                // float to long
                let float = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let long = float as u64;
                push_long(&mut stackframe.borrow_mut().operand_stack, long);
            }
            Instruction::Convert(Type::Float, Type::Double) => {
                // f2d
                // float to double
                let float = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let double = float as f64;
                push_long(&mut stackframe.borrow_mut().operand_stack, double.to_bits());
            }
            Instruction::Convert(Type::Double, Type::Int) => {
                // d2i
                // double to integer
                let double =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let int = double as u32;
                stackframe.borrow_mut().operand_stack.push(int);
            }
            Instruction::Convert(Type::Double, Type::Long) => {
                // d2l
                // double to long
                let double =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let int = double as u64;
                push_long(&mut stackframe.borrow_mut().operand_stack, int);
            }
            Instruction::Convert(Type::Double, Type::Float) => {
                // d2f
                // double to float
                let double =
                    f64::from_bits(pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap());
                let float = (double as f32).to_bits();
                stackframe.borrow_mut().operand_stack.push(float);
            }
            Instruction::Convert(Type::Int, Type::Byte) => {
                // i2b
                // int to byte
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let byte = int as i8 as i32;
                stackframe.borrow_mut().operand_stack.push(byte as u32);
            }
            Instruction::Convert(Type::Int, Type::Char) => {
                // i2c
                // int to char
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let char = int as u8;
                stackframe.borrow_mut().operand_stack.push(char as u32);
            }
            Instruction::Convert(Type::Int, Type::Short) => {
                // i2s
                // int to short
                let int = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let short = int as i16 as i32;
                stackframe.borrow_mut().operand_stack.push(short as u32);
            }
            Instruction::LCmp => {
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
            Instruction::FCmp(is_rev) => {
                // fcmp<op>
                // float comparison
                let rhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let lhs = f32::from_bits(stackframe.borrow_mut().operand_stack.pop().unwrap());
                let value = if lhs > rhs {
                    1
                } else if lhs == rhs {
                    0
                } else if lhs < rhs || is_rev {
                    -1
                } else {
                    1
                } as u32;
                stackframe.borrow_mut().operand_stack.push(value);
            }
            Instruction::DCmp(is_rev) => {
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
                } else if lhs < rhs || is_rev {
                    -1
                } else {
                    1
                } as u32;
                stackframe.borrow_mut().operand_stack.push(value);
            }
            Instruction::IfCmp(cmp, branch) => {
                // if<cond>
                // integer comparison to zero

                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let cond = match cmp {
                    Cmp::Eq => lhs == 0,
                    Cmp::Ne => lhs != 0,
                    Cmp::Lt => lhs < 0,
                    Cmp::Ge => lhs >= 0,
                    Cmp::Gt => lhs > 0,
                    Cmp::Le => lhs <= 0,
                };
                if cond {
                    self.pc_register += branch as usize;
                }
            }
            Instruction::ICmp(cnd, branch) => {
                // if_icmp<cond>
                // comparison between integers

                let rhs = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let lhs = stackframe.borrow_mut().operand_stack.pop().unwrap() as i32;
                let cond = match cnd {
                    Cmp::Eq => lhs == rhs,
                    Cmp::Ne => lhs != rhs,
                    Cmp::Lt => lhs < rhs,
                    Cmp::Ge => lhs >= rhs,
                    Cmp::Gt => lhs > rhs,
                    Cmp::Le => lhs <= rhs,
                };
                if cond {
                    self.pc_register += branch as usize;
                }
            }
            Instruction::Goto(goto) => {
                // goto bb1 bb2
                self.pc_register += goto as usize;
            }
            Instruction::Return0 => {
                // return
                // return void
                self.return_void();
            }
            Instruction::GetStatic(class, name, field_type) => {
                // getstatic
                // get a static field from a class
                let Some(class) = search_class_area(&self.class_area, class.clone()) else {
                    return Err(format!("Couldn't resolve class {class}"))
                };

                let staticindex = class
                    .statics
                    .iter()
                    .find(|(field, _)| field.name == name)
                    .ok_or_else(|| {
                        format!("Couldn't find static `{name}` on class `{}`", class.this)
                    })?
                    .1;
                println!("Getting Static {name} of {}", class.this);
                let static_fields = class.static_data.borrow();

                if field_type.get_size() == 1 {
                    let value = static_fields[staticindex];
                    stackframe.borrow_mut().operand_stack.push(value);
                } else {
                    let upper = static_fields[staticindex];
                    let lower = static_fields[staticindex + 1];
                    stackframe.borrow_mut().operand_stack.extend([upper, lower]);
                }
            }
            Instruction::GetField(class, name, field_type) => {
                // getfield
                // get a field from an object

                let Some(class) = search_class_area(&self.class_area, class.clone()) else {
                    return Err(format!("Couldn't resolve class {class}"))
                };

                let field_index = class
                    .fields
                    .iter()
                    .find(|(field, _)| field.name == name)
                    .ok_or_else(|| {
                        format!("Couldn't find field `{name}` on class `{}`", class.this)
                    })?
                    .1;
                let object_index = stackframe.borrow_mut().operand_stack.pop().unwrap();

                let object = self.heap.borrow()[object_index as usize].clone();

                let HeapElement::Object(ref mut object_borrow) = *object.borrow_mut() else {
                    return Err(String::from("Expected an object pointer"))
                };

                println!("Getting Field {name} of {}", class.this);
                let object_fields = object_borrow.class_mut_or_insert(class);

                if field_type.get_size() == 1 {
                    let value = object_fields[field_index];
                    stackframe.borrow_mut().operand_stack.push(value);
                } else {
                    let upper = object_fields[field_index];
                    let lower = object_fields[field_index + 1];
                    stackframe.borrow_mut().operand_stack.extend([upper, lower]);
                }
            }
            Instruction::PutField(class, name, field_type) => {
                // putfield
                // set a field in an object

                let Some(class) = search_class_area(&self.class_area, class.clone()) else {
                    return Err(format!("Couldn't resolve class {class}"))
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
                    stackframe.borrow_mut().operand_stack.pop().unwrap() as u64
                } else {
                    pop_long(&mut stackframe.borrow_mut().operand_stack).unwrap()
                };
                let object_index = stackframe.borrow_mut().operand_stack.pop().unwrap();

                let object = self.heap.borrow()[object_index as usize].clone();

                let HeapElement::Object(ref mut object_borrow) = *object.borrow_mut() else {
                    return Err(String::from("Expected an object pointer"))
                };

                let object_fields = object_borrow.class_mut_or_insert(class);

                if field_type.get_size() == 1 {
                    object_fields[field_index] = value as u32;
                } else {
                    object_fields[field_index] = (value >> 32) as u32;
                    object_fields[field_index + 1] = value as u32;
                }
            }
            Instruction::InvokeVirtual(class, name, method_type) => {
                // invokevirtual
                // invoke a method virtually I guess

                let (class_ref, method_ref) = search_method_area(
                    &self.method_area,
                    class.clone(),
                    name.clone(),
                    &method_type,
                )
                .ok_or_else(|| {
                    format!(
                        "Error during InvokeVirtual; {}.{} : {:?}",
                        class, name, method_type
                    )
                })?;
                let args_start =
                    stackframe.borrow().operand_stack.len() - method_type.parameter_size - 1;
                let stack = &mut stackframe.borrow_mut().operand_stack;
                let mut stack_iter = core::mem::take(stack).into_iter();
                stack.extend((&mut stack_iter).take(args_start));
                stack.push(self.pc_register as u32);

                println!(
                    "Invoking Virtual Method {} on {}",
                    method_ref.name, class_ref.this
                );
                self.invoke_method(method_ref, class_ref);
                self.pc_register = 0;

                let new_stackframe = self.stack.last().unwrap().clone();

                let new_locals = &mut new_stackframe.borrow_mut().locals;
                for (index, value) in stack_iter.enumerate() {
                    new_locals[index] = value;
                }
                println!("{new_locals:?}");
            }
            Instruction::InvokeSpecial(class, name, method_type) => {
                // invokespecial
                // invoke an instance method

                let (class_ref, method_ref) = search_method_area(
                    &self.method_area,
                    class.clone(),
                    name.clone(),
                    &method_type,
                )
                .ok_or_else(|| format!("Error during InvokeSpecial; {}.{}", class, name))?;
                let args_start =
                    stackframe.borrow().operand_stack.len() - method_type.parameter_size - 1;
                let stack = &mut stackframe.borrow_mut().operand_stack;
                let mut stack_iter = core::mem::take(stack).into_iter();
                stack.extend((&mut stack_iter).take(args_start));
                stack.push(self.pc_register as u32);

                let args = stack_iter.collect::<Vec<_>>();

                println!(
                    "Invoking Special Method {} on {}",
                    method_ref.name, class_ref.this
                );
                self.invoke_method(method_ref, class_ref);
                self.pc_register = 0;

                let new_stackframe = self.stack.last().unwrap().clone();

                new_stackframe.borrow_mut().operand_stack.extend(args);
            }
            Instruction::InvokeStatic(class, name, method_type) => {
                // invokestatic
                // make a static method
                let (class_ref, method_ref) = search_method_area(
                    &self.method_area,
                    class.clone(),
                    name.clone(),
                    &method_type,
                )
                .ok_or_else(|| format!("Error during InvokeStatic; {}.{}", class, name))?;
                println!(
                    "Invoking Static Method {} on {}",
                    method_ref.name, class_ref.this,
                );
                let args_start =
                    stackframe.borrow().operand_stack.len() - method_type.parameter_size;
                let stack = &mut stackframe.borrow_mut().operand_stack;
                let mut stack_iter = core::mem::take(stack).into_iter();
                stack.extend((&mut stack_iter).take(args_start));
                stack.push(self.pc_register as u32);

                let args = stack_iter.collect::<Vec<_>>();

                self.invoke_method(method_ref, class_ref);
                self.pc_register = 0;

                let new_stackframe = self.stack.last().unwrap().clone();

                new_stackframe.borrow_mut().operand_stack.extend(args);
            }
            Instruction::InvokeDynamic(bootstrap_index, method_name, method_type) => {
                // invokedynamic
                // dynamically figure out what to do

                let mut bootstrap_object = Object::new();
                let method_handle_class =
                    search_class_area(&self.class_area, "java/lang/invoke/MethodHandle".into())
                        .unwrap();
                bootstrap_object.class_mut_or_insert(method_handle_class)[0] =
                    bootstrap_index as u32;
                let bootstrap_pointer = heap_allocate(
                    &mut self.heap.borrow_mut(),
                    HeapElement::Object(bootstrap_object),
                );
                let bootstrap_method =
                    stackframe.borrow().class.bootstrap_methods[bootstrap_index as usize].clone();

                return Err(format!(
                    "Invoking Dynamic {bootstrap_method:?} - {method_name} : {method_type:?}"
                ));
                // TODO: Finish InvokeDynamic
            }
            Instruction::New(class) => {
                // new
                // make a new object instance

                let Some(class) = search_class_area(&self.class_area, class.clone()) else {
                    return Err(format!("Couldn't find class {class}"))
                };

                let mut new_object = Object::new();

                new_object.class_mut_or_insert(class);

                let objectref =
                    heap_allocate(&mut self.heap.borrow_mut(), HeapElement::Object(new_object));
                stackframe.borrow_mut().operand_stack.push(objectref);
            }
            Instruction::IfNull(is_rev, branch) => {
                // ifnull | ifnonnull
                let ptr = stackframe.borrow_mut().operand_stack.pop().unwrap();
                if (ptr == u32::MAX) ^ (is_rev) {
                    self.pc_register += branch as usize;
                }
            }
            other => return Err(format!("Invalid Opcode: 0x{other:?}")),
        }
        Ok(())
    }

    fn get_code(&self, stackframe: Rc<RefCell<StackFrame>>, idx: usize) -> Instruction {
        stackframe.borrow().method.code.as_ref().unwrap().code[idx].clone()
    }

    fn get_pc_byte(&mut self, stackframe: Rc<RefCell<StackFrame>>) -> Instruction {
        let b = self.get_code(stackframe, self.pc_register);
        self.pc_register += 1;
        b
    }

    pub fn invoke_method(&mut self, method: Rc<Method>, class: Rc<Class>) {
        let stackframe = StackFrame::from_method(method, class);
        self.stack.push(Rc::new(RefCell::new(stackframe)));
    }

    fn invoke_native(&mut self, stackframe: Rc<RefCell<StackFrame>>) -> Result<(), String> {
        let name = stackframe.borrow().method.name.clone();
        let class = stackframe.borrow().class.this.clone();
        match (&*class, &*name) {
            ("java/lang/Object", "<init>") => self.return_void(),
            ("java/io/PrintStream", "println") => {
                let args = stackframe.borrow().method.descriptor.parameter_size;
                if args == 0 {
                    println!();
                } else {
                    let arg = stackframe.borrow_mut().operand_stack.pop().unwrap();
                    let heap_borrow = self.heap.borrow();
                    let reference = heap_borrow.get(arg as usize);
                    match &*reference.unwrap().borrow() {
                        HeapElement::String(str) => println!("{str}"),
                        _ => todo!(),
                    }
                    drop(heap_borrow);
                }
                self.return_void();
            }
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

fn search_class_area(class_area: &[Rc<Class>], class: Rc<str>) -> Option<Rc<Class>> {
    for possible_class in class_area {
        if possible_class.this == class {
            return Some(possible_class.clone());
        }
    }
    None
}

pub(super) fn heap_allocate(heap: &mut Vec<Rc<RefCell<HeapElement>>>, element: HeapElement) -> u32 {
    let length = heap.len();

    heap.push(Rc::new(RefCell::new(element)));

    length as u32
}
