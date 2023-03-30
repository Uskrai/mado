import { RustChapterTask, RustModule } from "../deps";
import { assertOk } from "../deps/test";
import { initMadoModule } from "../module/mangashiro"

let allModule = initMadoModule()
let module = new RustModule(allModule[0]);
export async function getInfo__Ok() {
  //
  let url = "https://www.asurascans.com/manga/the-novels-extra-remake/"

  return await module.getInfo(url)
}

export async function getChapterImage__Ok() {
  let url = "https://www.asurascans.com/the-novels-extra-chapter-1"

  let task = RustChapterTask.fromRust()
  assertOk(await module.getChapterImage(url, task))

  let arr = task.toArray();
  console.log(arr);

  return arr;
}

export async function close() {
  for (const it of allModule) {
    await it.close();
  }
}
