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

use crate::function::RuneFunction;
use crate::DeserializeResult;
use crate::Rune;
use crate::SendValue;
use crate::VmError;

use super::http::Url;
use super::Error;

use async_trait::async_trait;
use mado_core::MangaInfo;
use mado_core::WebsiteModule as BaseWebsiteModule;
use rune::runtime::VmError as RuneVmError;
use rune::ToValue;

#[derive(Clone, Debug)]
pub struct WebsiteModule {
  rune: Rune,
  name: String,
  domain: Url,

  get_info: RuneFunction,
  get_chapter_images: RuneFunction,

  data: SendValue,
}

impl WebsiteModule {
  pub fn get_domain(&self) -> &Url {
    &self.domain
  }
}

impl WebsiteModule {
  async fn get_info(&self, url: Url) -> Result<MangaInfo, Error> {
    let fut = self
      .get_info
      .async_call::<_, DeserializeResult<_>>((self.data.clone(), url))
      .await;

    fut?.get()
  }
}

#[async_trait]
impl BaseWebsiteModule for WebsiteModule {
  async fn get_info(
    &self,
    url: mado_core::url::Url,
  ) -> Result<MangaInfo, mado_core::Error> {
    Ok(self.get_info(url.into()).await?)
  }
}

impl WebsiteModule {
  /// Retreive data
  pub fn data(&self) -> SendValue {
    self.data.clone()
  }

  pub fn name(&self) -> String {
    self.name.clone()
  }

  pub fn from_value(
    rune: crate::Rune,
    value: SendValue,
  ) -> Result<WebsiteModule, VmError> {
    let obj = rune.convert_result(value.into_object())?;

    let name = rune.from_value(obj["name"].clone())?;
    let domain = rune.from_value(obj["domain"].clone())?;

    let get_function = |name| {
      let fun = rune.convert_result(obj[name].clone().into_function())?;
      Ok(RuneFunction::new(rune.clone(), fun))
    };

    let get_info = get_function("get_info")?;
    let get_chapter_images = get_function("get_chapter_images")?;

    let data = obj.get("data").expect("cannot find data").clone();

    Ok(Self {
      rune,
      name,
      domain,
      get_info,
      get_chapter_images,
      data,
    })
  }

  pub fn from_value_vec(
    rune: crate::Rune,
    value: SendValue,
  ) -> Result<Vec<WebsiteModule>, VmError> {
    use super::SendValueKind as Kind;

    match value.kind_ref() {
      Kind::Vec(_) => {
        let v = rune.convert_result(value.into_vec())?;
        let mut vec = Vec::new();
        for it in v {
          vec.push(Self::from_value(rune.clone(), it)?);
        }
        Ok(vec)
      }

      Kind::Struct { .. } => Ok([Self::from_value(rune, value)?].to_vec()),
      _ => {
        let value = rune.convert_result(value.to_value())?;
        let type_info = rune.convert_result(value.type_info())?;
        let err = RuneVmError::expected::<rune::runtime::Vec>(type_info);

        Err(rune.convert_vm_error(err))
      }
    }
  }
}
