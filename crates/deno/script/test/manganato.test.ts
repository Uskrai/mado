import { RustChapterTask, RustModule } from "../deps/index";
import { assertEq, assertOk } from "../deps/test";
import { initMadoModule } from "../module/manganato";
import { Ok } from "../deps/error";

const allmodule = initMadoModule();
const module = new RustModule(allmodule[0]);
export async function getInfo__Ok__1() {
  let url = "https://chapmanganato.com/manga-yu976355";
  let manga = await module.getInfo(url).then(it => it.throwDebug());
  assertEq(manga.chapters[0].id, 'https://chapmanganato.com/manga-yu976355/chapter-1');
  
  return Ok(manga);
}

export async function getInfo__Ok__2() {
  let url = "https://manganato.com/manga-lu988903";
  let manga = await module.getInfo(url).then(it => it.throwDebug());
  assertEq(manga.chapters[0].id, 'https://chapmanganato.com/manga-lu988903/chapter-1');
  return Ok(manga);
}

export async function getInfo__Err_MadoError_RequestError__404() {
  let url = "https://chapmanganato.com/manga-yu176355";
  return await module.getInfo(url);
}

export async function getChapterImage__Ok__1() {
  let info = await getInfo__Ok__1().then((it) => it.throwDebug());
  let chapter = info.chapters[0];

  let id = chapter.id;
  let task = RustChapterTask.fromRust();
  assertOk(await module.getChapterImage(id, task));
  return task.toArray();
}

export async function getChapterImage__Err_MadoError_RequestError__404() {
  let id = "https://chapmanganato.com/manga-yu976355/chapter-1325";
  let task = RustChapterTask.fromRust();
  return await module
    .getChapterImage(id, task)
    .then((it) => it.map(() => task));
}

export async function downloadImage__Ok__1() {
  let info = await getChapterImage__Ok__1().then((it) => it.throwDebug());

  let images = info[0];
  return await module.downloadImage(images);
}

export async function close() {
  for (const it of allmodule) {
    await it.close();
  }
  return Ok({});
}
