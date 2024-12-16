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
  sleep,
  XHTMLPath,
} from "../deps";
import { Browser } from "../deps/browser";
import { Cloudflare } from "../deps/cloudflare";
import { Errors } from "../deps/error";
import { ResultModule } from "../deps/module";
import { RustBrowser } from "../deps/rust_browser";

const NOT_FOUND = "//div/div[class=content]/div[class=notf]/img";

class Madara implements HttpModule {
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
        .waitForElementByXPath(`${NOT_FOUND} or footer`)
        .get();

      let doc = new XHTMLPath(response.data);

      let manga = this.parse_info(url, doc);
      let chapters = this.parse_chapter(url, doc);

      if (this.reverseChapter) {
        chapters.reverse();
      }

      console.log(manga, chapters);
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

    function selectMap<T>(
      array: string[],
      map: (string: string) => T
    ): T | null {
      let result = null;

      for (const it of array) {
        result = map(it);

        if (result != null) {
          break;
        }
      }

      return result;
    }

    function selectString(array: string[]) {
      return selectMap(array, (string) => {
        let res = doc.selectString(string);
        if (res.length == 0) {
          return null;
        }
        return res;
      });
    }

    function select(array: string[]) {
      return selectMap(array, (string) => {
        let arr = doc.selectTextContent(string);
        if (arr.length == 0) {
          return null;
        }
        return arr;
      });
    }

    function selectTextData(array: string[]) {
      return selectMap(array, (string) => {
        let arr = doc.selectTextData(string) as string[];
        if (arr.length == 0) {
          return null;
        }
        return arr;
      });
    }

    let title = selectTextData([
      '//div[@class="post-title"]/*[self::h1 or self::h2 or self::h3]/text()',
      '//*[@id="manga-title"]/h1/text()',
    ])
      ?.join(" ")
      .trim();

    let cover_link = select([
      '//div[@class="summary_image"]//img/@data-src',
      '//div[@class="summary_image"]//img/@data-lazy-src',
      '//div[@class="summary_image"]//img/@src',
      // x.XPathString('//div[@class="my_profile-manga"]/@style'):match('background%-image:URL%((.-)%)')
      '//div[@class="series-img"]/img/@data-src',
    ]).at(0);

    let authors =
      select([
        '//div[@class="author-content"]/a',
        '//div[@class="summary-heading-creator"]/a',
        '//div[@class="manga-authors"]/a',
      ]) ?? []; // TODO
    let artists = select(['//div[@class="artist-content"]/a']) ?? [];
    let genres = select(['//div[@class="genres-content"]/a']) ?? [];

    let summary = selectString([
      '//div[contains(@class,"description-summary")]//p',
      '//div[contains(@class,"manga-summary")]//p',
    ]);
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
    // let query = '//*[@id="chapterlist"]//*[@class="eph-num"]/a';
    const queries = ['//li[contains(@class, "wp-manga-chapter")]/a'];
    let chapters = [];

    for (const query of queries) {
      chapters = doc.select(query).map((it: Element) => {
        let doc = XHTMLPath.fromNode(it);
        return {
          id: doc.selectString("@href"),
          title: doc.selectString("text()").trim(),
          chapter: null,
          volume: null,
          language: "en",
          scanlator: [this.name],
        };
      });
    }

    if (chapters.length == 0) {
      // chapters = doc
      //   .select('//*[@id="chapterlist"]//*[@class="eph-num"]/a')
      //   .map((it: Element) => {
      //     const path = XHTMLPath.fromNode(it);
      //     return {
      //       id: it.getAttribute("href"),
      //       title: path.selectString('span[@class="chapternum"]'),
      //       volume: null,
      //       chapter: null,
      //       language: "en",
      //       scanlator: [],
      //     };
      //   });
    }

    return chapters;
  }

  async getChapterImage(id: string, task: ChapterTask): Promise<void> {
    let cf = new Cloudflare(this.browser);
    try {
      const queries = [
        '//div[@class="reading-content"]//img/@data-src',
        '//div[contains(@class, "page-break")]/img/@data-src',
        '//div[contains(@class, "page-break")]/img/@src',
      ];

      let response = await cf
        .get(id)
        .waitForElementByXPath(
          [...queries, `script[contains(., "ts_reader")]`].join("or")
        )
        .get();

      let doc = new XHTMLPath(response.data);

      this.parse_404(id, doc);

      let images = [];

      if (images.length == 0) {
        for (const query of queries) {
          images = doc.selectText(query, "value").map((it) => {
            return {
              id: it.trim(),
              extension: CommonFunction.url_extension(it.trim()),
            };
          });

          if (images.length != 0) {
            break;
          }
        }
      }

      console.log(JSON.stringify(images, null, 4));
      if (images.length == 0) {
        throw Errors.message("got empty image");
      }

      images.forEach((it) => task.push(it));
    } finally {
      await cf.close();
    }
  }

  async downloadImage(image: ChapterImageInfo): Promise<HttpRequest> {
    await sleep(1000);
    return {
      url: image.id,
      header: {
        Referer: this.domain,
      },
    };
  }

  async close(): Promise<void> {
    await this.client.close();
  }
}

export function initModule() {
  let nitroscans = new Madara(
    new RustHttpClient(),
    "90abf9af-4894-4b54-87e1-e15ca5a253fb",
    "NitroScans",
    "https://nitroscans.com",
    true,
    new RustBrowser()
  );

  const leviatanscans = new Madara(
    new RustHttpClient(),
    "06e732a1-ccdc-4894-877e-5dee6e065df1",
    "LeviatanScans",
    "https://en.leviatanscans.com",
    true,
    new RustBrowser()
  );

  return [nitroscans, leviatanscans];
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
