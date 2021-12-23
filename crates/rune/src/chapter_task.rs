use mado_core::{ChapterImageInfo, ChapterInfo, ChapterTask};

use crate::DeserializeValue;

use rune::{runtime::VmError as RuneVmError, Value};

#[derive(serde::Deserialize, rune::Any)]
pub struct ImageTask {
    pub id: String,
    pub name: Option<String>,
}

#[derive(rune::Any)]
pub struct RuneChapterTask {
    inner: Box<dyn ChapterTask>,
}

impl RuneChapterTask {
    pub fn new(inner: Box<dyn ChapterTask>) -> Self {
        Self { inner }
    }

    pub fn add(&mut self, value: DeserializeValue<ChapterImageInfo>) -> Result<(), RuneVmError> {
        let value = value.get().map_err(RuneVmError::panic)?;
        self.inner.add(value);
        Ok(())
    }

    pub fn get_chapter_id(&self) -> String {
        self.inner.get_chapter_id().to_string()
    }
}

#[derive(Default, rune::Any)]
pub struct MockChapterTask(String, Vec<ChapterImageInfo>);

impl MockChapterTask {
    pub fn new(value: DeserializeValue<ChapterInfo>) -> Result<Self, RuneVmError> {
        let value = value.get().map_err(RuneVmError::panic)?;
        Ok(Self(value.id, Vec::new()))
    }

    pub fn add(&mut self, value: DeserializeValue<ChapterImageInfo>) -> Result<(), RuneVmError> {
        let value = value.get().map_err(RuneVmError::panic)?;
        self.1.push(value);
        Ok(())
    }

    pub fn get_chapter_id(&self) -> String {
        self.0.clone()
    }

    pub fn get_image_info_at(&self, idx: usize) -> Result<Value, RuneVmError> {
        crate::serializer::ValueSerializer::to_value(self.1[idx].clone())
    }
}

pub fn load_module() -> Result<rune::Module, rune::ContextError> {
    mado_rune_macros::register_module! {
      (RuneChapterTask) => {
        inst => {
          add, get_chapter_id
        }
      },
      (MockChapterTask) => {
        associated => {
            new
        }
        inst => {
            add, get_image_info_at, get_chapter_id
        }
      }
    }

    load_module_with(rune::Module::with_crate("mado"))
}
