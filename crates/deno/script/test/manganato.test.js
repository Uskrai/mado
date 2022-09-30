import { initMadoModule } from "../../dist/module/manganato.js";

const module = initMadoModule()[0];
export async function getInfo__Ok__1() {
  let url = "https://readmanganato.com/manga-yu976355";
  return await module.getInfo(url);
}

export async function getInfo__Ok__2() {
  let url = "https://manganato.com/manga-lu988903";
  return await module.getInfo(url);
}

export async function getInfo__Err_RequestError__404() {
  let url = "https://readmanganato.com/manga-yu176355";
  return await module.getInfo(url);
}

export async function getChapterImage__Ok__1() {
  let info = await getInfo__Ok__1().then((it) => it.throwDebug());
  let chapter = info.chapters[0];

  let id = chapter.id;
  let task = [];
  return await module.getChapterImage(id, task);
}

export async function getChapterImage__Err_RequestError__404() {
  let id = "https://readmanganato.com/manga-yu976355/chapter-1325";
  let task = [];
  return await module.getChapterImage(id, task);
}

export async function downloadImage__Ok__1() {
  let info = await getChapterImage__Ok__1().then((it) => it.throwDebug());

  let images = info[0];
  return await module.downloadImage(images);
}

export function close() {
  module.close();
}
