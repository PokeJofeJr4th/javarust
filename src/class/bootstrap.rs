use std::{fmt::Write, sync::Arc};

use crate::virtual_machine::{
    error,
    object::{Object, ObjectFinder as _, StringObj},
    thread::stacking::Stack,
    Thread,
};

use super::{Constant, FieldType, MethodHandle};

#[derive(Debug)]
pub enum BootstrapRunner {
    MakeConcatWithConsts {
        template: String,
        parameters: Vec<FieldType>,
        parameter_size: usize,
    },
    LambdaMetaFactory {
        captures_length: usize,
        lambda_interface: Arc<str>,
    },
    Todo {
        method_handle: String,
        args: String,
    },
}

impl BootstrapRunner {
    /// # Panics
    /// # Errors
    #[allow(clippy::too_many_lines)]
    pub fn run(&self, thread: &mut Thread, verbose: bool) -> error::Result<()> {
        match self {
            Self::MakeConcatWithConsts {
                template,
                parameters,
                parameter_size,
            } => {
                let mut output = String::new();
                let mut parameters_iter = parameters.iter();
                let mut args_iter = (0..*parameter_size)
                    .map(|_| thread.stackframe.operand_stack.pop().unwrap())
                    .collect::<Vec<_>>();

                for c in template.chars() {
                    if c != '\u{1}' {
                        output.push(c);
                        continue;
                    }
                    let Some(field_type) = parameters_iter.next() else {
                        return Err(format!("Not enough parameters for java/lang/invoke/StringConcatFactory.makeConcatWithConstants: {template:?} {parameters:?}").into());
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
                                let heap_borrow = thread.heap.lock().unwrap();
                                if verbose {
                                    println!("{value}");
                                }
                                StringObj::SELF.get(&heap_borrow, value as usize, |str| {
                                    write!(output, "{str}").map_err(|err| format!("{err:?}"))
                                }).unwrap()?;
                                drop(heap_borrow);
                            }
                            other @ (FieldType::Object(_) | FieldType::Array(_)) => return Err(format!("Unsupported item for java/lang/invoke/StringConcatFactory.makeConcatWithConstants: {other:?}").into()),
                            FieldType::Long | FieldType::Double => unreachable!(),
                        }
                    }
                }
                let heap_pointer = thread
                    .heap
                    .lock()
                    .unwrap()
                    .allocate_str(Arc::from(&*output));
                thread.stackframe.operand_stack.push(heap_pointer);
                thread.rember_temp(heap_pointer, verbose);
                if verbose {
                    println!("makeConcatWithConstants: {heap_pointer}");
                }
            }
            Self::LambdaMetaFactory {
                captures_length,
                lambda_interface,
            } => {
                let lambda_object = Object {
                    fields: (0..*captures_length)
                        .map(|_| thread.stackframe.operand_stack.pop().unwrap())
                        .rev()
                        .collect(),
                    native_fields: Vec::new(),
                    class: lambda_interface.clone(),
                };
                let idx = thread.heap.lock().unwrap().allocate(lambda_object);
                thread.stackframe.operand_stack.push(idx);
                thread.rember_temp(idx, verbose);
            }
            Self::Todo {
                method_handle,
                args,
            } => {
                return Err(
                    format!("Unimplemented Bootstrap Method: {method_handle} {args}").into(),
                )
            }
        }
        Ok(())
    }
}

#[must_use]
pub fn make_runner(method_handle: &MethodHandle, args: &[Constant]) -> BootstrapRunner {
    match (method_handle, &args[..]) {
        (method_handle, args) => BootstrapRunner::Todo {
            method_handle: format!("{method_handle:?}"),
            args: format!("{args:?}"),
        },
    }
}
