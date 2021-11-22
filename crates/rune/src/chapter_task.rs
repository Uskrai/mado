/*
 *  Copyright (c) 2021 Uskrai
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use mado_core::{ChapterInfo, ChapterTask};

use crate::DeserializeValue;

use rune::runtime::VmError as RuneVmError;

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

  pub fn mock(
    value: DeserializeValue<ChapterInfo>,
  ) -> Result<Self, RuneVmError> {
    #[derive(Default)]
    pub struct TaskMock(ChapterInfo);
    impl ChapterTask for TaskMock {
      fn add(&mut self, _: Option<String>, _: String) {}
      fn get_chapter(&self) -> &ChapterInfo {
        &self.0
      }
    }

    let value = value.get().map_err(RuneVmError::panic)?;

    Ok(Self::new(Box::new(TaskMock(value))))
  }

  pub fn add(&mut self, name: Option<String>, id: String) {
    self.inner.add(name, id)
  }

  pub fn get_chapter_id(&self) -> String {
    self.inner.get_chapter().id.clone()
  }
}

pub fn load_module() -> Result<rune::Module, rune::ContextError> {
  mado_rune_macros::register_module! {
    (RuneChapterTask) => {
      inst => {
        add, get_chapter_id
      }
      associated => {
        mock
      }
    }
  }

  load_module_with(rune::Module::with_crate("mado"))
}
