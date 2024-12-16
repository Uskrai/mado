import { rx } from "verbose-regexp";
import { Browser } from "../deps/browser";
import { Cloudflare } from "../deps/cloudflare";
import {
  Chapter,
  ChapterImageInfo,
  ChapterTask,
  CommonFunction,
  Error,
  HttpClient,
  HttpModule,
  HttpRequest,
  Manga,
  MangaAndChapters,
  RustHttpClient,
  sleep,
  XHTMLPath,
} from "../deps/index";
import { ResultModule } from "../deps/module";
import { RustBrowser } from "../deps/rust_browser";

const FLOAT_REGEX = rx`[+-]?\d+(\.\d+)?`;

const NAME_REGEX = rx`
  Chapter (?<chapter>${FLOAT_REGEX.source})
`;

function parseIntOr(string: string, or: number): number {
  try {
    return parseInt(string) || or;
  } catch (_) {
    return or;
  }
}

function isIUAM(content: string) {
  return content.match("\\/cdn-cgi\\/images\\/trace\\/managed\\/");
}

const NOT_FOUND_XPATH = "//div//div[contains(text(), '404')]";
const TITLE_XPATH =
  '//div[contains(@class, "col-span-12")]/div[contains(@class, "text-center")]/span';

class AsuraScans implements HttpModule {
  // public browser: BrowserWrapper;
  constructor(
    public client: HttpClient,
    public uuid: string,
    public name: string,
    public domain: string,
    public browser: Browser,
  ) {
    // this.browser = new BrowserWrapper(browser);
  }

  async getInfo(id: string): Promise<MangaAndChapters> {
    let url = id;
    let browser = new Cloudflare(this.browser);
    try {
      let manga: Manga;
      let chapters = [];

      let response = await browser
        .get(url)
        .waitForElementByXPath(`${NOT_FOUND_XPATH} or //h1[@class=entry-title]`)
        .get();

      let doc = new XHTMLPath(response.data);

      manga = this.parse_info(url, doc);

      let chapterquery = '//div[contains(@class, "scrollbar-thin")]/div';

      doc
        .select(chapterquery)
        .map((it) => this.parse_chapter(url, it))
        .forEach((it) => chapters.push(it));

      console.log(manga, chapters)
      chapters.reverse();

      return {
        manga,
        chapters,
      };
    } finally {
      await browser.close();
    }
  }

  parse_404(url: string, doc: XHTMLPath) {
    if (doc.select(NOT_FOUND_XPATH).length) {
      throw Error.request_error(url, "404 PAGE NOT FOUND");
    }
  }

  parse_info(url: string, doc: XHTMLPath): Manga {
    this.parse_404(url, doc);

    let info: Manga = {
      id: url,
      title: doc.selectString("//h1"),
      types: "Series",
      authors: [],
      artists: [],
      genres: [],
      summary: "",
      cover_link: "",
      chapters: [],
    };
    info.id = url;

    info.title = doc.selectString(TITLE_XPATH).trim();
    info.cover_link = doc.selectString(
      '//div[contains(@class, "overflow-hidden")]/img/@src',
    );

    info.authors = [];
    info.artists = [];
    info.genres = [];
    info.summary = doc
      .selectString('//section/div[@aria-label="card"]//p')
      .trim();
    info.types = "Series";

    return info;
  }

  parse_chapter(url_id: string, node: Node): Chapter {
    let doc = XHTMLPath.fromNode(node);
    let title = doc.selectString('.//h3/text()').trim();
    let { groups } = NAME_REGEX.exec(title) || {};

    let ch = {
      title: null,
      volume: null,
      chapter: null,
    };

    if (groups?.chapter != null) {
      ch.chapter = groups.chapter;
    } else {
      ch.title = title;
    }

    let info: Chapter = {
      id: CommonFunction.url_join(url_id, doc.selectString(".//a/@href")),
      ...ch,
      language: "en",
      scanlator: [this.name],
    };

    return info;
  }

  async getChapterImage(id: string, task: ChapterTask) {
    let browser = new Cloudflare(this.browser);
    try {
      let response = await browser
        .get(id)
        .waitForElementByXPath('div[contains(@class, "py-4")]')
        .get();

      let doc = new XHTMLPath(response.data);

      this.parse_404(id, doc);

      let queries = '//img[contains(@class, "object-cover mx-auto")]/@src';

      let images = doc.selectText(queries, "value").map((it) => {
        return {
          id: it,
          extension: CommonFunction.url_extension(it),
          name: null,
        };
      });

      // let images = [];
      // for (const query of queries) {
      //   images = doc.selectText(query, "value").map((it) => {
      //     return { id: it, extension: CommonFunction.url_extension(it) };
      //   });
      //
      //   if (images.length != 0) {
      //     break;
      //   }
      // }

      images.forEach((it) => task.push(it));
    } finally {
      console.log("finally", await browser.close());
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

  async close() {
    await this.client.close();
  }
}

export function initModule() {
  return [
    new AsuraScans(
      new RustHttpClient(),
      "7fa94290-99b7-4bbc-8413-f2d364294224",
      "Asurascans",
      "https://asuracomic.net",
      new RustBrowser(),
    ),
  ];
}

export function initMadoModule() {
  return initModule().map((it) => new ResultModule(it));
}
