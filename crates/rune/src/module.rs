use crate::chapter_task::RuneChapterTask;
use crate::function::RuneFunction;
use crate::uuid::Uuid as RuneUuid;
use crate::DeserializeResult;
use crate::Rune;
use crate::SendValue;
use mado_core::MadoModule;

use super::http::Url;
use super::Error;

use async_trait::async_trait;
use mado_core::ChapterTask;
use mado_core::MangaInfo;
use mado_core::Uuid;
use rune::runtime::VmError as RuneVmError;
use rune::FromValue;
use rune::ToValue;

#[derive(Clone, Debug)]
pub struct RuneMadoModule {
    rune: Rune,

    uuid: Uuid,
    name: String,
    domain: Url,

    get_info: RuneFunction,
    get_chapter_images: RuneFunction,

    data: SendValue,
}
impl RuneMadoModule {
    async fn get_info(&self, url: Url) -> Result<MangaInfo, Error> {
        let fut = self
            .get_info
            .async_call::<_, DeserializeResult<_>>((self.data.clone(), url))
            .await;

        fut?.get()
    }

    pub async fn get_chapter_images(&self, task: RuneChapterTask) -> Result<(), Error> {
        Ok(self
            .get_chapter_images
            .async_call::<_, ()>((self.data.clone(), task))
            .await?)
    }
}

#[async_trait]
impl MadoModule for RuneMadoModule {
    fn get_uuid(&self) -> Uuid {
        self.uuid
    }

    fn get_domain(&self) -> mado_core::url::Url {
        self.domain.clone().into()
    }

    async fn get_info(&self, url: mado_core::url::Url) -> Result<MangaInfo, mado_core::Error> {
        self.get_info(Url::from(url)).await.map_err(Into::into)
    }

    async fn get_chapter_images(&self, task: Box<dyn ChapterTask>) -> Result<(), mado_core::Error> {
        self.get_chapter_images(RuneChapterTask::new(task))
            .await
            .map_err(Into::into)
    }
}

impl RuneMadoModule {
    /// Retreive data
    pub fn data(&self) -> SendValue {
        self.data.clone()
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn from_value(rune: crate::Rune, value: SendValue) -> Result<RuneMadoModule, RuneVmError> {
        fn from_value<R: FromValue, T: ToValue>(value: T) -> Result<R, RuneVmError> {
            FromValue::from_value(value.to_value()?)
        }
        let obj = value.into_object()?;

        let uuid = from_value::<RuneUuid, _>(obj["uuid"].clone())?.into();
        let name = from_value(obj["name"].clone())?;
        let domain = from_value(obj["domain"].clone())?;

        let get_function = |name| -> Result<_, RuneVmError> {
            let fun = obj[name].clone().into_function()?;
            Ok(RuneFunction::new(rune.clone(), fun))
        };

        let get_info = get_function("get_info")?;
        let get_chapter_images = get_function("get_chapter_images")?;

        let data = obj.get("data").expect("cannot find data").clone();

        Ok(Self {
            rune,
            uuid,
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
    ) -> Result<Vec<RuneMadoModule>, RuneVmError> {
        use super::SendValueKind as Kind;

        match value.kind_ref() {
            Kind::Vec(_) => {
                let v = value.into_vec()?;
                let mut vec = Vec::new();
                for it in v {
                    vec.push(Self::from_value(rune.clone(), it)?);
                }
                Ok(vec)
            }

            Kind::Struct { .. } => Ok([Self::from_value(rune, value)?].to_vec()),
            _ => {
                let value = value.to_value()?;
                let type_info = value.type_info()?;
                let err = RuneVmError::expected::<rune::runtime::Vec>(type_info);

                Err(err)
            }
        }
    }
}
