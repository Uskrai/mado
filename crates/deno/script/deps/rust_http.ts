import { Result, ResultFromJson } from "./error";
import { HttpClient, HttpRequest, HttpResponse } from "./http";
import { Resource } from "./resource";

type ResponseDecl = {
  status: number;
  rid: number;
  url: string;
};

export class RustHttpResponse implements HttpResponse {
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
    return ResultFromJson(await Deno.core.ops.op_http_response_text(this.rid));
  }

  async text_data(): Promise<string> {
    return await this.text().then((it) => it.data);
  }

  async json(): Promise<Result<any>> {
    return await this.text().then((it) => it.map((text) => JSON.parse(text)));
  }

  async json_data(): Promise<any> {
    return await this.json().then((it) => it.data);
  }

  async close(): Promise<void> {
    //TODO
  }
}

export class RustHttpClient extends Resource implements HttpClient {
  constructor(rid: number = null) {
    if (rid == null) {
      rid = Deno.core.ops.op_http_client_new();
    }

    super(rid, "op_http_client");
  }

  async get(request: HttpRequest): Promise<HttpResponse> {
    return new RustHttpResponse(
      ResultFromJson(
        await Deno.core.opAsync("op_http_client_get", this.rid, request)
      )
    );
  }

  async close(): Promise<void> {
    this.decrement_strong_count().data;
  }

  clone() {
    let rid = this.increment_strong_count();
    return new RustHttpClient(rid);
  }
}
