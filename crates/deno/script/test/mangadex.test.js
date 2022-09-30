// import { initMadoModule } from "../../dist/module/mangadex.js";
import { initMadoModule } from "../../dist/module/mangadex.js";

const module = initMadoModule()[0];
export async function getInfo__Ok() {
  let url = "https://mangadex.org/title/5ebe4265-da26-4a3f-a2e4-5634af489ce5";
  return await module.getInfo(url);
}

export async function getInfo__Err_UnexpectedError() {
  let url = "https://mangadex.org/title/5ebe4265-da26-4a3f-a2e4-56c4af489ce5";

  let it = await module.getInfo(url);
  return it;
}

export async function getChapterImage__Ok__1() {
  let url = "https://mangadex.org/title/5ebe4265-da26-4a3f-a2e4-5634af489ce5";

  let info = await module.getInfo(url).then((it) => it.data);
  let chapter = info.chapters[0];
  let id = chapter.id;
  let task = [];
  return await module.getChapterImage(id, task);
}

export async function downloadImage__Ok__1() {
  let chapter = await getChapterImage__Ok__1().then((it) => it.data);
  let image = chapter[0];

  return await module.downloadImage(image);
}

export function close() {
  module.close();
}
