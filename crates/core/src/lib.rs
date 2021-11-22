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

mod error;
mod http_error;
#[allow(dead_code)]
mod manga;

pub use error::Error;
pub use manga::*;

pub mod url;

pub trait ChapterTask: Send {
  fn add(&mut self, name: Option<String>, id: String);
  fn get_chapter(&self) -> &ChapterInfo;
}

#[async_trait::async_trait]
pub trait WebsiteModule: Send {
  /// Get Manga information from `url`
  async fn get_info(&self, url: self::url::Url) -> Result<MangaInfo, Error>;

  /// Get Image of Chapter from `task::get_chapter`
  /// for each image `task::add` should be called
  async fn get_chapter_images(
    &self,
    task: Box<dyn ChapterTask>,
  ) -> Result<(), Error>;
}
