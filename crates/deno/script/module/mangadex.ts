import {
  Chapter,
  ChapterTask,
  CommonClosure,
  CommonFunction,
  Error,
  HttpClient,
  HttpModule,
  HttpRequest,
  Manga,
  MangaAndChapters,
} from "../deps/index";

const API_URL = "https://api.mangadex.org";
const API_PARAMS = "includes[]=author&includes[]=artist&includes[]=cover_art";
const COVER_URL = "https://uploads.mangadex.org/covers";
const HEX = "[0-9a-fA-F]";
const REGEX_ID = new RegExp(
  `${HEX}{8}-${HEX}{4}-${HEX}{4}-${HEX}{4}-${HEX}{12}`
);

export class MangaDex extends HttpModule {
  constructor(uuid: string, name: string, domain: string) {
    super(uuid, name, domain, new HttpClient());
  }

  parse_response(url: string, json: any) {
    if (json?.result == "error") {
      let message = CommonFunction.query(json, "$.errors..detail").join(",");
      throw Error.unexpected_error(url, message);
    }
    return json;
  }

  async get_info(id: string): Promise<MangaAndChapters> {
    id = REGEX_ID.exec(id)?.at(0);
    if (id == null) {
      throw Error.invalid_url(id);
    }

    let manga_future = this.get_manga_info(id);
    let chapter_future = this.get_chapter_info(id);

    let manga = await manga_future;
    let chapters = await chapter_future;

    return {
      manga,
      chapters
    };
  }

  async get_manga_info(id: string): Promise<Manga> {
    let url = `${API_URL}/manga/${id}?${API_PARAMS}`;
    let response = await this.client.get({
      url,
    });

    let json = await response.json();
    this.parse_response(url, json);

    let query = CommonClosure.query(json);

    let title =
      query("$.data.attributes.title.en") ||
      query("$.data.attributes.title.ja") ||
      query("$.data.attributes.title[0]") ||
      "";

    let summary =
      query("$.data.attributes.description.en") ||
      query("$.data.attributes.description.ja") ||
      query("$.data.attributes.description[0]") ||
      "";

    let authors = query(
      "$.data.relationships[?(@.type=='author')].attributes.name"
    );

    let artists = query(
      "$.data.relationships[?(@.type=='artist')].attributes.name"
    );

    let genres = query("$.data.attributes.tags..attributes.name.en");
    {
      let g = [
        query("$.data.attributes.contentRating"),
        query("$.data.attributes.publicationDemographic"),
      ];

      for (const it of g) {
        if (it != null) {
          genres.push(it);
        }
      }
    }

    let cover_link = query(
      "$.data.relationships[?(@.type=='cover_art')].attributes.fileName"
    );

    cover_link = `${COVER_URL}/${id}/${cover_link.at(0)}`;

    let info: Manga = {
      id,
      title,
      types: "Series",
      authors,
      artists,
      genres,
      summary,
      cover_link,
      chapters: [],
    };

    return info;
  }

  async get_chapter_info(id: string): Promise<Array<Chapter>> {
    let chapters = [];
    let total = 1;
    let offset = 0;
    let request_limit = 2;

    var request = [];

    while (total > offset) {
      let url =
        `${API_URL}/manga/${id}/feed?${API_PARAMS}&offset=${offset}&limit=500` +
        "&contentRating[]=safe&contentRating[]=suggestive" +
        "&contentRating[]=erotica&contentRating[]=pornographic" +
        "&includes[]=scanlation_group&order[volume]=asc&order[chapter]=asc" +
        "&translatedLanguage[]=en";

      let response = await this.client.get({ url });

      let json = await response.json();

      this.parse_response(url, json);

      total = json["total"];
      offset = json["limit"];

      json.data.forEach((it) => {
        let ch = this.parse_chapter_info(it);
        chapters.push(ch);
      });
    }

    return chapters;
  }

  parse_chapter_info(json): Chapter {
    let attr = CommonFunction.query(json, "$.attributes");

    let info: Chapter = {
      id: json.id,
      title: attr.title,
      volume: attr.volume,
      chapter: attr.title,
      language: attr.translatedLanguage,
      scanlator: [],
    };

    info.scanlator = CommonFunction.query(
      json,
      "$.relationships[?(@.type=='scanlation_group')]..name"
    );

    return info;
  }

  async get_chapter_image(id: string, task: ChapterTask): Promise<void> {
    let url = `${API_URL}/at-home/server/${id}`;
    let response = await this.client.get({ url });
    let json = await response.json();
    this.parse_response(url, json);

    let query = CommonClosure.query(json);

    let server = query("$.baseUrl");
    let hash = query("$.chapter.hash");

    query("$.chapter.data").forEach((it: string) => {
      let id = `${server}/data/${hash}/${it}`;
      task.push({
        id,
        extension: CommonFunction.url_extension(id),
        name: null,
      });
    });

  }

  async download_image(image: any): Promise<HttpRequest> {
    return {
      url: image.id,
      header: undefined,
    };
  }

  close() {
    this.client.close();
  }
}

export function initMadoModule() {
  return [
    new MangaDex(
      "07bd7f6b-12a1-48f1-9873-f175d4f76c9a",
      "MangaDex",
      "https://mangadex.org"
    ),
  ];
}
