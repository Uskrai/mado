use crate::chapter_task::RuneChapterTask;
use crate::function::RuneFunction;
use crate::uuid::Uuid as RuneUuid;
use crate::DeserializeResult;
use crate::Rune;
use crate::SendValue;
use mado_core::ChapterImageInfo;
use mado_core::MadoModule;
use mado_core::MangaAndChaptersInfo;
use mado_core::Url;

use super::Error;

use async_trait::async_trait;
use mado_core::ChapterTask;
use mado_core::Uuid;
use rune::runtime::VmError as RuneVmError;
use rune::FromValue;
use rune::ToValue;

#[derive(Clone, derivative::Derivative)]
#[derivative(Debug)]
pub struct RuneMadoModule {
    #[derivative(Debug = "ignore")]
    #[allow(dead_code)]
    rune: Rune,
    client: mado_core::Client,

    uuid: Uuid,
    name: String,
    domain: Url,

    get_info: RuneFunction,
    get_chapter_images: RuneFunction,
    download_image: RuneFunction,

    data: SendValue,
}
impl RuneMadoModule {
    async fn get_info(&self, url: super::http::Url) -> Result<MangaAndChaptersInfo, Error> {
        let fut = self
            .get_info
            .async_call::<_, DeserializeResult<_>>((self.data.clone(), url))
            .await;

        fut?.get()
    }

    pub async fn get_chapter_images(&self, id: &str, task: RuneChapterTask) -> Result<(), Error> {
        self.get_chapter_images
            .async_call((self.data.clone(), id, task))
            .await?
    }

    pub async fn download_image(
        &self,
        image: ChapterImageInfo,
    ) -> Result<mado_core::RequestBuilder, Error> {
        let value = crate::serializer::for_async_call(image);

        let request = self
            .download_image
            .async_call::<_, Result<crate::http::RequestBuilder, Error>>((self.data.clone(), value))
            .await??;

        let request = request.into_inner();

        Ok(mado_core::RequestBuilder::Http(request))
    }
}

#[async_trait]
impl MadoModule for RuneMadoModule {
    fn uuid(&self) -> Uuid {
        self.uuid
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn client(&self) -> &mado_core::Client {
        &self.client
    }

    fn domain(&self) -> &Url {
        &self.domain
    }

    async fn get_info(&self, url: Url) -> Result<MangaAndChaptersInfo, mado_core::Error> {
        self.get_info(url.into()).await.map_err(Into::into)
    }

    async fn get_chapter_images(
        &self,
        id: &str,
        task: Box<dyn ChapterTask>,
    ) -> Result<(), mado_core::Error> {
        self.get_chapter_images(id, RuneChapterTask::new(task))
            .await
            .map_err(Into::into)
    }

    async fn download_image(
        &self,
        image: mado_core::ChapterImageInfo,
    ) -> Result<mado_core::RequestBuilder, mado_core::Error> {
        self.download_image(image).await.map_err(Into::into)
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
        let domain = from_value::<crate::http::Url, _>(obj["domain"].clone())?.into_inner();

        let client = from_value::<crate::http::Client, _>(obj["client"].clone())?.clone();
        let client = mado_core::Client::Http(client.clone().into_inner());

        macro_rules! get_function {
            ($name:literal) => {
                RuneFunction::new(rune.clone(), obj[$name].clone().into_function()?)
            };
        }

        let get_info = get_function!("get_info");
        let get_chapter_images = get_function!("get_chapter_images");
        let download_image = get_function!("download_image");

        let data = obj["module"].clone();

        Ok(Self {
            rune,
            uuid,
            name,
            domain,
            client,
            get_info,
            get_chapter_images,
            download_image,
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
