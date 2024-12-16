// import { RustChapterTask, RustModule } from "../deps";
// import { assertEq, assertOk } from "../deps/test";
// import { initMadoModule } from "../module/batoto";
//
// const allModule = initMadoModule();
// const module = new RustModule(allModule[0]);
// export async function getInfo__Ok() {
//   let url =
//     "https://bato.to/series/115663/my-not-so-fair-lady-is-doomed-but-not-if-i-can-help-it-official";
//
//
//   let response = await module.getInfo(url);
//   assertEq(response.data.chapters[0].id, "chapter/2122722");
//
//   return response;
// }
//
// export async function getChapterImage__Ok() {
//   let url = "/chapter/2123725";
//
//   let task = RustChapterTask.fromRust();
//   assertOk(await module.getChapterImage(url, task));
//
//   let arr = task.toArray();
//   return arr;
// }
//
// export async function getInfo__Err_MadoError_RequestError() {
//   let url = "https://bato.to/series/1231231231232";
//   let response = await module.getInfo(url);
//   return response;
// }
//
// export async function downloadImage__Ok() {
//   let url = "https://bato.to";
//
//   return await module.downloadImage({
//     id: url,
//     extension: "webp",
//     name: null,
//   });
// }
//
// export async function close() {
//   for (const it of allModule) {
//     await it.close();
//   }
// }
