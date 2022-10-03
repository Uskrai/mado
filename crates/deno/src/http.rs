use std::num::NonZeroU16;
use std::rc::Rc;
use std::{cell::RefCell, collections::HashMap};

use deno_core::{op, Extension, ExtensionBuilder, OpState, Resource};
use serde::{Deserialize, Serialize};

use crate::error::Error;
use crate::{try_json, ResultJson, ToResultJson};

#[derive(Default)]
pub struct Client {
    pub client: mado_core::http::Client,
}
impl Resource for Client {}

#[derive(Deserialize, Serialize, Debug)]
pub struct RequestBuilder {
    url: url::Url,
    #[serde(default)]
    header: HashMap<String, String>,
}

impl RequestBuilder {
    pub fn to_request(self, client: &mado_core::http::Client) -> mado_core::http::RequestBuilder {
        let mut builder = client.get(self.url.clone());

        for (key, value) in self.header {
            builder = builder.header(key, value);
        }

        builder
    }
}

#[derive(Deserialize, Serialize)]
pub struct ResponseJson {
    status: u16,
    url: url::Url,
    rid: u32,
}

pub struct ResponseResource(mado_core::http::Response);
impl Resource for ResponseResource {}

#[derive(Deserialize, Serialize)]
pub struct StatusCode(NonZeroU16);

fn get_http(state: &mut OpState, rid: u32) -> ResultJson<Rc<Client>> {
    state
        .resource_table
        .get(rid)
        .map_err(|_| Error::resource_error(rid, "Http Client already closed"))
        .to_result_json(state)
}

#[op]
pub fn op_http_client_new(state: &mut OpState) -> u32 {
    state.resource_table.add(Client::default())
}

#[op]
pub fn op_http_client_clone(state: &mut OpState, rid: u32) -> ResultJson<u32> {
    let http = try_json!(get_http(state, rid));
    let rid = state.resource_table.add_rc(http);

    ResultJson::Ok(rid)
}

#[op]
pub fn op_http_client_close(state: &mut OpState, rid: u32) -> ResultJson<()> {
    state
        .resource_table
        .close(rid)
        .map_err(|_| Error::resource_error(rid, "Http already closed"))
        .to_result_json(state)
}

#[op]
pub async fn op_http_client_get<'a>(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    request: RequestBuilder,
) -> ResultJson<ResponseJson> {
    let client = try_json!(get_http(&mut state.borrow_mut(), rid));

    let response = request.to_request(&client.client).send().await;

    let response = match response {
        Ok(response) => response,
        Err(err) => {
            return ResultJson::Err(crate::error::error_to_deno(
                &mut state.borrow_mut(),
                err.into(),
            ));
        }
    };

    ResultJson::Ok(ResponseJson {
        status: response.status().as_u16(),
        url: response.url().clone(),
        rid: state
            .borrow_mut()
            .resource_table
            .add(ResponseResource(response)),
    })
}

#[op]
pub async fn op_http_response_text(state: Rc<RefCell<OpState>>, rid: u32) -> ResultJson<String> {
    let response = {
        let state = &mut state.borrow_mut();

        state
            .resource_table
            .take::<ResponseResource>(rid)
            .map(|it| std::rc::Rc::try_unwrap(it).ok())
            .transpose()
            .and_then(|it| it.ok())
            .ok_or_else(|| Error::resource_error(rid, "Response already closed"))
            .to_result_json(state)
    };

    let response = try_json!(response);

    response
        .0
        .text()
        .await
        .map_err(Error::from)
        .to_result_json_borrow(state)
}

pub fn init() -> Extension {
    ExtensionBuilder::default()
        .ops(vec![
            op_http_client_new::decl(),
            op_http_client_close::decl(),
            op_http_client_clone::decl(),
            op_http_client_get::decl(),
            op_http_response_text::decl(),
        ])
        .build()
}
