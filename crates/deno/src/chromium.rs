use std::{cell::RefCell, rc::Rc, sync::Arc};

use anyhow::Context;
use async_once_cell::OnceCell;
use deno_core::{op, Extension, OpState, Resource};
use headless_chrome::{
    browser::default_executable,
    protocol::cdp::DOM::{Node, NodeId},
    Browser, Element, Tab as ChromeTab,
};

pub fn spawn_blocking<F, R>(f: F) -> tokio::task::JoinHandle<R>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let span = tracing::Span::current();
    tokio::task::spawn_blocking(move || {
        let _s = span.entered();
        f()
    })
}

use crate::{try_json, Error, ResultJson, ToResultJson};

#[derive(Clone)]
pub struct Chromium {
    browser: Rc<async_once_cell::OnceCell<Browser>>,
}

impl Drop for Chromium {
    fn drop(&mut self) {
        tracing::trace!("dropping chromium");
    }
}

#[derive(Clone)]
pub struct Tab {
    tab: Arc<ChromeTab>,
}

impl Drop for Tab {
    fn drop(&mut self) {
        tracing::trace!("dropping tab");
    }
}

pub struct TabElement {
    tab: Arc<ChromeTab>,
    node_id: NodeId,
}

impl Chromium {
    pub async fn get(&self) -> Result<&Browser, anyhow::Error> {
        self.browser
            .get_or_try_init(async {
                spawn_blocking(|| {
                    let ws_url = std::env::var("CHROMIUM_WS_URL").ok();
                    if let Some(ws_url) = ws_url {
                        tracing::trace!("connecting to {ws_url}");

                        Browser::connect(ws_url).map_err(|err| anyhow::anyhow!(err))
                    } else {
                        tracing::trace!("launching browser");
                        Browser::new(
                            headless_chrome::LaunchOptions::default_builder()
                                .path(Some(
                                    default_executable().map_err(|err| anyhow::anyhow!(err))?,
                                ))
                                .args(
                                    ["--no-zygote", "--no-sandbox"]
                                        .map(|it| std::ffi::OsStr::new(it))
                                        .to_vec(),
                                )
                                .idle_browser_timeout(std::time::Duration::from_secs(u64::MAX))
                                .build()?,
                        )
                        .tap_ok(|_| tracing::info!("launching browser success"))
                        .map_err(|err| anyhow::anyhow!(err))
                    }
                })
                .await
                .map_err(|err| anyhow::anyhow!(err))
                .and_then(|it| it)
            })
            .await
    }
}

impl Resource for Chromium {}
impl Resource for Tab {}
impl Resource for TabElement {}

fn get_chromium(state: Rc<RefCell<OpState>>) -> Result<Chromium, Error> {
    let browser = state.borrow().try_borrow::<Chromium>().cloned();

    if let Some(browser) = browser {
        Ok(browser)
    } else {
        state.borrow_mut().put(Chromium {
            browser: Rc::new(OnceCell::new()),
        });

        Ok(state.borrow().borrow::<Chromium>().clone())
    }
}

#[tracing::instrument(skip_all)]
#[op]
pub async fn op_mado_chromium_new_tab(state: Rc<RefCell<OpState>>) -> ResultJson<u32> {
    // let tab = chromium.
    let inner = || async {
        let browser = get_chromium(state.clone())?.get().await?.clone();

        let tab = spawn_blocking(move || {
            tracing::trace!("launching new tab");
            let tab = browser.new_tab()?;
            tab.set_user_agent(
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.114 Safari/537.36", 
                Some("en-US,en;q=0.9,hi;q=0.8,es;q=0.7,lt;q=0.6"), 
                Some("macOS")
            )?;

            Ok::<_, Error>(tab)
        })
        .await
        .context("spawn blocking failed")?;

        tab
    };

    let tab = try_json!(inner().await.to_result_json_borrow(state.clone()));

    let rid = state.borrow_mut().resource_table.add(Tab { tab });
    ResultJson::Ok(rid)
}

#[tracing::instrument(skip_all, fields(tab = rid))]
#[op]
pub async fn op_mado_chromium_tab_goto(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    url: String,
) -> ResultJson<()> {
    with_tab_async(state.clone(), rid, move |tab| {
        tracing::trace!("goto {}", url);
        tab.navigate_to(&url)?;
        Ok(())
    })
    .await
    .to_result_json_borrow(state)
}

#[tracing::instrument(skip_all, fields(tab = rid))]
#[op]
pub async fn op_mado_chromium_tab_wait_for_navigation(
    state: Rc<RefCell<OpState>>,
    rid: u32,
) -> ResultJson<()> {
    with_tab_async(state.clone(), rid, |tab| {
        tracing::trace!("wait for navigation");
        tab.wait_until_navigated()?;
        Ok(())
    })
    .await
    .to_result_json_borrow(state)
}

#[op]
pub async fn op_mado_chromium_tab_content(
    state: Rc<RefCell<OpState>>,
    rid: u32,
) -> ResultJson<String> {
    with_tab_async(state.clone(), rid, |tab| {
        tab.get_content().map_err(Into::into)
    })
    .await
    .to_result_json_borrow(state)
}

fn with_tab<F, R>(state: Rc<RefCell<OpState>>, rid: u32, fun: F) -> Result<R, Error>
where
    F: FnOnce(Arc<ChromeTab>) -> Result<R, Error>,
{
    let tab = state
        .borrow_mut()
        .resource_table
        .get::<Tab>(rid)?
        .tab
        .clone();

    fun(tab)
}

async fn with_tab_async<F, R>(state: Rc<RefCell<OpState>>, rid: u32, fun: F) -> Result<R, Error>
where
    F: FnOnce(Arc<ChromeTab>) -> Result<R, Error> + Send + 'static,
    R: Send + 'static,
{
    let inner = || async {
        let tab = state.borrow().resource_table.get::<Tab>(rid)?.tab.clone();

        let result = spawn_blocking(move || fun(tab))
            .await
            .context("spawn_blocking failed")??;

        Ok(result)
    };

    inner().await
}

async fn with_element<F, R>(state: Rc<RefCell<OpState>>, rid: u32, fun: F) -> Result<R, Error>
where
    F: FnOnce(Element<'_>) -> Result<R, Error> + Send + 'static,
    R: Send + 'static,
{
    let inner = || async {
        let element = state.borrow().resource_table.get::<TabElement>(rid)?;

        let tab = element.tab.clone();
        let node_id = element.node_id;
        let result = spawn_blocking(move || {
            let element = Element::new(&tab, node_id)?;

            fun(element)
        })
        .await
        .context("spawn_blocking failed")??;

        Ok(result)
    };

    inner().await
}

#[op]
pub async fn op_mado_chromium_tab_url(state: Rc<RefCell<OpState>>, rid: u32) -> ResultJson<String> {
    with_tab(state.clone(), rid, |tab| Ok(tab.get_url())).to_result_json_borrow(state)
}

#[op]
pub async fn op_mado_chromium_tab_evaluate(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    script: String,
) -> ResultJson<()> {
    with_tab_async(state.clone(), rid, move |tab| {
        let it = tab.evaluate(&script, true)?;
        println!("{:?}", it);
        Ok(())
    })
    .await
    .to_result_json_borrow(state)
}

#[tracing::instrument(
    skip_all,
    fields(tab = rid)
)]
#[op]
pub async fn op_mado_chromium_tab_wait_for_element(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    selector: String,
) -> ResultJson<u32> {
    to_tab_element(state, rid, selector, |tab, selector| {
        tracing::trace!("wait for element {}", selector);
        tab.wait_for_element(selector).map_err(Into::into)
    })
    .await
}

#[tracing::instrument(skip_all, fields(tab = rid))]
#[op]
pub async fn op_mado_chromium_tab_wait_for_element_by_xpath(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    selector: String,
) -> ResultJson<u32> {
    to_tab_element(state, rid, selector, |tab, selector| {
        tracing::trace!("wait for element by xpath {}", selector);
        tab.wait_for_xpath(selector).map_err(Into::into)
    })
    .await
}

async fn to_tab_element<F>(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    selector: String,
    fun: F,
) -> ResultJson<u32>
where
    F: for<'a> FnOnce(&'a Arc<ChromeTab>, &'a str) -> Result<headless_chrome::Element<'a>, Error>
        + Send
        + 'static,
{
    let element = with_tab_async(state.clone(), rid, move |tab| {
        let node_id = fun(&tab, &selector)?.node_id;

        Ok(TabElement { tab, node_id })
    })
    .await
    .to_result_json_borrow(state.clone());

    let element = try_json!(element);
    let rid = state.borrow_mut().resource_table.add(element);

    ResultJson::Ok(rid)
}

#[tracing::instrument(
    skip_all,
    fields(tab = rid)
)]
#[op]
pub async fn op_mado_chromium_tab_click(
    state: Rc<RefCell<OpState>>,
    rid: u32,
    selector: String,
) -> ResultJson<()> {
    with_tab_async(state.clone(), rid, move |tab| {
        tracing::trace!("click: {}", selector);
        tab.wait_for_element(&selector)?.click()?;

        Ok(())
    })
    .await
    .to_result_json_borrow(state)
}

#[tracing::instrument(
    skip_all,
    fields(
        tab = rid
    )
)]
#[op]
pub async fn op_mado_chromium_tab_close(state: Rc<RefCell<OpState>>, rid: u32) -> ResultJson<()> {
    let it = with_tab_async(state.clone(), rid, move |tab| {
        tracing::trace!("close tab");
        tab.close_with_unload()?;

        Ok(())
    })
    .await
    .to_result_json_borrow(state.clone());

    state.borrow_mut().resource_table.close(rid).ok();

    it
}

#[tracing::instrument(
    skip_all,
    fields(
        tab = rid,
        force = force,
    )
)]
pub async fn reload_tab(state: Rc<RefCell<OpState>>, rid: u32, force: bool) -> ResultJson<()> {
    with_tab_async(state.clone(), rid, move |tab| {
        tab.reload(force, None)?;

        Ok(())
    })
    .await
    .to_result_json_borrow(state)
}

#[op]
pub async fn op_mado_chromium_tab_reload(state: Rc<RefCell<OpState>>, rid: u32) -> ResultJson<()> {
    reload_tab(state, rid, false).await
}

#[op]
pub async fn op_mado_chromium_tab_reload_force(
    state: Rc<RefCell<OpState>>,
    rid: u32,
) -> ResultJson<()> {
    reload_tab(state, rid, true).await
}

#[op]
pub async fn op_mado_chromium_element_click(
    state: Rc<RefCell<OpState>>,
    rid: u32,
) -> ResultJson<()> {
    with_element(state.clone(), rid, |element| {
        element.click()?;
        Ok(())
    })
    .await
    .to_result_json_borrow(state)
}

#[op]
pub async fn op_mado_chromium_element_node(
    state: Rc<RefCell<OpState>>,
    rid: u32,
) -> ResultJson<Node> {
    with_element(state.clone(), rid, |element| {
        let it = element.get_description()?;

        Ok(it)
    })
    .await
    .to_result_json_borrow(state)
}

pub fn init() -> Extension {
    Extension::builder()
        .ops(vec![
            op_mado_chromium_new_tab::decl(),
            op_mado_chromium_tab_content::decl(),
            op_mado_chromium_tab_goto::decl(),
            op_mado_chromium_tab_wait_for_navigation::decl(),
            op_mado_chromium_tab_url::decl(),
            op_mado_chromium_tab_reload::decl(),
            op_mado_chromium_tab_reload_force::decl(),
            op_mado_chromium_tab_wait_for_element::decl(),
            op_mado_chromium_tab_wait_for_element_by_xpath::decl(),
            op_mado_chromium_tab_click::decl(),
            op_mado_chromium_tab_evaluate::decl(),
            op_mado_chromium_tab_close::decl(),
            op_mado_chromium_element_click::decl(),
            op_mado_chromium_element_node::decl(),
            // op_mado_chapter_task_new::decl(),
            // op_mado_chapter_task_add::decl(),
            // op_mado_chapter_task_to_array::decl(),
        ])
        .build()
}
