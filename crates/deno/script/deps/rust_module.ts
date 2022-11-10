import { Result, ResultFromJson } from "./error";
import { HttpRequest } from "./http";
import { ChapterTask, MangaAndChapters } from "./manga";
import { ResultModule } from "./module";
import { RustChapterTask } from "./rust_chapter_task";

export class RustModule {
  rid: number;
  constructor(module: ResultModule) {
    this.rid = RustModule.fromRust(module).data;
  }

  static fromRust(module: ResultModule): Result<number> {
    return ResultFromJson(Deno.core.ops.op_mado_module_new(module));
  }

  async getInfo(id: string): Promise<Result<MangaAndChapters>> {
    return ResultFromJson(
      await Deno.core.opAsync("op_mado_module_get_info", this.rid, id)
    );
  }

  async getChapterImage(
    id: string,
    task: RustChapterTask
  ): Promise<Result<ChapterTask>> {
    return ResultFromJson(
      await Deno.core.opAsync(
        "op_mado_module_get_chapter_images",
        this.rid,
        id,
        task.rid
      )
    );
  }

  async downloadImage(info: object): Promise<Result<HttpRequest>> {
    return ResultFromJson(
      await Deno.core.opAsync("op_mado_module_download_image", this.rid, info)
    );
  }

  async close() {
    let it = ResultFromJson(
      await Deno.core.opAsync("op_mado_module_close", this.rid)
    );
    return it;
  }
}
