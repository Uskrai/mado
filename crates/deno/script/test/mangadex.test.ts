import { RustChapterTask } from "../deps/manga";
import { RustModule } from "../deps/rust_module";
import { assertOk } from "../deps/test";
import { initMadoModule } from "../module/mangadex";

const allmodule = initMadoModule();
let module = new RustModule(allmodule[0]);
export async function getInfo__Ok() {
  let url = "https://mangadex.org/title/5ebe4265-da26-4a3f-a2e4-5634af489ce5";
  return await module.getInfo(url);
}

export async function getInfo__Err_MadoError_RequestError() {
  let url = "https://mangadex.org/title/5ebe4265-da26-4a3f-a2e4-56c4af489ce5";

  let it = await module.getInfo(url);
  return it;
}

export async function getInfo__Err_MadoError_RequestError__InvalidUrl() {
  let url = "https://mangadex.org/title/zebe4265-da26-4a3f-a2e4-56c4af489ce5";

  let it = await module.getInfo(url);
  return it;
}

export async function getChapterImage__Ok__1() {
  let url = "https://mangadex.org/title/5ebe4265-da26-4a3f-a2e4-5634af489ce5";

  let info = await module.getInfo(url).then((it) => it.data);
  let chapter = info.chapters[0];
  let id = chapter.id;
  let task = RustChapterTask.fromRust();
  assertOk(await module.getChapterImage(id, task));
  let arr = task.toArray();
  return arr;
}

export async function downloadImage__Ok__1() {
  let chapter = await getChapterImage__Ok__1();
  chapter = chapter.data;
  let image = chapter[0];

  let it = await module.downloadImage(image);
  return it;
}

export function close() {
  for (let it of allmodule) {
    it.close();
  }
}
