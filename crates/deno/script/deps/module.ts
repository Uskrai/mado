import { catchAndReturn} from "./error";
import { HttpClient, HttpRequest } from "./http";
import { ChapterImageInfo, ChapterTask, MangaAndChapters} from "./manga";
import { RustChapterTask } from './rust_chapter_task';

export interface Module {
  uuid: string;
  name: string;
  domain: string;
  client: any;

  getInfo(id: string): Promise<MangaAndChapters>;
  getChapterImage(id: string, task: ChapterTask): Promise<void>;
  downloadImage(info: object): Promise<HttpRequest>;
  close(): Promise<void>;
}

export class ResultModule {
  public name: string;
  public client: any;
  public uuid: string;
  public domain: string;

  constructor(public module: Module) {
    this.name = module.name;
    this.client = module.client;
    this.uuid = module.uuid;
    this.domain = module.domain;
  }

  async getInfo(id: string) {
    return await catchAndReturn(() => this.module.getInfo(id));
  }

  async getChapterImage(id: string, task: ChapterTask) {
    return await catchAndReturn(() => this.module.getChapterImage(id, task));
  }

  async getChapterImageRust(id: string, task_rid: number) {
    return await this.getChapterImage(id, new RustChapterTask(task_rid));
  }

  async downloadImage(image: ChapterImageInfo) {
    return await catchAndReturn(() => this.module.downloadImage(image));
  }

  async close() {
    return await catchAndReturn(() => this.module.close());
  }
}

export interface HttpModule extends Module {
  client: HttpClient;
}

// Module Wrapper that just use underlying module to operate
// with different uuid
export class ModuleWrapper implements Module {
  public client: any;
  constructor(
    public uuid: string,
    public name: string,
    public domain: string,
    public module: Module
  ) {
    this.client = module.client.clone();
  }

  async getInfo(id: string): Promise<MangaAndChapters> {
    return await this.module.getInfo(id);
  }

  async getChapterImage(id: string, task: ChapterTask): Promise<void> {
    return await this.module.getChapterImage(id, task);
  }

  async downloadImage(info: object): Promise<HttpRequest> {
    return await this.module.downloadImage(info);
  }

  async close(): Promise<void> {
    await this.module.close();
  }
}
