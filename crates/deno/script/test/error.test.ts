import { catchAndReturn, Err, Errors, Ok, ResultBase } from "../deps/error";
import { assertEq, assertTrue } from "../deps/test";

export async function error__Ok() {
  return Ok({});
}

export async function error__Err_InvalidUrl_Test() {
  let error = Err(Errors.invalid_url("https://google.com"));
  assertTrue(error.isError());
  return Err(Errors.invalid_url("https://google.com"));
}

export async function error__Err_ExternalError() {
  return await catchAndReturn(() => { throw "Error Error Error" });
}

export function error__Err_ResourceError__Close() {
  let error = Errors.invalid_url("https://google.com");
  error.close();

  let it = error.close();
  return it;
}

export function error__Ok__Or() {
  assertEq((Err({} as any) as ResultBase<number>).or(0), 0);
  assertEq((Err({} as any) as ResultBase<number>).orElse(() => 0), 0);
  assertEq((Ok(1)).or(2), 1);
  assertEq(Ok(2).orElse(() => 0), 2);
  assertEq((Err({} as any).okOrNull()), null);
  assertEq((Ok({}).okOrNull()), Ok({}));

  return Ok({});
}


export async function error__Ok__Throw() {
  try {
    Err({} as any).throw()
  } catch (e) {
    assertEq(e, Err({} as any));
  }

  return Ok({});
}

export async function error__Ok__ToString() {
  let error = Errors.invalid_url("https://google.com");

  assertEq(error.intoString(), "https://google.com is invalid");
  assertEq(error.intoDebug(), "InvalidUrl { url: \"https://google.com\" }");

  return Ok({});
}

export async function error__Ok__CloseCustom() {
  let error = Errors.message("Custom Error");

  return error.close();
}
