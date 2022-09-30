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

    it.task.borrow_mut().add(image);

    Ok(())
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

        op_mado_chapter_task_add::call(&mut state, task, info).unwrap();
    }
}
