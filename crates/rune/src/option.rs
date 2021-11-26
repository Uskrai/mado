use rune::{
  runtime::{Function, Shared, Value, VmError},
  ContextError, Module,
};

pub fn load_module() -> Result<Module, ContextError> {
  let mut module = Module::with_crate_item("std", &["result"]);
  module.inst_fn("ok_or_else", ok_or_else)?;
  module.inst_fn("filter", filter)?;
  module.inst_fn("flatten", flatten)?;

  Ok(module)
}

fn ok_or_else(
  option: &Option<Value>,
  then: Function,
) -> Result<Value, VmError> {
  if let Some(v) = option {
    Ok(Value::Result(Shared::new(Ok(v.clone()))))
  } else {
    then.call(())
  }
}

fn filter(
  option: Option<Value>,
  predicate: Function,
) -> Result<Option<Value>, VmError> {
  Ok(match option {
    Some(v) => {
      if predicate.call((v.clone(),))? {
        Some(v)
      } else {
        None
      }
    }
    _ => None,
  })
}

fn flatten(option: Option<Value>) -> Result<Option<Value>, VmError> {
  Ok(match option {
    Some(v) => match v {
      Value::Option(v) => flatten(v.take()?)?,
      val => Some(val),
    },
    None => None,
  })
}
