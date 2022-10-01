import { initMadoModule } from "../module/mangadex";
import { RustChapterTask } from "../deps/manga";
import { RustModule } from "../deps/rust_module";

let module = new RustModule(initMadoModule()[0]);
export async function getInfo__Ok() {
  let url = "https://mangadex.org/title/5ebe4265-da26-4a3f-a2e4-5634af489ce5";
  return await module.getInfo(url);
}

export async function getInfo__Err_MadoError() {
  let url = "https://mangadex.org/title/5ebe4265-da26-4a3f-a2e4-56c4af489ce5";

  let it = await module.getInfo(url);
  return it;
}

async function getChapterImage__Ok__1() {
  let url = "https://mangadex.org/title/5ebe4265-da26-4a3f-a2e4-5634af489ce5";

  let info = await module.getInfo(url).then((it) => it.data);
  let chapter = info.chapters[0];
  let id = chapter.id;
  let task = RustChapterTask.fromRust();
  await module.getChapterImage(id, task);
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
  module.close();
}
