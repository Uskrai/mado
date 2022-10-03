import { HttpClient } from "../deps";
import { Ok } from "../deps/error";

export function http__Ok() {
  let http = new HttpClient();
  http.close();

  return Ok({});
}

export function http__Ok__Clone() {
  let http = new HttpClient();
  let http2 = http.clone();

  http.close().data;
  http2.close().data;

  return Ok({});
}

export async function http__Err_ResourceError__Decrement() {
  let http = new HttpClient();
  http.close();
  return http.close();
}

export async function http__Ok__Response() {
  let http = new HttpClient();
  let url = "https://google.com/";
  let response = await http.get({
    url
  });

  console.assert(response.url == url);
  console.assert(response.status == 301);

  return Ok({});
}
