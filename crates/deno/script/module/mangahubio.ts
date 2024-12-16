import { rx } from "verbose-regexp";
import { Browser } from "../deps/browser";
import { Cloudflare } from "../deps/cloudflare";
import { Errors } from "../deps/error";
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
  #(?<chapter>${FLOAT_REGEX.source}) - (?<title>.+)`;

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

class MangaHubIO implements HttpModule {
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

  async getInfo(id: string): Promise<MangaAndChapters> {
    let url = id;
    let browser = new Cloudflare(this.browser);
    try {
      const CHAPTER_QUERY =
        '//div[contains(@class, "mt-6")]//ul[@role="list"]//a';

      let current = 1;
      let pages = 0;

      // let manga: Manga;
      let chapters = [];

      let text = await browser
        .get(url)
        .waitForElementByXPath('//section[@id="comments"]')
        .get();

      let doc = new XHTMLPath(text.data);

      let manga = this.parse_info(url, doc);

      for (const it of doc.select('//div[contains(@id, "chapters-tab")]/div/div/ul/li//a[not(@title)]')) {
        let chapter = this.parse_chapter(it)
        chapters.push(chapter)

        // console.log(chapter)
      }

      // console.log(text.data)
      // console.log(manga);
      // console.log(chapters);
      // console.log(doc.select('//div[contains(@id, "chapters-tab")]'))
      // let chapters = this.parse_chapter

      // while (true) {
      //   let text = await browser
      //     .get(url)
      //     .waitForElementByXPath(`${CHAPTER_QUERY} or ${NOT_FOUND_XPATH}`)
      //     .get();
      //
      //   let doc = new XHTMLPath(text.data);
      //
      //   if (current == 1) {
      //     pages = parseIntOr(
      //       doc.selectString(
      //         '//span[contains(@class, "z-0")]/span[last()-1]/button'
      //       ),
      //       1
      //     );
      //
      //     manga = this.parse_info(url, doc);
      //   }
      //
      //   let chapterquery =
      //     '//div[contains(@class, "mt-6")]//ul[@role="list"]//a';
      //
      //   doc
      //     .select(chapterquery)
      //     .map((it) => this.parse_chapter(it))
      //     .forEach((it) => chapters.push(it));
      //
      //   current += 1;
      //
      //   if (current > pages) {
      //     break;
      //   }
      //
      //   url = `${id}?page=${current}`;
      // }

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
      title: doc.selectString('//div[@id="mangadetail"]//h1/text()'),
      types: "Series",
      authors: [
        doc.selectString(
          '//div[@id="mangadetail"]//div/span[contains(., "Author")]/following-sibling::span'
        ),
      ],
      artists: [
        doc.selectString(
          '//div[@id="mangadetail"]//div/span[contains(., "Artist")]/following-sibling::span'
        ),
      ],
      genres: doc.selectTextContent('//div[@id="mangadetail"]//div/p/a'),
      summary: doc.selectString(
        '//div[contains(@id, "chapters-tab")]//div/p'
      ),
      cover_link: doc.selectString('//div[@id="mangadetail"]//img/@src'),
      chapters: [],
    };
    // info.id = url;
    //
    // info.title = doc
    //   .selectString('//div[contains(@class, "container")]//h1')
    //   .trim();
    // info.cover_link = doc.selectString(
    //   '//div[contains(@class, "overflow-hidden")]/img/@src'
    // );
    //
    // info.authors = [];
    // info.artists = [];
    // info.genres = [];
    // info.summary = doc
    //   .selectString('//section/div[@aria-label="card"]//p')
    //   .trim();
    // info.types = "Series";

    return info;
  }

  parse_chapter(node: Node): Chapter {
    let doc = XHTMLPath.fromNode(node);
    let title = doc.selectString("span").trim();
    let { groups } = NAME_REGEX.exec(title) || {};
    console.log(groups);

    let ch = {
      title: null,
      volume: null,
      chapter: null,
    };

    if (groups?.chapter != null) {
      ch.chapter = groups.chapter.trim();

      let replace_title = `Chapter ${ch.chapter}`
      if (groups.title.startsWith(replace_title)) {
        groups.title = groups?.title.replace(replace_title, "")

        replace_title = " - "
        if (groups.title.startsWith(replace_title)) {
          groups.title = groups?.title.replace(replace_title, "")
        }
      }

      if(groups?.title?.length != 0) {
        ch.title = groups?.title.trim();
      }
      ch.volume = groups?.volume;
    } else {
      ch.title = title;
    }
    console.log(ch);

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
        .waitForElementByXPath("footer")
        .get();

      let doc = new XHTMLPath(response.data);

      this.parse_404(id, doc);

      let queries = '//div[contains(@id, "mangareader")]//img/@src';

      let images = doc.selectText(queries, "value").map((it) => {
        return {
          id: it,
          extension: CommonFunction.url_extension(it),
          name: null,
        };
      });

      console.log(images)

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


      if (images.length == 0) {
        throw Errors.message("got empty image");
      }

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
        Referer: 'imgx.mghubcdn.com',
      },
    };
  }

  async close() {
    await this.client.close();
  }
}

export function initModule() {
  return [
    new MangaHubIO(
      new RustHttpClient(),
      "b2f6f826-c557-4b33-b5fa-d85a063f52d7",
      "MangaHubIO",
      "https://mangahub.io",
      new RustBrowser()
    ),
  ];
}

export function initMadoModule() {
  return initModule().map((it) => new ResultModule(it));
}
