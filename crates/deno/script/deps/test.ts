import deepequal from "deep-equal";
import inspect from "object-inspect";
import { Errors } from "./error";

function message(it: string) {
  return Errors.fromCatch(new Error(it));
}

export function assertTrue(truth: boolean) {
  return assertEq(truth, true);
}

export function assertEq<T>(actual: T, expected: T) {
  if (!deepequal(actual, expected)) {
    throw message(`expected: ${inspect(expected)}, actual: ${inspect(actual)}`);
  }
}
