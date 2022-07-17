import { JSONPath } from "jsonpath-plus";
export { JSONPath };
export {
  Manga,
  Chapter,
  MangaAndChapters,
  ChapterTask,
  RustChapterTask,
  ChapterImageInfo,
} from "./manga";

export { HttpModule, ModuleWrapper, Module } from "./module";
export { HttpClient, HttpResponse, HttpRequest } from "./http";
export {
  catchAndReturn,
  Errors as Error,
  ResultOk,
  ResultError,
  Result,
  ResultFromJson,
} from "./error";
export { XHTMLPath } from "./xhtmlpath";
export { Resource } from "./resource";

import { rx as VRegex } from "verbose-regexp";

export { VRegex };
// export const VRegex = VRegex;
//
if (typeof globalThis.__bootstrap !== "undefined") {
  // @ts-ignore
  globalThis.window = globalThis;

  // @ts-ignore
  console = new __bootstrap.console.Console((msg: any, level: any) =>
    // @ts-ignore
    Deno.core.print(msg, level > 1)
  );
}

export class CommonClosure {
  static query(json: object): (args0: string) => any {
    return (path: string) => CommonFunction.query(json, path);
  }
}

export class CommonFunction {
  static url_extension(url: string) {
    // https://stackoverflow.com/questions/6997262/how-to-pull-url-file-extension-out-of-url-string-using-javascript
    return url.split(/[#?]/)[0].split(".").pop().trim();
  }

  static query(json: object, path: string) {
    return JSONPath({ path, json, wrap: false });
  }
}

// export default {
//     xpath: await import("xpath-ts"),
// };
