import deepequal from "deep-equal";
import inspect from "object-inspect";
import { Errors, Ok, Result } from "./error";


export function assertTrue(truth: boolean) {
  return assertEq(truth, true);
}

export function assertEq<T>(actual: T, expected: T) {
  if (!deepequal(actual, expected)) {
    throw Errors.message(`expected: ${inspect(expected)}, found: ${inspect(actual)}`);
  }
}

export function assertOk<T>(actual: Result<T>) {
  if (actual.isError()) {
    throw Errors.message(`expected: Ok, found: ${inspect(actual)}`);
  }
}
