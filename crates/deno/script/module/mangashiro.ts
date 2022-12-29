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

      return {
        manga,
        chapters,
      };
    } catch (e) {
      console.log(e)
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

    let title = doc.selectString('//h1[@class="entry-title"]');
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
          scanlator: [],
        };
      });
  }

  async getChapterImage(id: string, task: ChapterTask): Promise<void> {
    let cf = new Cloudflare(this.browser);
    try {
      const queries = [
        '//*[@id="readerarea"]/p/img/@src',
        '//*[@id="readerarea"]//img/@src',
        '//*[@id="readerarea"]/p//img/@src',
      ];

      let response = await cf.get(id).waitForElementByXPath(queries.join("or")).get();

      let doc = new XHTMLPath(response.data);

      this.parse_404(id, doc);


      let images = [];
      for (const query of queries) {
        images = doc.selectText(query, "value").map((it) => {
          return { id: it, extension: CommonFunction.url_extension(it) };
        });

        if (images.length != 0) {
          break;
        }
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
      },
    };
  }

  async close(): Promise<void> {
    await this.client.close();
  }
}

export function initModule() {
  let asurascans = new MangaShiro(
    new RustHttpClient(),
    "7fa94290-99b7-4bbc-8413-f2d364294224",
    "Asurascans",
    "https://asurascans.com",
    true,
    new RustBrowser()
  );

  return [asurascans];
}

export function initMadoModule() {
  return initModule().map((it) => new ResultModule(it));
}
