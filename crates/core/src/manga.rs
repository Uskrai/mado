use std::fmt::Display;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct MangaInfo {
  pub title: String,
  pub summary: Option<String>,
  pub authors: Vec<String>,
  pub artists: Vec<String>,
  pub cover_link: Option<String>,
  pub genres: Vec<String>,
  pub types: MangaType,
  pub chapters: Vec<Arc<ChapterInfo>>,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct ChapterInfo {
  pub id: String,
  pub title: Option<String>,
  pub chapter: Option<String>,
  pub volume: Option<String>,
  pub scanlator: Option<String>,
  pub language: String,
}

impl Display for ChapterInfo {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    macro_rules! write_if {
      ($name:ident, $fmt:literal) => {
        match &self.$name {
          Some(val) => {
            write!(f, $fmt, val)?;
          }
          None => {}
        }
      };
    }

    write_if!(volume, "Vol. {} ");
    write_if!(chapter, "Chapter {} ");
    if self.volume.is_some() || self.chapter.is_some() {
      write!(f, ": ")?;
    }
    write_if!(title, "{}");
    write_if!(scanlator, " [{}]");
    write!(f, " [{}]", self.language)?;

    Ok(())
  }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum MangaType {
  Series,
  Anthology,
}

impl Default for MangaType {
  fn default() -> Self {
    Self::Series
  }
}
