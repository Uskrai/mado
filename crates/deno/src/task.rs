use std::cell::RefCell;

use deno_core::{op, OpState, Resource};
use mado_core::{ChapterImageInfo, ChapterTask};

pub struct DenoChapterTask {
    task: RefCell<Box<dyn ChapterTask>>,
}

impl Resource for DenoChapterTask {}

impl DenoChapterTask {
    pub fn new_to_state(task: Box<dyn ChapterTask>, state: &mut OpState) -> u32 {
        state.resource_table.add(DenoChapterTask {
            task: RefCell::new(task),
        })
    }
}
#[op]
fn op_mado_chapter_task_add(
    state: &mut OpState,
    rid: u32,
    image: ChapterImageInfo,
) -> Result<(), anyhow::Error> {
    let it = state.resource_table.get::<DenoChapterTask>(rid)?;

    let it = it.task.borrow_mut().add(image);

    Ok(it)
}
