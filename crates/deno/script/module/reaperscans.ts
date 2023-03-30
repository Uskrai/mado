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

class ReaperScans implements HttpModule {
  // public browser: BrowserWrapper;
  constructor(
    public client: HttpClient,
    public uuid: string,
    public name: string,
    public domain: string,
    public browser: Browser
  ) {
    // this.browser = new BrowserWrapper(browser);
  }

  async getInfo(url: string): Promise<MangaAndChapters> {
    let browser = new Cloudflare(this.browser);
    try {
      const CHAPTER_QUERY =
        '//div[contains(@class, "mt-6")]//ul[@role="list"]//a';

      let current = 1;
      let pages = 0;

      let manga: Manga;
      let chapters = [];

      while (true) {
        let text = await browser
          .get(url)
          .waitForElementByXPath(`${CHAPTER_QUERY} or ${NOT_FOUND_XPATH}`)
          .get();

        let doc = new XHTMLPath(text.data);

        if (current == 1) {
          pages = parseIntOr(
            doc.selectString(
              '//span[contains(@class, "z-0")]/span[last()-1]/button'
            ),
            1
          );

          manga = this.parse_info(url, doc);
        }

        let chapterquery =
          '//div[contains(@class, "mt-6")]//ul[@role="list"]//a';

        doc
          .select(chapterquery)
          .map((it) => this.parse_chapter(it))
          .forEach((it) => chapters.push(it));

        current += 1;

        if (current > pages) {
          break;
        }

        url = `${url}?page=${current}`;
      }

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

    info.title = doc
      .selectString('//div[contains(@class, "container")]//h1')
      .trim();
    info.cover_link = doc.selectString(
      '//div[contains(@class, "overflow-hidden")]/img/@src'
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

  parse_chapter(node: Node): Chapter {
    let doc = XHTMLPath.fromNode(node);
    let title = doc.selectString("(.//p)[1]").trim();
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
      id: doc.selectString("@href"),
      ...ch,
      language: "en",
      scanlator: [],
    };

    return info;
  }

  async getChapterImage(id: string, task: ChapterTask) {
    let browser = new Cloudflare(this.browser);
    try {
      let response = await browser
        .get(id)
        .waitForElementByXPath("p[class=py-4]")
        .get();

      let doc = new XHTMLPath(response.data);

      this.parse_404(id, doc);

      let queries = '//img[contains(@class, "max-w-full")]/@src';

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
    new ReaperScans(
      new RustHttpClient(),
      "be1e2e81-5e6a-45fc-a843-ebc600245a27",
      "ReaperScans",
      "https://reaperscans.com",
      new RustBrowser()
    ),
  ];
}

export function initMadoModule() {
  return initModule().map((it) => new ResultModule(it));
}
