use std::{cell::RefCell, rc::Rc};

use anyhow::Context;
use deno_core::{op, Extension, ExtensionBuilder, OpState, Resource};
use mado_core::{ChapterImageInfo, ChapterTask};

use crate::{error::error_to_deno, try_json, ResultJson};

pub struct DenoChapterTask {
    task: RefCell<ChapterTaskType>,
}

pub enum ChapterTaskType {
    Trait(Box<dyn ChapterTask>),
    Js(JsChapterTask),
}

pub struct JsChapterTask {
    vec: Vec<ChapterImageInfo>,
}
impl ChapterTask for JsChapterTask {
    fn add(&mut self, image: ChapterImageInfo) {
        self.vec.push(image);
    }
}

impl Resource for DenoChapterTask {}

impl DenoChapterTask {
    pub fn new_to_state(task: Box<dyn ChapterTask>, state: &mut OpState) -> u32 {
        Self::new_type_to_state(ChapterTaskType::Trait(task), state)
    }

    pub fn new_type_to_state(task: ChapterTaskType, state: &mut OpState) -> u32 {
        state.resource_table.add(DenoChapterTask {
            task: RefCell::new(task),
        })
    }

    pub fn into_inner(self) -> Box<dyn ChapterTask> {
        match self.task.into_inner() {
            ChapterTaskType::Trait(it) => it,
            ChapterTaskType::Js(it) => Box::new(it),
        }
    }
}

fn get_chapter_task(state: &mut OpState, rid: u32) -> ResultJson<Rc<DenoChapterTask>> {
    state
        .resource_table
        .get::<DenoChapterTask>(rid)
        .context("Chapter Task is already closed")
        .map_err(|err| error_to_deno(state, err.into()))
        .into()
}

#[op]
fn op_mado_chapter_task_add(
    state: &mut OpState,
    rid: u32,
    image: ChapterImageInfo,
) -> ResultJson<()> {
    let it = try_json!(get_chapter_task(state, rid));

    let mut task = it.task.borrow_mut();
    match &mut *task {
        ChapterTaskType::Trait(it) => it.add(image),
        ChapterTaskType::Js(it) => it.add(image),
    }

    ResultJson::Ok(())
}

#[op]
fn op_mado_chapter_task_to_array(
    state: &mut OpState,
    rid: u32,
) -> ResultJson<Vec<ChapterImageInfo>> {
    let it = try_json!(get_chapter_task(state, rid));

    let mut task = it.task.borrow_mut();
    match &mut *task {
        ChapterTaskType::Js(it) => {
            ResultJson::Ok(it.vec.clone())
        }
        ChapterTaskType::Trait(_) => ResultJson::Ok(vec![]),
    }
}

#[op]
fn op_mado_chapter_task_new(state: &mut OpState) -> u32 {
    let it = JsChapterTask { vec: vec![] };

    DenoChapterTask::new_type_to_state(ChapterTaskType::Js(it), state)
}

pub fn init() -> Extension {
    ExtensionBuilder::default()
        .ops(vec![
            op_mado_chapter_task_new::decl(),
            op_mado_chapter_task_add::decl(),
            op_mado_chapter_task_to_array::decl(),
        ])
        .build()
}

#[cfg(test)]
mod tests {

    use mado_core::MockChapterTask;

    use super::*;

    #[test]
    pub fn test_task() {
        let mut state = OpState::new(0);

        let mut task = MockChapterTask::new();

        let info = ChapterImageInfo {
            id: "id-1".to_string(),
            name: Some("id-1".to_string()),
            extension: "jpeg".to_string(),
        };

        task.expect_add()
            .withf({
                let info = info.clone();
                move |image| *image == info
            })
            .return_once(|_| ());

        let task = DenoChapterTask::new_to_state(Box::new(task), &mut state);

        assert!(matches!(
            op_mado_chapter_task_add::call(&mut state, task, info),
            ResultJson::Ok(_)
        ));
    }
}