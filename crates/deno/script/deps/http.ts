import { Result, ResultFromJson } from "./error";
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

  async text(): Promise<Result<string>> {
    return ResultFromJson(
      await Deno.core.opAsync("op_http_response_text", this.rid)
    );
  }

  async text_data(): Promise<string> {
    return await this.text().then(it => it.data);
  }

  async json(): Promise<Result<any>> {
    return await this.text().then((it) => it.map((text) => JSON.parse(text)));
  }

  async json_data(): Promise<any> {
    return await this.json().then(it => it.data);
  }
}

export class HttpClient extends Resource {
  constructor(rid: number = null) {
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

  close(): Result<void> {
    return this.decrement_strong_count();
  }

  clone() {
    let rid = this.increment_strong_count();
    return new HttpClient(rid);
  }
}

export interface HttpRequest {
  url: string;
  header: Record<string, string>;
}
