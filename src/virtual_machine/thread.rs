use std::{cmp::Ordering, fmt::Write, sync::Arc};

use crate::{
    class::{BootstrapMethod, Class, Constant, FieldType, Method, MethodDescriptor, MethodHandle},
    data::{Heap, SharedClassArea, SharedHeap, SharedMethodArea, NULL},
    virtual_machine::object::LambdaOverride,
};

use super::{
    error,
    instruction::Type,
    object::{AnyObj, Array1, Array2, ArrayType, Object, ObjectFinder, StringObj},
    Cmp, Instruction, Op, StackFrame,
};

pub mod stacking;

use stacking::Stack;

pub struct Thread {
    pub pc_register: usize,
    pub stack: Vec<StackFrame>,
    pub stackframe: StackFrame,
    pub method_area: SharedMethodArea,
    pub class_area: SharedClassArea,
    pub heap: SharedHeap,
}

macro_rules! stack {
    ($stack: expr => [$($before:ident),*] => [$($after:ident),*]) => {
        $(
            let $before = $stack.pop().unwrap();
        )*
        $stack.extend([$($after),*]);
    };
}

impl Thread {
    #[allow(clippy::too_many_lines, clippy::cognitive_complexity)]
    /// # Panics
    /// # Errors
    pub fn tick(&mut self, verbose: bool) -> super::error::Result<()> {
        // this way we can mutate the stack frame without angering the borrow checker
        let method = self.stackframe.method.clone();
        if let Some(native_method) = method.code.as_native() {
            // return self.invoke_native(&stackframe, verbose);
            return native_method.run(self, verbose);
        }
        let opcode = self.get_pc_byte();
        if verbose {
            println!("{opcode:?}");
        }
        match opcode {
            Instruction::Noop => {
                // nope
            }
            Instruction::Push1(i) => {
                // push one item onto the operand stack
                self.stackframe.operand_stack.push(i);
            }
            Instruction::Push2(a, b) => {
                // push 2 items onto the operand stack
                let operand_stack = &mut self.stackframe.operand_stack;
                operand_stack.push(a);
                operand_stack.push(b);
            }
            Instruction::LoadString(str) => {
                let str_ptr = self.heap.lock().unwrap().allocate_str(str);
                // never forget a static string
                self.rember(str_ptr, verbose);
                self.stackframe.operand_stack.push(str_ptr);
            }
            Instruction::Load2(index) => {
                // load a double from locals to stack
                long_load(&mut self.stackframe, index);
            }
            Instruction::Load1(index) => {
                // load one item from locals to stack
                value_load(&mut self.stackframe, index);
                if verbose {
                    println!("stack {:?}", self.stackframe.operand_stack);
                }
            }
            Instruction::Store2(index) => {
                // put two values into a local
                long_store(&mut self.stackframe, index);
            }
            Instruction::Store1(index) => {
                // put one reference into a local
                value_store(&mut self.stackframe, index);
                if verbose {
                    println!("locals {:?}", self.stackframe.locals);
                }
            }
            Instruction::Pop => {
                self.stackframe.operand_stack.pop();
            }
            Instruction::Pop2 => {
                self.stackframe.operand_stack.pop();
                self.stackframe.operand_stack.pop();
            }
            Instruction::Dup => {
                stack!(self.stackframe.operand_stack => [a] => [a, a]);
            }
            Instruction::Dupx1 => {
                stack!(self.stackframe.operand_stack => [x, y] => [y, x, y]);
            }
            Instruction::Dupx2 => {
                stack!(self.stackframe.operand_stack => [x, y, z] => [z, x, y, z]);
            }
            Instruction::Dup2 => {
                stack!(self.stackframe.operand_stack => [x, y] => [x, y, x, y]);
            }
            Instruction::Dup2x1 => {
                stack!(self.stackframe.operand_stack => [x, y, z] => [y, z, x, y, z]);
            }
            Instruction::Dup2x2 => {
                stack!(self.stackframe.operand_stack => [w, x, y, z] => [y, z, w, x, y, z]);
            }
            Instruction::Swap => {
                stack!(self.stackframe.operand_stack => [x, y] => [y, x]);
            }
            Instruction::IOp(Op::Add) => {
                // iadd
                // int add
                let rhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let result = lhs.wrapping_add(rhs);
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Add) => {
                // ladd
                // long add
                let rhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let result = lhs.wrapping_add(rhs);
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::FOp(Op::Add) => {
                // fadd
                // float add
                let rhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let result = lhs + rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::DOp(Op::Add) => {
                // dadd
                // double add
                let rhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let sum = rhs + lhs;
                self.stackframe.operand_stack.pushd(sum);
            }
            Instruction::IOp(Op::Sub) => {
                // isub
                // int subtract
                let rhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let result = lhs.wrapping_sub(rhs);
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Sub) => {
                // lsub
                // long subtract
                let rhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let result = lhs.wrapping_sub(rhs);
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::FOp(Op::Sub) => {
                // fsub
                // float sub
                let rhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let result = lhs - rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::DOp(Op::Sub) => {
                // dsub
                // double subtraction
                let rhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let result = lhs - rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::Mul) => {
                // imul
                // int multiply
                let rhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let result = lhs.wrapping_mul(rhs);
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Mul) => {
                // lmul
                // long multiply
                let rhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let result = lhs.wrapping_mul(rhs);
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::FOp(Op::Mul) => {
                // fmul
                // float mul
                let rhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let result = lhs * rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::DOp(Op::Mul) => {
                // dmul
                // double multiplication
                let rhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let result = lhs * rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::Div) => {
                // idiv
                // int divide
                let rhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                // TODO: Check for zero division
                if rhs == 0 {
                    todo!("throw new ArithmeticException");
                }
                let result = lhs / rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Div) => {
                // ldiv
                // long division

                let rhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                // TODO: Check for zero division
                if rhs == 0 {
                    let exception = Object::from_class(
                        &self
                            .class_area
                            .search("java/lang/ArithmeticException")
                            .unwrap(),
                    );
                    self.throw_obj(exception, verbose)?;
                    return Ok(());
                }
                let result = lhs / rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::FOp(Op::Div) => {
                // fdiv
                // float div
                let rhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let result = lhs / rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::DOp(Op::Div) => {
                // ddiv
                // double division
                let rhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let result = lhs / rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::Mod) => {
                // irem
                // int remainder
                let rhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                // TODO: Check for zero division
                if rhs == 0 {
                    todo!("throw new ArithmeticException");
                }
                let result = lhs % rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Mod) => {
                // lrem
                // long modulo

                let rhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                // TODO: Check for zero division
                if rhs == 0 {
                    todo!("throw new ArithmeticException");
                }
                let result = lhs % rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::FOp(Op::Mod) => {
                // frem
                // float rem
                let rhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let result = lhs % rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::DOp(Op::Mod) => {
                // drem
                // double remainder
                let rhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let result = lhs % rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::Neg) => {
                // ineg
                // negate int
                let f = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let result = -f;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Neg) => {
                // lneg
                // negate long
                let l = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let result = -l;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::FOp(Op::Neg) => {
                // fneg
                // negate float
                let f = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let result = -f;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::DOp(Op::Neg) => {
                // dneg
                // negate double
                let d = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let result = -d;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::Shl) => {
                // ishl
                // int shift left
                let rhs = self.stackframe.operand_stack.pop().unwrap() & 0x1F;
                let lhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let result = lhs << rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Shl) => {
                // lshl
                // long shift left
                let rhs = self.stackframe.operand_stack.pop().unwrap() & 0x3F;
                let lhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let result = lhs << rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::Shr) => {
                // ishr
                // int shift right
                let rhs = self.stackframe.operand_stack.pop().unwrap() & 0x1F;
                let lhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let result = lhs >> rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Shr) => {
                // lshr
                // long shift right
                let rhs = self.stackframe.operand_stack.pop().unwrap() & 0x3F;
                let lhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let result = lhs >> rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::Ushr) => {
                // iushr
                // int logical shift right
                let rhs = (self.stackframe.operand_stack.pop().unwrap()) & 0x1F;
                let lhs = self.stackframe.operand_stack.pop().unwrap();
                let result = lhs >> rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Ushr) => {
                // lushr
                // long logical shift right
                let rhs = self.stackframe.operand_stack.pop().unwrap() & 0x3F;
                let lhs = self.stackframe.operand_stack.popd::<u64>().unwrap();
                let result = lhs >> rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::And) => {
                // iand
                // int boolean and
                let rhs = self.stackframe.operand_stack.pop().unwrap();
                let lhs = self.stackframe.operand_stack.pop().unwrap();
                let result = lhs & rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::And) => {
                // land
                // long boolean and
                let rhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let result = lhs & rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::Or) => {
                // ior
                // int boolean or
                let rhs = self.stackframe.operand_stack.pop().unwrap();
                let lhs = self.stackframe.operand_stack.pop().unwrap();
                let result = lhs | rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Or) => {
                // lor
                // long boolean or
                let rhs = self.stackframe.operand_stack.popd::<u64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<u64>().unwrap();
                let result = lhs | rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IOp(Op::Xor) => {
                // ixor
                // int boolean xor
                let rhs = self.stackframe.operand_stack.pop().unwrap();
                let lhs = self.stackframe.operand_stack.pop().unwrap();
                let result = lhs ^ rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::LOp(Op::Xor) => {
                // lxor
                // long boolean xor
                let rhs = self.stackframe.operand_stack.popd::<u64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<u64>().unwrap();
                let result = lhs ^ rhs;
                self.stackframe.operand_stack.pushd(result);
            }
            Instruction::IInc(index, inc) => {
                // iinc
                // int increment
                self.stackframe.locals[index] =
                    (self.stackframe.locals[index] as i32).wrapping_add(inc) as u32;
            }
            Instruction::Convert(Type::Int, Type::Long) => {
                // i2l
                // int to long
                let int = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let long = int as i64;
                self.stackframe.operand_stack.pushd(long);
            }
            Instruction::Convert(Type::Int, Type::Float) => {
                // i2f
                // int to float
                let int = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let float = int as f32;
                self.stackframe.operand_stack.pushd(float);
            }
            Instruction::Convert(Type::Int, Type::Double) => {
                // i2d
                // int to double
                let int = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let double = int as f64;
                self.stackframe.operand_stack.pushd(double);
            }
            Instruction::Convert(Type::Long, Type::Int) => {
                // l2i
                // long to int
                let long = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let int = long as i32;
                self.stackframe.operand_stack.pushd(int);
            }
            Instruction::Convert(Type::Long, Type::Float) => {
                // l2f
                // long to float
                let long = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let float = long as f32;
                self.stackframe.operand_stack.pushd(float);
            }
            Instruction::Convert(Type::Long, Type::Double) => {
                // l2d
                // long to double
                let long = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let double = long as f64;
                self.stackframe.operand_stack.pushd(double);
            }
            Instruction::Convert(Type::Float, Type::Int) => {
                // f2i
                // float to integer
                let float = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let int = float as i32;
                self.stackframe.operand_stack.pushd(int);
            }
            Instruction::Convert(Type::Float, Type::Long) => {
                // f2l
                // float to long
                let float = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let long = float as i64;
                self.stackframe.operand_stack.pushd(long);
            }
            Instruction::Convert(Type::Float, Type::Double) => {
                // f2d
                // float to double
                let float = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let double = float as f64;
                self.stackframe.operand_stack.pushd(double);
            }
            Instruction::Convert(Type::Double, Type::Int) => {
                // d2i
                // double to integer
                let double = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let int = double as i32;
                self.stackframe.operand_stack.pushd(int);
            }
            Instruction::Convert(Type::Double, Type::Long) => {
                // d2l
                // double to long
                let double = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let int = double as i64;
                self.stackframe.operand_stack.pushd(int);
            }
            Instruction::Convert(Type::Double, Type::Float) => {
                // d2f
                // double to float
                let double = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let float = double as f32;
                self.stackframe.operand_stack.pushd(float);
            }
            Instruction::Convert(Type::Int, Type::Byte) => {
                // i2b
                // int to byte
                let int = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let byte = int as i8 as i32;
                self.stackframe.operand_stack.pushd(byte);
            }
            Instruction::Convert(Type::Int, Type::Char) => {
                // i2c
                // int to char
                let int = self.stackframe.operand_stack.popd::<u32>().unwrap();
                let char = int as u8 as u32;
                self.stackframe.operand_stack.pushd(char);
            }
            Instruction::Convert(Type::Int, Type::Short) => {
                // i2s
                // int to short
                let int = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let short = int as i16 as i32;
                self.stackframe.operand_stack.pushd(short);
            }
            Instruction::LCmp => {
                // lcmp
                // long comparison
                let rhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i64>().unwrap();
                let value = match lhs.cmp(&rhs) {
                    Ordering::Equal => 0,
                    Ordering::Greater => 1,
                    Ordering::Less => -1,
                };
                self.stackframe.operand_stack.pushd(value);
            }
            Instruction::FCmp(is_rev) => {
                // fcmp<op>
                // float comparison
                let rhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f32>().unwrap();
                let value = if lhs > rhs {
                    1
                } else if (lhs - rhs).abs() < f32::EPSILON {
                    0
                } else if lhs < rhs || is_rev {
                    -1
                } else {
                    1
                };
                self.stackframe.operand_stack.pushd(value);
            }
            Instruction::DCmp(is_rev) => {
                // dcmp<op>
                // double comparison
                let rhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<f64>().unwrap();
                let value = if lhs > rhs {
                    1
                } else if (lhs - rhs).abs() < f64::EPSILON {
                    0
                } else if lhs < rhs || is_rev {
                    -1
                } else {
                    1
                };
                self.stackframe.operand_stack.pushd(value);
            }
            Instruction::IfCmpZ(cmp, branch) => {
                // if<cond>
                // integer comparison to zero
                let lhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
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
                let rhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
                let lhs = self.stackframe.operand_stack.popd::<i32>().unwrap();
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
                self.return_void()?;
            }
            Instruction::Return1 => {
                // return one thing
                self.return_one(verbose);
            }
            Instruction::PutStatic(class, name, field_type, index_lock) => {
                // putstatic
                // put a static field to a class
                let Some(class) = self.class_area.search(&class) else {
                    return Err(error::Error::class_resolution(&class));
                };

                if self.maybe_initialize_class(&class) {
                    return Ok(());
                }

                let &staticindex = index_lock.get_or_init(|| {
                    class
                        .statics
                        .iter()
                        .find(|(field, _)| field.name == name)
                        .ok_or_else(|| {
                            format!("Couldn't find static `{name}` on class `{}`", class.this)
                        })
                        .unwrap()
                        .1
                });
                if verbose {
                    println!("Putting Static {name} of {}", class.this);
                }
                let mut static_fields = class.static_data.lock().unwrap();

                if field_type.get_size() == 1 {
                    let value = self.stackframe.operand_stack.pop().unwrap();
                    if field_type.is_reference() {
                        self.forgor(static_fields[staticindex], verbose);
                        self.rember(value, verbose);
                    }
                    static_fields[staticindex] = value;
                    drop(static_fields);
                } else {
                    let lower = self.stackframe.operand_stack.pop().unwrap();
                    let upper = self.stackframe.operand_stack.pop().unwrap();
                    static_fields[staticindex] = upper;
                    static_fields[staticindex + 1] = lower;
                }
            }
            Instruction::GetStatic(class, name, field_type, staticindex) => {
                // getstatic
                // get a static field from a class
                let Some(class) = self.class_area.search(&class) else {
                    return Err(error::Error::class_resolution(&class));
                };

                if self.maybe_initialize_class(&class) {
                    return Ok(());
                }

                let &staticindex = staticindex.get_or_init(|| {
                    class
                        .statics
                        .iter()
                        .find(|(field, _)| field.name == name)
                        .ok_or_else(|| {
                            format!("Couldn't find static `{name}` on class `{}`", class.this)
                        })
                        .unwrap()
                        .1
                });
                if verbose {
                    println!("Getting Static {name} of {}", class.this);
                }
                let static_fields = class.static_data.lock().unwrap();

                if field_type.get_size() == 1 {
                    let value = static_fields[staticindex];
                    drop(static_fields);
                    if field_type.is_reference() {
                        self.rember_temp(value, verbose);
                    }
                    self.stackframe.operand_stack.push(value);
                } else {
                    let upper = static_fields[staticindex];
                    let lower = static_fields[staticindex + 1];
                    drop(static_fields);
                    self.stackframe.operand_stack.extend([upper, lower]);
                }
            }
            Instruction::GetField(Some(idx), _class, _name, field_type) => {
                // get a field from an object
                let object_index = self.stackframe.operand_stack.pop().unwrap();

                let rember =
                    AnyObj.inspect(&self.heap, object_index as usize, |object_borrow| {
                        if field_type.get_size() == 1 {
                            let value = object_borrow.fields[idx];
                            self.stackframe.operand_stack.push(value);
                            if field_type.is_reference() {
                                Some(value)
                            } else {
                                None
                            }
                        } else {
                            let upper = object_borrow.fields[idx];
                            let lower = object_borrow.fields[idx + 1];
                            self.stackframe.operand_stack.extend([upper, lower]);
                            None
                        }
                    })?;
                if let Some(value) = rember {
                    self.rember_temp(value, verbose);
                }
            }
            Instruction::PutField(Some(idx), _class, _name, field_type) => {
                // putfield
                // set a field in an object

                let value = if field_type.get_size() == 1 {
                    self.stackframe.operand_stack.pop().unwrap() as u64
                } else {
                    self.stackframe.operand_stack.popd::<u64>().unwrap()
                };
                let object_index = self.stackframe.operand_stack.pop().unwrap();

                let forgor_rember = AnyObj.inspect(
                    &self.heap,
                    object_index as usize,
                    |object_borrow| -> Result<Option<(u32, u32)>, String> {
                        if verbose {
                            println!("Object class: {}", object_borrow.this_class());
                        }

                        // handle memory stuff outside of the closure so we don't deadlock
                        let forgor_rember = if field_type.is_reference() {
                            Some((object_borrow.fields[idx], value as u32))
                        } else {
                            None
                        };
                        if field_type.get_size() == 1 {
                            object_borrow.fields[idx] = value as u32;
                        } else {
                            object_borrow.fields[idx] = (value >> 32) as u32;
                            object_borrow.fields[idx + 1] = value as u32;
                        }
                        Ok(forgor_rember)
                    },
                )??;
                if let Some((forgor, rember)) = forgor_rember {
                    self.forgor(forgor, verbose);
                    self.rember(rember, verbose);
                }
            }
            Instruction::InvokeVirtual(_class, name, method_type)
            | Instruction::InvokeInterface(_class, name, method_type) => {
                // invokevirtual
                // invoke a method virtually I guess
                let arg_count = method_type.parameter_size;
                let obj_pointer = *self
                    .stackframe
                    .operand_stack
                    .iter()
                    .rev()
                    .nth(arg_count)
                    .unwrap();
                let (resolved_class, resolved_method) =
                    AnyObj.inspect(&self.heap, obj_pointer as usize, |obj| {
                        obj.resolve_method(
                            &self.method_area,
                            &self.class_area,
                            &name,
                            &method_type,
                            verbose,
                        )
                    })?;
                let args_start = self.stackframe.operand_stack.len() - arg_count - 1;
                if verbose {
                    println!(
                        "Args Start: {args_start}\nStack: {:?}",
                        self.stackframe.operand_stack
                    );
                }
                let stack = &mut self.stackframe.operand_stack;
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

                for (index, value) in stack_iter.enumerate() {
                    if verbose {
                        println!("new_locals[{index}]={value}");
                    }
                    self.stackframe.locals[index] = value;
                }
                if verbose {
                    println!("new locals: {:?}", self.stackframe.locals);
                }
            }
            Instruction::InvokeSpecial(class, name, method_type) => {
                // invoke an instance method
                let current_class = if &*name == "<init>" {
                    class
                } else {
                    self.class_area
                        .search(&class)
                        .ok_or_else(|| error::Error::class_resolution(&class))?
                        .super_class
                        .clone()
                };
                let (class_ref, method_ref) = self
                    .method_area
                    .search(&current_class, &name, &method_type)
                    .ok_or_else(|| format!("Error during InvokeSpecial; {current_class}.{name}"))?;
                let args_start =
                    self.stackframe.operand_stack.len() - method_type.parameter_size - 1;
                let stack = &mut self.stackframe.operand_stack;
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

                for (index, value) in stack_iter.enumerate() {
                    if verbose {
                        println!("new_locals[{index}]={value}");
                    }
                    self.stackframe.locals[index] = value;
                }
                if verbose {
                    println!("new locals: {:?}", self.stackframe.locals);
                }
            }
            Instruction::InvokeStatic(class, name, method_type, resolved_method) => {
                // make a static method
                let (class_ref, method_ref) = resolved_method
                    .get_or_init(|| {
                        self.method_area
                            .search(&class, &name, &method_type)
                            .ok_or_else(|| {
                                format!(
                                    "Error during InvokeStatic; {class}.{name}: {method_type:?}"
                                )
                            })
                            .unwrap()
                    })
                    .clone();

                if self.maybe_initialize_class(&class_ref) {
                    return Ok(());
                }

                if verbose {
                    println!(
                        "Invoking Static Method {} on {}",
                        method_ref.name, class_ref.this,
                    );
                }
                let args_start = self.stackframe.operand_stack.len() - method_type.parameter_size;
                let stack = &mut self.stackframe.operand_stack;
                let mut stack_iter = core::mem::take(stack).into_iter();
                stack.extend((&mut stack_iter).take(args_start));
                stack.push(self.pc_register as u32);

                self.invoke_method(method_ref, class_ref);
                for (index, value) in stack_iter.enumerate() {
                    if verbose {
                        println!("new_locals[{index}]={value}");
                    }
                    self.stackframe.locals[index] = value;
                }
                if verbose {
                    println!("new locals: {:?}", self.stackframe.locals);
                }
            }
            Instruction::InvokeDynamic(bootstrap_index, method_name, method_type) => {
                let bootstrap_method =
                    self.stackframe.class.bootstrap_methods[bootstrap_index as usize].clone();
                self.invoke_dynamic(
                    &method_name,
                    bootstrap_method,
                    method_type,
                    bootstrap_index,
                    verbose,
                )?;
            }
            Instruction::New(class, class_lock) => {
                // make a new object instance
                let class = class_lock.get_or_init(|| self.class_area.search(&class).unwrap());
                if self.maybe_initialize_class(class) {
                    return Ok(());
                }
                let objectref = self
                    .heap
                    .lock()
                    .unwrap()
                    .allocate(Object::from_class(class));
                self.stackframe.operand_stack.push(objectref);
                self.rember_temp(objectref, verbose);
            }
            Instruction::IfNull(is_rev, branch) => {
                // ifnull | ifnonnull
                let ptr = self.stackframe.operand_stack.pop().unwrap();
                if (ptr == NULL) ^ (is_rev) {
                    self.pc_register = branch as usize;
                }
            }
            Instruction::NewArray1(field_type) => {
                // {ty}newarray
                let count = self.stackframe.operand_stack.pop().unwrap();
                let new_array = Array1::new(count as usize, field_type);
                let objectref = self.heap.lock().unwrap().allocate(new_array);
                self.stackframe.operand_stack.push(objectref);
                self.rember_temp(objectref, verbose);
            }
            Instruction::NewArray2(field_type) => {
                // {ty}newarray
                let count = self.stackframe.operand_stack.pop().unwrap();
                let new_array = Array2::new(count as usize, field_type);
                let objectref = self.heap.lock().unwrap().allocate(new_array);
                self.stackframe.operand_stack.push(objectref);
                self.rember_temp(objectref, verbose);
            }
            Instruction::ArrayStore1 => {
                // store 1 value into an array
                let value = self.stackframe.operand_stack.pop().unwrap();
                let index = self.stackframe.operand_stack.pop().unwrap();
                let array_ref = self.stackframe.operand_stack.pop().unwrap();

                let (old, is_reference) =
                    Array1.inspect(&self.heap, array_ref as usize, |arr| {
                        let old = arr.contents[index as usize];
                        arr.contents[index as usize] = value;
                        (old, arr.arr_type.is_reference())
                    })?;
                if is_reference {
                    // if it's a reference type, increment the ref count
                    self.rember(value, verbose);
                    self.forgor(old, verbose);
                }
            }
            Instruction::ArrayStore2 => {
                // store 2 values into an array
                let value = self.stackframe.operand_stack.popd::<u64>().unwrap();
                let index = self.stackframe.operand_stack.pop().unwrap();
                let array_ref = self.stackframe.operand_stack.pop().unwrap();

                Array2.inspect(&self.heap, array_ref as usize, |arr| {
                    arr.contents[index as usize] = value;
                })?;
            }
            Instruction::ArrayLoad1 => {
                // load 1 value from an array
                let index = self.stackframe.operand_stack.pop().unwrap();
                let array_ref = self.stackframe.operand_stack.pop().unwrap();

                let value = Array1.inspect(&self.heap, array_ref as usize, |arr| {
                    arr.contents[index as usize]
                })?;
                self.stackframe.operand_stack.push(value);
                if ArrayType::SELF.inspect(&self.heap, array_ref as usize, |f| f.is_reference())? {
                    self.rember_temp(value, verbose);
                }
            }
            Instruction::ArrayLoad2 => {
                // load 2 values from an array
                let index = self.stackframe.operand_stack.pop().unwrap();
                let array_ref = self.stackframe.operand_stack.pop().unwrap();

                let value = Array2.inspect(&self.heap, array_ref as usize, |arr| {
                    arr.contents[index as usize]
                })?;
                self.stackframe.operand_stack.pushd(value);
            }
            Instruction::NewMultiArray(dimensions, arr_type) => {
                let dimension_sizes = (0..dimensions)
                    .map(|_| self.stackframe.operand_stack.pop().unwrap())
                    .collect::<Vec<_>>();
                // make all the thingies
                let allocation = allocate_multi_array(
                    &mut self.heap.lock().unwrap(),
                    &dimension_sizes,
                    arr_type,
                )?;
                self.stackframe.operand_stack.push(allocation);
                self.rember_temp(allocation, verbose);
            }
            Instruction::ArrayLength => {
                let arr_ref = self.stackframe.operand_stack.pop().unwrap() as usize;
                let length = Array1.inspect(&self.heap, arr_ref, |arr| arr.contents.len())? as u32;
                self.stackframe.operand_stack.push(length);
            }
            Instruction::CheckedCast(ty) => {
                let objref = *self.stackframe.operand_stack.last().unwrap();
                if objref != NULL {
                    let obj_works = AnyObj
                        .inspect(&self.heap, objref as usize, |o| {
                            o.isinstance(&self.class_area, &ty, verbose)
                        })
                        .is_ok_and(|x| x);
                    if !obj_works {
                        let obj_type =
                            AnyObj.inspect(&self.heap, objref as usize, |o| o.this_class())?;
                        return Err(format!(
                            "CheckedCast failed; expected a(n) {ty} but got a(n) {obj_type}"
                        )
                        .into());
                    }
                }
            }
            Instruction::AThrow => {
                let objref = self.stackframe.operand_stack.pop().unwrap();
                self.throw(objref, verbose)?;
            }
            other => return Err(format!("Invalid Opcode: {other:?}").into()),
        }
        Ok(())
    }

    /// # Panics
    pub fn rember_temp(&mut self, value: u32, verbose: bool) {
        self.heap.lock().unwrap().inc_ref(value);
        self.stackframe.garbage.push(value);
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
    pub fn maybe_initialize_class(&mut self, class: &Class) -> bool {
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
        self.stackframe
            .operand_stack
            .push((self.pc_register - 1) as u32);
        self.invoke_method(method, class);
        true
    }

    fn get_code(&self, idx: usize) -> Instruction {
        self.stackframe.method.code.as_bytecode().unwrap().code[idx].clone()
    }

    fn get_pc_byte(&mut self) -> Instruction {
        let b = self.get_code(self.pc_register);
        self.pc_register += 1;
        b
    }

    /// first, resolve a concrete method for an object. Then, create a new stackframe for that method
    /// # Errors
    pub fn resolve_and_invoke(
        &mut self,
        this_index: u32,
        method: &str,
        descriptor: &MethodDescriptor,
        verbose: bool,
    ) -> error::Result<()> {
        let (resolved_class, resolved_method) =
            AnyObj.inspect(&self.heap, this_index as usize, |o| {
                o.resolve_method(
                    &self.method_area,
                    &self.class_area,
                    method,
                    descriptor,
                    verbose,
                )
            })?;
        self.invoke_method(resolved_method, resolved_class);
        Ok(())
    }

    pub fn invoke_method(&mut self, method: Arc<Method>, class: Arc<Class>) {
        // create a new stackframe for the callee
        let stackframe = StackFrame::from_method(method, class);
        // add the caller to the stack and set the callee to active
        self.stack
            .push(core::mem::replace(&mut self.stackframe, stackframe));
        self.pc_register = 0;
    }

    fn throw_obj(&mut self, exception: Object, verbose: bool) -> Result<(), String> {
        let idx = self.heap.lock().unwrap().allocate(exception);
        self.throw(idx, verbose)
    }

    fn throw(&mut self, exception_ptr: u32, verbose: bool) -> Result<(), String> {
        loop {
            for entry in &self
                .stackframe
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
                            .inspect(&self.heap, exception_ptr as usize, |obj| {
                                obj.isinstance(&self.class_area, catch_type, verbose)
                            })
                            .is_ok_and(|a| a)
                    })
                {
                    if verbose {
                        println!("Found an exception handler! {entry:?}");
                    }
                    self.pc_register = entry.handler_pc as usize;
                    self.stackframe.operand_stack.push(exception_ptr);
                    return Ok(());
                }
            }
            if verbose {
                println!(
                    "No exception handlers found : {:?}",
                    self.stackframe.method.name
                );
            }
            match self.stack.pop() {
                Some(s) => self.stackframe = s,
                None => return Err(String::from("Exception propagated past main")),
            }
            self.pc_register = self.stackframe.operand_stack.pop().unwrap() as usize;
        }
    }

    #[allow(clippy::too_many_lines)]
    fn invoke_dynamic(
        &mut self,
        method_name: &str,
        method_handle: BootstrapMethod,
        method_descriptor: MethodDescriptor,
        _callsite_number: u16,
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
                let mut args_iter = (0..parameter_size)
                    .map(|_| self.stackframe.operand_stack.pop().unwrap())
                    .collect::<Vec<_>>();

                for c in str.chars() {
                    if c != '\u{1}' {
                        output.push(c);
                        continue;
                    }
                    let Some(field_type) = parameters_iter.next() else {
                        return Err(format!("Not enough parameters for java/lang/invoke/StringConcatFactory.makeConcatWithConstants: {str:?} {parameters:?}"));
                    };
                    if field_type.get_size() == 2 {
                        let value = args_iter.popd::<u64>().unwrap();
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
                if verbose {
                    println!("{value}");
                }
                StringObj::SELF.inspect(&self.heap, value as usize, |str| {
                    write!(output, "{str}").map_err(|err| format!("{err:?}"))
                }).unwrap()?;
                            }
                            other => return Err(format!("Unsupported item for java/lang/invoke/StringConcatFactory.makeConcatWithConstants: {other:?}")),
                        }
                    }
                }
                let heap_pointer = self.heap.lock().unwrap().allocate_str(Arc::from(&*output));
                self.stackframe.operand_stack.push(heap_pointer);
                self.rember_temp(heap_pointer, verbose);
                if verbose {
                    println!("makeConcatWithConstants: {heap_pointer}");
                }
            }
            (
                method_name,
                BootstrapMethod {
                    method:
                        MethodHandle::InvokeStatic {
                            class: bootstrap_class,
                            name: bootstrap_name,
                            method_type: _,
                        },
                    args,
                },
                MethodDescriptor {
                    parameter_size,
                    parameters,
                    return_type,
                },
            ) if &*bootstrap_name == "metafactory"
                && &*bootstrap_class == "java/lang/invoke/LambdaMetafactory" =>
            {
                if verbose {
                    println!("LambdaMetafactory:\nMethod name: {method_name}\nBootstrap Arguments: {args:#?}\nMethod Descriptor: {return_type:?} {parameters:?}");
                }
                let Some(FieldType::Object(lambda_class)) = return_type else {
                    return Err(format!(
                        "LambdaMetaFactory expects an object return; got {return_type:?}"
                    ));
                };
                let [Constant::MethodType(interface_descriptor), Constant::MethodHandle(method_handle), Constant::MethodType(_enforced_type)] =
                    &args[..]
                else {
                    return Err("Wrong parameters for LambdaMetaFactory".to_string());
                };
                let lambda_object = Object {
                    fields: Vec::new(),
                    native_fields: vec![Box::new(LambdaOverride {
                        method_name: Arc::from(method_name),
                        method_descriptor: interface_descriptor.clone(),
                        invoke: method_handle.clone(),
                        captures: (0..parameter_size)
                            .map(|_| self.stackframe.operand_stack.pop().unwrap())
                            .rev()
                            .collect(),
                    })],
                    class: lambda_class,
                };
                let lambda_index = self.heap.lock().unwrap().allocate(lambda_object);
                self.rember_temp(lambda_index, verbose);
                self.stackframe.operand_stack.push(lambda_index);
            }
            (n, h, d) => return Err(format!("Error during InvokeDynamic: {n}: {d:?}; {h:?}")),
        }
        Ok(())
    }

    fn collect_garbage(&mut self) {
        let mut heap_borrow = self.heap.lock().unwrap();
        for ptr in core::mem::take(&mut self.stackframe.garbage) {
            // println!("Collecting Garbage {ptr}");
            heap_borrow.dec_ref(ptr);
        }
    }

    /// # Panics
    /// # Errors
    pub fn return_void(&mut self) -> error::Result<()> {
        self.collect_garbage();
        let Some(next_method) = self.stack.pop() else {
            return Err(error::Error::ThreadKill);
        };
        self.stackframe = next_method;
        let return_address = self.stackframe.operand_stack.pop().unwrap();
        self.pc_register = return_address as usize;
        Ok(())
    }

    /// # Panics
    pub fn return_one(&mut self, verbose: bool) {
        // outer_stackframe is the calling method and self.stackframe is the method that was called
        let mut outer_stackframe = self.stack.pop().unwrap();
        if verbose {
            println!(
                "ret1 from {}.{}",
                self.stackframe.class.this, self.stackframe.method.name
            );
        }
        let is_reference = self
            .stackframe
            .method
            .descriptor
            .return_type
            .as_ref()
            .unwrap()
            .is_reference();
        let ret_value = self.stackframe.operand_stack.pop().unwrap();
        if verbose {
            println!("{ret_value}");
        }
        // now self.stackframe is the calling method and outer_stackframe is the method that was called
        core::mem::swap(&mut outer_stackframe, &mut self.stackframe);
        if is_reference {
            self.rember_temp(ret_value, verbose);
        }
        let ret_address = self.stackframe.operand_stack.pop().unwrap();
        self.pc_register = ret_address as usize;
        self.stackframe.operand_stack.push(ret_value);
        // now self.stackframe is the method that was called and outer_stackframe is the calling method
        core::mem::swap(&mut outer_stackframe, &mut self.stackframe);
        self.collect_garbage();
        // now self.stackframe is the calling method and outer_stackframe is the method that was called
        core::mem::swap(&mut outer_stackframe, &mut self.stackframe);
        if verbose {
            println!("Stack: {:?}", self.stackframe.operand_stack);
        }
    }

    /// # Panics
    pub fn return_two(&mut self, verbose: bool) {
        let outer_stackframe = self.stack.pop().unwrap();
        let ret_value = self.stackframe.operand_stack.popd::<u64>().unwrap();
        if verbose {
            println!("{ret_value}");
        }
        self.collect_garbage();
        self.stackframe = outer_stackframe;
        let ret_address = self.stackframe.operand_stack.pop().unwrap();
        self.pc_register = ret_address as usize;
        self.stackframe.operand_stack.pushd(ret_value);
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

fn value_store(stackframe: &mut StackFrame, index: usize) {
    let value = stackframe.operand_stack.pop().unwrap();
    stackframe.locals[index] = value;
}

fn value_load(stackframe: &mut StackFrame, index: usize) {
    let value = stackframe.locals[index];
    stackframe.operand_stack.push(value);
}

fn long_store(stackframe: &mut StackFrame, index: usize) {
    let lower = stackframe.operand_stack.pop().unwrap();
    let upper = stackframe.operand_stack.pop().unwrap();
    stackframe.locals[index] = upper;
    stackframe.locals[index + 1] = lower;
}

fn long_load(stackframe: &mut StackFrame, index: usize) {
    let value_upper = stackframe.locals[index];
    let value_lower = stackframe.locals[index + 1];
    stackframe.operand_stack.extend([value_upper, value_lower]);
}
