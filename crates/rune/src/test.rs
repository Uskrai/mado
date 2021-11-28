use crate::DeserializeResult;
use mado_core::MangaInfo;
use rune::{ContextError, Module};

pub fn load_module() -> Result<Module, ContextError> {
    let mut module = Module::with_crate_item("mado", &["test"]);

    macro_rules! register_type {
        ($name:ident) => {
            module.function(
                &[stringify!($name)],
                |v: DeserializeResult<$name>| match v.get() {
                    Ok(_) => Ok(()),
                    Err(v) => return Err(rune::runtime::VmError::panic(v)),
                },
            )?;
        };
    }

    register_type!(MangaInfo);

    Ok(module)
}
//
