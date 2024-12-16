import {
  ChapterImageInfo,
  ChapterTask,
  CommonFunction,
  HttpClient,
  HttpModule,
  HttpRequest,
  Manga,
  MangaAndChapters,
  RustHttpClient,
  XHTMLPath,
} from "../deps";
import { Browser } from "../deps/browser";
import { Cloudflare } from "../deps/cloudflare";
import { Errors } from "../deps/error";
import { ResultModule } from "../deps/module";
import { RustBrowser } from "../deps/rust_browser";

const NOT_FOUND = "//div/div[class=content]/div[class=notf]/img";

import parseObjectLiteral from "object-literal-parse";

class MangaShiro implements HttpModule {
  constructor(
    public client: HttpClient,
    public uuid: string,
    public name: string,
    public domain: string,
    public reverseChapter: boolean,
    public browser: Browser
  ) {}

  async getInfo(url: string): Promise<MangaAndChapters> {
    let cf = new Cloudflare(this.browser);
    try {
      let response = await cf
        .get(url)
        .waitForElementByXPath(`${NOT_FOUND} or //h1[@class=entry-title]`)
        .get();

      let doc = new XHTMLPath(response.data);

      let manga = this.parse_info(url, doc);
      let chapters = this.parse_chapter(url, doc);

      if (this.reverseChapter) {
        chapters.reverse();
      }

      console.log(manga, chapters)

      return {
        manga,
        chapters,
      };
    } catch (e) {
      console.log(e);
      throw e;
    } finally {
      await cf.close();
    }
  }

  parse_404(url: string, doc: XHTMLPath) {
    if (doc.select('//title[contains(., "Page Not Found")]').length) {
      throw Errors.request_error(url, "404 PAGE NOT FOUND");
    }
  }

  parse_info(id: string, doc: XHTMLPath): Manga {
    this.parse_404(id, doc);

    function selectString(array: string[]) {
      let result = "";
      for (const it of array) {
        result = doc.selectString(it);

        if (result != "") {
          break;
        }
      }

      return result;
    }

    let title = selectString(['//h1[@class="entry-title"]', "//h1/text()"]);

    let cover_link = "";

    let authors = []; // TODO
    let artists = [];
    let genres = [];

    let summary = "";
    let types: "Series" | "Anthology" = "Series";

    return {
      id,
      title,
      types,
      authors,
      artists,
      genres,
      summary,
      cover_link,
      chapters: [],
    };
  }

  parse_chapter(id: string, doc: XHTMLPath) {
    let query = '//*[@id="chapterlist"]//*[@class="eph-num"]/a';

    // flamescans
    if (this.uuid == "9caa1926-d020-458a-8d43-276b36df7dbd") {
      return doc.select('//*[@id="chapterlist"]//li/a').map((it: Element) => {
        const path = XHTMLPath.fromNode(it);
        const title = path
          .selectString('div/div/span[@class="chapternum"]')
          .replaceAll("\n", " ");
        console.log(JSON.stringify({ title }));

        return {
          id: it.getAttribute("href"),
          title,
          chapter: null,
          volume: null,
          language: "en",
          scanlator: [this.name],
        };
      });
    }

    if (
      [
        // manhwafreak
        "a51ebfb8-9790-45d5-89cd-867c48a095c0",
        // mmscans
        "38ee90a6-e1f3-43e2-8447-8e090399d7d2",
      ].includes(this.uuid)
    ) {
      const queries = {
        'a51ebfb8-9790-45d5-89cd-867c48a095c0': '//div[@class="chapter-li"]/a',
        "38ee90a6-e1f3-43e2-8447-8e090399d7d2": '//li[@class="chapter-li]/a',
        "7fa94290-99b7-4bbc-8413-f2d364294224": '//div[@class="pl-4 py-2 border rounded-md group w-full hover:bg-[#343434] cursor-pointer border-[#A2A2A2]/20 relative"]/a'
      }
      console.log("test");

      return doc.select(queries[this.uuid]).map((it: Element) => {
        const path = XHTMLPath.fromNode(it);
        const title = path.selectString("div/div/p[1]").replaceAll("\n", " ");

        console.log(JSON.stringify({ title }));

        return {
          id: it.getAttribute("href"),
          title,
          chapter: null,
          volume: null,
          language: "en",
          scanlator: [this.name],
        };
      });
    }

    return doc
      .select('//*[@id="chapterlist"]//*[@class="eph-num"]/a')
      .map((it: Element) => {
        const path = XHTMLPath.fromNode(it);
        return {
          id: it.getAttribute("href"),
          title: path.selectString('span[@class="chapternum"]'),
          volume: null,
          chapter: null,
          language: "en",
          scanlator: [this.name],
        };
      });
  }

  async getChapterImage(id: string, task: ChapterTask): Promise<void> {
    let cf = new Cloudflare(this.browser);
    try {
      const queries = [
        '//*[@id="readerarea"]//img/@src',
        '//*[@id="readerarea"]/p/img/@src',
        '//*[@id="readerarea"]/p//img/@src',
      ];
      let ignore_id = [
        "https://manhwafreak.com/wp-content/uploads/2022/08/Comp-11.gif",
        "https://flamescans.org/wp-content/uploads/2021/10/Swordsman.jpg",
      ];

      let response = await cf
        .get(id)
        .waitForElementByXPath(
          [...queries, `script[contains(., "ts_reader")]`].join(" or ")
        )
        .get();

      let doc = new XHTMLPath(response.data);

      this.parse_404(id, doc);

      const q = `//script[contains(text(), "ts_reader")]`;
      const script = doc.selectTextContent(q).at(0);

      let images = [];
      if (script != null) {
        const jsonString = script.match(/ts_reader\.run\((.*)\);/s)?.[1];

        let src = undefined;

        console.log(jsonString, src);
        try {
          src = JSON.parse(jsonString);
        } catch (e) {
          src = parseObjectLiteral(jsonString);

          src = Object.fromEntries(src) || {};

          src.sources = JSON.parse(src?.sources);
          console.log(src);
        }

        images = src?.sources[0]?.images
          ?.map((it: string) => {
            return { id: it, extension: CommonFunction.url_extension(it) };
          })
          ?.filter((it: ChapterImageInfo) => {
            return !ignore_id.includes(it.id);
          });

        console.log(JSON.stringify(src, null, 2));
      }

      if (images.length == 0) {
        throw Errors.message("got empty image");
        for (const query of queries) {
          images = doc.selectText(query, "value").map((it) => {
            return { id: it, extension: CommonFunction.url_extension(it) };
          });

          if (images.length != 0) {
            break;
          }
        }
      }

      if (images.length == 0) {
        throw Errors.message("got empty image");
      }

      images.forEach((it) => task.push(it));
    } finally {
      await cf.close();
    }
  }

  async downloadImage(image: ChapterImageInfo): Promise<HttpRequest> {
    return {
      url: image.id,
      header: {
        Referer: this.domain,
        referer: this.domain,
      },
    };
  }

  async close(): Promise<void> {
    await this.client.close();
  }
}

export function initModule() {
  let flamescans = new MangaShiro(
    new RustHttpClient(),
    "9caa1926-d020-458a-8d43-276b36df7dbd",
    "Flamescans",
    "https://flamescans.org",
    true,
    new RustBrowser()
  );

  let manhwafreak = new MangaShiro(
    new RustHttpClient(),
    "a51ebfb8-9790-45d5-89cd-867c48a095c0",
    "ManhwaFreak",
    "https://manhwafreak.com",
    true,
    new RustBrowser()
  );

  let mmscans = new MangaShiro(
    new RustHttpClient(),
    "38ee90a6-e1f3-43e2-8447-8e090399d7d2",
    "MMScans",
    "https://mm-scans.org",
    true,
    new RustBrowser()
  );

  let realmscans = new MangaShiro(
    new RustHttpClient(),
    "3593adad-980d-454a-be48-9c42e7158032",
    "RealmScans",
    "https://realmscans.com",
    true, new RustBrowser(),
  )

  return [flamescans, manhwafreak, realmscans, mmscans];
}

export function initMadoModule() {
  return initModule().map((it) => new ResultModule(it));
}

function separateRight(value: string, delimiter: string): string {
  let x: number = value.indexOf(delimiter);
  if (x > -1) {
    x += delimiter.length - 1;
  }
  return value.substring(x + 1);
}

function getBetween(value: string, pairBegin: string, pairEnd: string): string {
  let n: number, x: number, max: number;
  let s: string, str: string;
  let lenBegin: number = pairBegin.length;
  let lenEnd: number = pairEnd.length;
  let lengthValue: number = value.length;

  if (value === pairBegin + pairEnd) {
    return ""; // nothing between
  }

  if (lengthValue < lenBegin + lenEnd) {
    return value;
  }

  s = separateRight(value, pairBegin);
  if (s === value) {
    return value;
  }

  n = s.indexOf(pairEnd);
  if (n === -1) {
    return value;
  }

  let result: string = "";
  x = 1;
  max = s.length - lenEnd + 1;
  for (n = 0; n < max; n++) {
    str = s.substring(n, n + lenEnd);
    if (str === pairEnd) {
      x--;
      if (x <= 0) {
        break;
      }
    }
    str = s.substring(n, n + lenBegin);
    if (str === pairBegin) {
      x++;
    }
    result += s.charAt(n);
  }
  return result;
}
