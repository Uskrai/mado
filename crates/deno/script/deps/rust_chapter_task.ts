import { ResultFromJson } from "./error";

export class RustChapterTask {
  rid: number;

  constructor(rid: number) {
    this.rid = rid;
  }

  static fromRust() {
    return new RustChapterTask(Deno.core.ops.op_mado_chapter_task_new());
  }

  push(image: object) {
    return ResultFromJson(
      Deno.core.ops.op_mado_chapter_task_add(this.rid, image)
    );
  }

  toArray() {
    return ResultFromJson(
      Deno.core.ops.op_mado_chapter_task_to_array(this.rid)
    );
  }
}
