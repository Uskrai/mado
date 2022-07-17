import { catchAndReturn } from "./error";
import { HttpClient, HttpRequest } from "./http";
import { ChapterTask, Manga, MangaAndChapters, RustChapterTask } from "./manga";

export abstract class Module {
  constructor(
    public uuid: string,
    public name: string,
    public domain: string,
    public client: any
  ) {}
  abstract get_info(id: string): Promise<MangaAndChapters>;

  async getInfo(id: string) {
    return await catchAndReturn(() => this.get_info(id));
  }

  abstract get_chapter_image(
    id: string,
    task: ChapterTask
  ): Promise<ChapterTask>;

  async getChapterImage(id: string, task: ChapterTask) {
    return await catchAndReturn(() => this.get_chapter_image(id, task));
  }

  async getChapterImageRust(id: string, task_rid: number) {
    return this.getChapterImage(id, new RustChapterTask(task_rid));
  }

  abstract download_image(info: object): Promise<HttpRequest>;

  async downloadImage(info: object) {
    return await catchAndReturn(() => this.download_image(info));
  }
}

export abstract class HttpModule extends Module {
  declare client: HttpClient;

  constructor(uuid: string, name: string, domain: string, client: HttpClient) {
    super(uuid, name, domain, client);
  }
}

// Module Wrapper that just use underlying module to operate
// with different uuid
export class ModuleWrapper extends Module {
  constructor(
    uuid: string,
    name: string,
    domain: string,
    public module: Module
  ) {
    super(uuid, name, domain, module.client.clone());
  }

  async get_info(id: string): Promise<MangaAndChapters> {
    return await this.module.get_info(id);
  }

  async get_chapter_image(id: string, task: ChapterTask): Promise<ChapterTask> {
    return await this.module.get_chapter_image(id, task);
  }

  async download_image(info: object): Promise<HttpRequest> {
    return await this.module.download_image(info);
  }

  close() {
    this.client.close();
  }
}

export function owo() {}
