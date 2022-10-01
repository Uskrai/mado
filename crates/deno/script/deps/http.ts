import { Errors, Result, ResultFromJson } from "./error";
import { Resource } from "./resource";

type ResponseDecl = {
  status: number;
  rid: number;
  url: string;
};

export class HttpResponse {
  constructor(public result: Result<ResponseDecl>) {}

  get data(): ResponseDecl {
    return this.result.throw();
  }

  get status() {
    return this.data.status;
  }

  get rid() {
    return this.data.rid;
  }

  get url() {
    return this.data.url;
  }

  async text(): Promise<string> {
    return await Deno.core.opAsync("op_http_response_text", this.data.rid);
  }

  async json(): Promise<any> {
    let text = await this.text();
    return JSON.parse(text);
  }
}

export class HttpClient extends Resource {
  constructor(rid = null) {
    if (rid == null) {
      rid = Deno.core.opSync("op_http_client_new");
    }
    super(rid, "op_http_client");
  }

  async get(request: object): Promise<HttpResponse> {
    return new HttpResponse(
      ResultFromJson(
        await Deno.core.opAsync("op_http_client_get", this.rid, request)
      )
    );
  }

  close() {
    this.decrement_strong_count();
  }

  clone() {
    return new HttpClient(this.increment_strong_count());
  }
}

export interface HttpRequest {
  url: string;
  header: Record<string, string>;
}
