use rune::{ContextError, Module};

pub fn load_module() -> Result<Module, ContextError> {
  let mut module = Module::with_crate_item("std", &["vec"]);
  module.inst_fn("reverse", reverse)?;

  Ok(module)
}

pub fn reverse(vec: &mut rune::runtime::Vec) {
  vec.reverse();
}
