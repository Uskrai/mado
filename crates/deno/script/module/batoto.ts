import aes from "crypto-js/aes";
import enc from "crypto-js/enc-utf8";
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
  VRegex as rx,
  XHTMLPath,
} from "../deps/index";
import { ResultModule } from "../deps/module";

const FLOAT_REGEX = rx`[+-]?\d+(\.\d+)?`;

const NAME_REGEX = rx`
  (Volume\s+(?<vol>\d+)\s+?)?     // Get volume (optional)
  (Chapter\s+(?<ch>${FLOAT_REGEX.source}))     // Get Chapter
  ((\s+)?:(\s+)?              // Get Title (optional)
  (?<title>.*))?
`;

class Batoto implements HttpModule {
  constructor(
    public client: HttpClient,
    public uuid: string,
    public name: string,
    public domain: string,
    public reverseChapter: boolean,
  ) {}

  async getInfo(url: string): Promise<MangaAndChapters> {
    let response = await this.client.get({
      url: url,
    });

    let text = await response.text_data();
    // let response = self.client.get(url.clone()).send().await.unwrap().text().await;

    let doc = new XHTMLPath(text);

    let manga = this.parse_info(url, doc);

    let language = doc.selectString(
      '//div[@class="attr-item" and (./b="Translated Language:")]/span'
    );

    let chapters = doc
      .select(
        '//div[contains(@class, "episode-list")]/div[@class="main"]/div/a'
      )
      .map((it) => this.parse_chapter(it, language));

    if (this.reverseChapter) {
      chapters.reverse();
    }

    return {
      manga,
      chapters,
    };
  }

  parse_404(url: string, doc: XHTMLPath) {
    if (doc.doc.firstChild.textContent == "404 Page Not Found (1)") {
      throw Error.request_error(url, "404 PAGE NOT FOUND");
    }
  }

  parse_info(url: string, doc: XHTMLPath): Manga {
    this.parse_404(url, doc);

    let info: Manga = {
      id: url,
      title: "",
      types: "Series",
      authors: [],
      artists: [],
      genres: [],
      summary: "",
      cover_link: "",
      chapters: [],
    };
    info.id = url;

    info.title = doc.selectString('//h3[contains(@class, "item-title")]/a');
    info.cover_link = doc.selectString(
      '//div[contains(@class, "attr-cover")]/img/@src'
    );

    info.authors = doc.selectTextContent(
      '//div[@class="attr-item" and (./b="Authors:")]/span/a'
    );
    info.artists = doc.selectTextContent(
      '//div[@class="attr-item" and (./b="Artists:")]/span/a'
    );

    info.genres = doc.selectTextContent(
      '//div[@class="attr-item" and (./b="Genres:")]/span/span'
    );
    info.summary = doc.selectString('//div[@class="limit-html"]').trim();
    info.types = "Series";

    return info;
  }

  parse_chapter(node: Node, language: string): Chapter {
    let doc = XHTMLPath.fromNode(node);

    let info: Chapter = {
      id: this.sanitizeChapterId(doc.selectString("@href")),
      title: doc.selectString("normalize-space()"),
      volume: null,
      chapter: null,
      language: "en",
      scanlator: [],
    };

    let regex_result = NAME_REGEX.exec(info.title);
    let { groups } = regex_result || {};

    if (groups) {
      info.volume = groups.vol;
      info.chapter = groups.ch;
      info.title = groups.title;
    }

    return info;
  }

  sanitizeChapterId(id: string) {
    const DOMAIN_PREFIX = this.domain;

    if (id.startsWith(DOMAIN_PREFIX)) {
      id = id.slice(DOMAIN_PREFIX.length);
    }

    if (id.startsWith("/")) {
      id = id.slice(1);
    }

    return id;
  }

  async getChapterImage(id: string, task: ChapterTask) {
    let response = await this.client.get({
      url: `${this.domain}/${this.sanitizeChapterId(id)}`,
    });

    let text = await response.text_data();

    let doc = new XHTMLPath(text);
    this.parse_404(id, doc);

    const script = doc.selectString('//script[contains(.,"const batoPass")]');
    let result = new Function(
      "window",
      `
        ${script}
        return {
          word: batoWord, pass: batoPass, images: imgHttps
        }
      `
    )({});

    let encrypt: Array<string> = JSON.parse(
      aes.decrypt(result.word, result.pass).toString(enc)
    );

    (result.images as Array<string>)
      .map((url: string, i: number) => {
        let id = `${url}?${encrypt[i]}`;
        return {
          id,
          extension: CommonFunction.url_extension(id),
          name: null,
        };
      })
      .forEach((it) => task.push(it));
  }

  async downloadImage(image: ChapterImageInfo): Promise<HttpRequest> {
    return {
      url: image.id,
    };
  }

  async close() {
    await this.client.close();
  }
}

export function initModule() {
  return [
    new Batoto(
      new RustHttpClient(),
      "3a8f7bb48682485192a06c9d620a9026",
      "Batoto",
      "https://bato.to",
      true,
    ),
  ];
}

export function initMadoModule() {
  return initModule().map((it) => new ResultModule(it));
}
