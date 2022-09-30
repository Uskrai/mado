use std::fmt::Display;
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::Arc;

use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub struct MangaInfo {
    pub id: String,
    pub title: String,
    pub summary: Option<String>,
    pub authors: Vec<String>,
    pub artists: Vec<String>,
    pub cover_link: Option<String>,
    pub genres: Vec<String>,
    pub types: MangaType,
}

#[derive(Deserialize, Serialize, Debug, Default, Clone, PartialEq)]
pub struct MangaAndChaptersInfo {
    pub manga: Arc<MangaInfo>,
    pub chapters: Arc<ChaptersInfo>,
}

#[derive(Serialize, Debug, Default, Clone, PartialEq)]
pub struct ChaptersInfo(pub Vec<Arc<ChapterInfo>>);

impl Deref for ChaptersInfo {
    type Target = Vec<Arc<ChapterInfo>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ChaptersInfo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'de> serde::Deserialize<'de> for ChaptersInfo {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Vec<Arc<ChapterInfo>>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                std::write!(formatter, "a sequence")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut vec = Vec::new();
                if let Some(reserve) = seq.size_hint() {
                    vec.reserve(reserve);
                }

                while let Some(mut it) = seq.next_element::<ChapterInfo>()? {
                    // index started from 1, so len() + 1
                    it.index = Some(vec.len() + 1);
                    vec.push(Arc::new(it));
                }
                vec.shrink_to_fit();
                Ok(vec)
            }
        }

        let it = deserializer.deserialize_seq(Visitor)?;

        Ok(Self(it))
    }
}

#[derive(Debug, Deserialize, Serialize, Default, Clone, PartialEq)]
pub struct ChapterInfo {
    pub index: Option<usize>,
    pub id: String,
    pub title: Option<String>,
    pub chapter: Option<String>,
    pub volume: Option<String>,
    pub scanlator: Vec<String>,
    pub language: String,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone, PartialEq)]
pub struct ChapterImageInfo {
    pub id: String,
    pub extension: String,
    pub name: Option<String>,
}

impl ChapterInfo {
    pub fn display_without_index(&self) -> impl Display + '_ {
        pub struct ImplDisplay<'a>(&'a ChapterInfo);

        impl<'a> Display for ImplDisplay<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt_without_index(f)
            }
        }

        ImplDisplay(self)
    }

    pub fn fmt_without_index(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        macro_rules! write_if {
            ($fmt:literal, $name:ident) => {
                if let Some(val) = &self.$name {
                    write!(f, $fmt, val)?;
                }
            };
        }

        write_if!("Vol. {} ", volume);
        write_if!("Chapter {} ", chapter);
        if (self.volume.is_some() || self.chapter.is_some()) && self.title.is_some() {
            write!(f, ": ")?;
        }
        write_if!("{} ", title);
        if !self.scanlator.is_empty() {
            write!(f, "[{}]", self.scanlator.join(","))?;
        }
        write!(f, "[{}]", self.language)?;

        Ok(())
        //
    }
}

impl Display for ChapterInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(val) = self.index {
            write!(f, "{:0>4}. ", val)?;
        }

        self.fmt_without_index(f)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum MangaType {
    Series,
    Anthology,
}

impl Default for MangaType {
    fn default() -> Self {
        Self::Series
    }
}
