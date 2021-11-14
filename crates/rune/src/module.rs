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

use std::sync::Arc;

use crate::SendValue;

use super::http::Url;
use super::DeserializeResult;
use async_trait::async_trait;
use mado_core::MangaInfo;
use mado_core::WebsiteModule as BaseWebsiteModule;
use runestick::SyncFunction;

use super::Error;

#[derive(Clone)]
pub struct WebsiteModule {
  name: String,
  domain: Url,
  get_info: Arc<SyncFunction>,
  data: SendValue,
}

impl WebsiteModule {
  pub fn get_domain(&self) -> &Url {
    &self.domain
  }
}

impl TryFrom<SendValue> for WebsiteModule {
  type Error = Error;
  fn try_from(value: SendValue) -> Result<Self, Self::Error> {
    let obj = value.into_object()?;

    macro_rules! get_string {
      ($name:literal) => {
        obj
          .get($name)
          .expect(concat!($name, " doesn't exists"))
          .clone()
          .into_string()?
      };
    }

    let name = get_string!("name");
    let domain = get_string!("domain").parse()?;

    let get_info = obj
      .get("get_info")
      .expect("get_info doesn't exist")
      .clone()
      .into_function()?;

    let data = obj.get("data").expect("cannot find data").clone();

    Ok(Self {
      name,
      domain,
      get_info,
      data,
    })
  }
}

impl TryFrom<SendValue> for Vec<WebsiteModule> {
  type Error = Error;
  fn try_from(value: SendValue) -> Result<Self, Self::Error> {
    use super::SendValueKind as Kind;
    match value.kind_ref() {
      Kind::Vec(_) => {
        let v = value.into_vec()?;
        let mut vec = Vec::new();
        for it in v {
          vec.push(it.try_into()?)
        }
        Ok(vec)
      }

      Kind::Struct(_) | Kind::Object(_) => Ok([value.try_into()?].to_vec()),
      val => Err(Error::expected(
        "Vector, Struct, or Object".to_string(),
        val.to_string_variant().to_string(),
      )),
    }
  }
}

impl WebsiteModule {
  async fn get_info(&self, url: Url) -> Result<MangaInfo, Error> {
    let fut = self
      .get_info
      .async_send_call::<_, DeserializeResult<_>>((self.data.clone(), url));

    let res = fut.await;

    res?.get()
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
}
