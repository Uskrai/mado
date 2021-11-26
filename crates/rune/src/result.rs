use rune::{
  runtime::{Function, Shared, Value, VmError},
  ContextError, Module,
};

pub fn load_module() -> Result<Module, ContextError> {
  let mut module = Module::with_crate_item("std", &["result"]);
  module.inst_fn("or_else", or_else)?;
  module.inst_fn("or", or)?;

  Ok(module)
}

fn or_else(
  result: Result<Value, Value>,
  then: Function,
) -> Result<Value, VmError> {
  match result {
    Ok(_) => Ok(Value::Result(Shared::new(result))),
    Err(_) => then.call(()),
  }
}

fn or(
  result: Result<Value, Value>,
  then: Result<Value, Value>,
) -> Result<Value, VmError> {
  match result {
    Ok(_) => Ok(Value::Result(Shared::new(result))),
    Err(_) => Ok(Shared::new(then).into()),
  }
}
