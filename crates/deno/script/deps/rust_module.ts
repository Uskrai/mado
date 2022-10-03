import { BaseModule } from "./module";
import { Result, ResultFromJson } from "./error";
import { HttpRequest } from "./http";
import { ChapterTask, MangaAndChapters, RustChapterTask } from "./manga";

export class RustModule {
  rid: number;
  constructor(module: BaseModule) {
    this.rid = RustModule.fromRust(module).data;
  }

  static fromRust(module: BaseModule): Result<number> {
    return ResultFromJson(Deno.core.opSync("op_mado_module_new", module));
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
    return ResultFromJson(await Deno.core.opAsync("op_mado_module_download_image", this.rid, info))
  }

  async close() {
    let it = ResultFromJson(await Deno.core.opAsync("op_mado_module_close", this.rid));
    return it;
  }
}
