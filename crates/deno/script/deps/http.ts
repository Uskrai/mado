import { Result, ResultFromJson } from "./error";
import { Resource } from "./resource";

type ResponseDecl = {
  status: number;
  rid: number;
  url: string;
};

export interface HttpResponse {
  get status(): number;
  get url(): string;

  text(): Promise<Result<string>>;
  text_data(): Promise<string>;
  json(): Promise<Result<any>>;
  json_data(): Promise<any>;

  close(): Promise<void>;
}

export interface HttpClient {
  get(request: HttpRequest): Promise<HttpResponse>;

  close(): Promise<void>;
  clone(): HttpClient;
}

export interface HttpRequest {
  url: string;
  header?: Record<string, string>;
}
