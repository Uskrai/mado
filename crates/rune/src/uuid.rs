use rune::{ContextError, Module};

#[derive(rune::Any, Copy, Clone, Debug)]
pub struct Uuid {
    inner: mado_core::Uuid,
}

impl Uuid {
    pub fn parse_str(string: String) -> Uuid {
        let inner = mado_core::Uuid::parse_str(&string).unwrap();
        Self { inner }
    }
}

impl From<Uuid> for mado_core::Uuid {
    fn from(v: Uuid) -> Self {
        v.inner
    }
}

pub fn load_module() -> Result<Module, ContextError> {
    mado_rune_macros::register_module! {
      (Uuid) => {
        associated => {
          parse_str
        }
      }
    }

    load_module_with(Module::with_crate("mado"))
}
