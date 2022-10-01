import {
  CommonFunction,
  Error,
  HttpClient,
  HttpModule,
  VRegex as rx,
  XHTMLPath,
  ModuleWrapper,
  Manga,
  ChapterTask,
  ChapterImageInfo,
  HttpRequest,
  Chapter,
  MangaAndChapters,
} from "../deps/index";

const FLOAT_REGEX = `/^[+-]?\d+(\.\d+)?$/`;

const NAME_REGEX = rx`
    (Vol\.(?<vol>\d+)\s+?)?     // Get volume (optional)
    (Chapter\s+?(?<ch>${FLOAT_REGEX}))     // Get Chapter
    ((\s+)?:(\s+)?              // Get Title (optional)
    (?<title>.*))?
`;

class MangaNato extends HttpModule {
  constructor(uuid: string, name: string, domain: string) {
    super(uuid, name, domain, new HttpClient());
  }

  async get_info(url: string): Promise<MangaAndChapters> {
    let response = await this.client.get({
      url: url,
    });

    let text = await response.text();
    // let response = self.client.get(url.clone()).send().await.unwrap().text().await;

    let doc = new XHTMLPath(text);

    let manga = this.parse_info(url, doc);

    let chapters = doc
      .select(
        '//ul[@class="row-content-chapter"]/li/a[contains(@class, "chapter-name")]'
      )
      .map((it) => this.parse_chapter(it));

    return {
      manga,
      chapters
    };
  }

  parse_404(url: string, doc: XHTMLPath) {
    if (doc.select('//p[contains(., "404 - PAGE NOT FOUND")]').length) {
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

    info.title = doc.selectString("//h1");
    info.cover_link = doc.selectString('//span[@class="info-image"]/img/@src');

    info.authors = doc.selectTextData(
      '//td[contains(., "Author(s)")]/following-sibling::td/a/text()'
    );
    info.artists = info.authors;
    info.genres = doc.selectTextData(
      '//td[contains(., "Genres")]/following-sibling::td/a/text()'
    );
    // info.status    = doc.selectString('//td[contains(., "Status")]/following-sibling::td')
    info.summary = doc
      .selectString(
        '//div[@class="panel-story-info-description"]/text()[last()]'
      )
      .trim();
    info.types = "Series";

    return info;
  }

  parse_chapter(node: Node): Chapter {
    let doc = XHTMLPath.fromNode(node);

    let info: Chapter = {
      id: doc.selectString("@href"),
      title: doc.selectString("text()"),
      volume: null,
      chapter: null,
      language: "en",
      scanlator: [],
    };

    let { groups } = NAME_REGEX.exec(info.title) || {};

    if (groups) {
      info.volume = groups.vol;
      info.chapter = groups.ch;
      info.title = groups.title;
    }

    return info;
  }

  async get_chapter_image(id: string, task: ChapterTask) {
    let response = await this.client.get({ url: id });

    let doc = new XHTMLPath(await response.text());

    this.parse_404(id, doc);

    let queries = [
      '//div[@id="vungdoc"]/img[@title]/@src',
      '//div[@class="vung_doc"]/img[@title]/@src',
      '//div[@class="container-chapter-reader"]/img[@title]/@src',
      '//div[@id="vungdoc"]/img[@title]/@data-src',
    ];

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
  }

  async download_image(image: ChapterImageInfo): Promise<HttpRequest> {
    return {
      url: image.id,
      header: {
        Referer: this.domain,
      },
    };
  }
}

export function initMadoModule() {
  let readmanganato = new MangaNato(
    "fa8bb4d1ceea4c8fa0e98c00755f95d4",
    "Manganato",
    "https://readmanganato.com"
  );

  let manganato = new ModuleWrapper(
    "d690b8c3-03bb-4129-b245-48aadae9eba9",
    "Manganato",
    "https://manganato",
    readmanganato
  );

  return [
    manganato,
    readmanganato,
    new MangaNato(
      "74674292e13c496699b8c5e4efd4b583",
      "MangaKakalot",
      "https://mangakakalot.com"
    ),
    new MangaNato(
      "ed4175a390e74aedbe4b4f622f3767c6",
      "MangaKakalots",
      "https://mangakakalots.com"
    ),
    new MangaNato(
      "2234588abb544fc6a279c7811f2a9733",
      "MangaBat",
      "https://m.mangabat.com"
    ),
  ];
}
