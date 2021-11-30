mod builder;
mod chapter_task;
mod de;
mod error;
mod error_impl;
mod function;
pub mod http;
pub mod json;
mod module;
mod regex;
mod rune;
mod selector;
mod send_value;
mod source_loader;
mod test;
mod uuid;

mod deserializer;

// rune std stuff
mod option;
mod result;
mod vec;

pub use self::rune::Rune;
pub use error::{BuildError, Error, RuneError, VmError};
pub use json::Json;
pub use module::WebsiteModule;
pub use send_value::{SendValue, SendValueKind};

pub use builder::{create_context, Build};
pub use source_loader::SourceLoader;

pub use de::{DeserializeResult, DeserializeValue};

pub fn load_modules(
    context: &mut ::rune::compile::Context,
) -> Result<(), ::rune::compile::ContextError> {
    context.install(&http::load_module()?)?;
    context.install(&json::load_module()?)?;
    context.install(&regex::load_module()?)?;
    context.install(&error::load_module()?)?;
    context.install(&uuid::load_module()?)?;
    context.install(&selector::load_module()?)?;
    context.install(&chapter_task::load_module()?)?;
    context.install(&test::load_module()?)?;

    // rune std stuff
    context.install(&result::load_module()?)?;
    context.install(&option::load_module()?)?;
    context.install(&vec::load_module()?)?;

    context.install(&rune_modules::test::module(true)?)?;
    context.install(&rune_modules::fmt::module(true)?)?;

    Ok(())
}
