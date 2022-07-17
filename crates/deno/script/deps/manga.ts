export interface ChapterTask {
  push(image: ChapterImageInfo): void;
}

export class RustChapterTask {
  rid: number;

  constructor(rid: number) {
    this.rid = rid;
  }

  push(image: object) {
    return Deno.core.opSync("op_mado_chapter_task_add", this.rid, image);
  }
}

export interface Manga {
  id: string;
  title: string;
  types: "Series" | "Anthology";
  authors: Array<string>;
  artists: Array<string>;
  genres: Array<string>;
  summary: string | null;
  cover_link: string | null;
  chapters: Array<any>;
};

export interface MangaAndChapters {
  manga: Manga;
  chapters: Array<Chapter>;
};

export interface Chapter {
  id: string;
  title: string | null;
  volume: string | null;
  chapter?: string | null;
  language: string;
  scanlator: Array<string>;
}

export interface ChapterImageInfo {
  id: string;
  extension: string;
  name: string | null;
}
