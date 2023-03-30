import { RustChapterTask, RustModule } from "../deps";
import { assertEq, assertOk } from "../deps/test";
import { initMadoModule } from "../module/reaperscans";

let allModule = initMadoModule();
const module = new RustModule(allModule[0]);

export async function getInfo__Ok() {
  let url =
    "https://reaperscans.com/comics/3407-whats-wrong-with-being-the-villainess";

  let info = await module.getInfo(url);

  assertEq(
    info.data.chapters[0].id,
    "https://reaperscans.com/comics/3407-whats-wrong-with-being-the-villainess/chapters/91341515-chapter-1"
  );

  return info;
}

export async function getInfo__Err_MadoError_RequestError() {
  let url = 
    "https://reaperscans.com/comics/3407-whats-wrong-with-being-the-villaine";

  let info = await module.getInfo(url);
  return info;
}

export async function getChapterImage__Ok() {
  let url =
    "https://reaperscans.com/comics/3407-whats-wrong-with-being-the-villainess/chapters/91341515-chapter-1";

  let task = RustChapterTask.fromRust();
  assertOk(await module.getChapterImage(url, task));

  let arr = task.toArray();
  return arr;
}

export async function downloadImage__Ok() {
  let url = "https://reaperscans.com";

  return await module.downloadImage({
    id: url,
    extension: "webp",
    name: null
  });
}

export async function close() {
  for (const it of allModule) {
    await it.close();
  }
}
