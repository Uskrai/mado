export abstract class ResultBase<O> {
  content: O | Errors;

  abstract isError(): this is ResultError<O>;

  abstract isOk(): this is ResultOk<O>;

  abstract throw(): O;
  abstract throwDebug(): O;

  or(val: O): O {
    return this.orElse(() => val);
  }

  orElse(val: () => O): O {
    return this.isOk() ? this.data : val();
  }

  map<T>(val: (arg: O) => T): ResultBase<T> {
    if (this.isOk()) {
      return Ok(val(this.data));
    } else {
      return Err(this.content as Errors);
    }
  }

  okOrNull(): this | null {
    if (this.isOk()) {
      return this;
    }
    return null;
  }

  get data(): O {
    return this.throw();
  }

  get error(): Errors {
    if (this.isError()) {
      return this.content;
    } else {
      throw Errors.message("This is not error");
    }
  }
}

export type Result<T> = ResultBase<T>;

export const Ok = <T>(data: T): Result<T> => {
  return new ResultOk(data);
};

export const Err = <T>(error: Errors): Result<T> => {
  return new ResultError<T>(error);
};

export class ResultError<T> extends ResultBase<T> {
  type = "Err";
  constructor(public content: Errors) {
    super();
  }

  isError() {
    return true;
  }

  isOk() {
    return false;
  }
  throw(): T {
    throw this;
  }

  throwDebug(): T {
    let stack = this.content.stack;
    let deb = this.content.intoDebug();
    this.content.close();
    throw `${deb}\n${stack}`;
  }
}

export class ResultOk<T> extends ResultBase<T> {
  type = "Ok";
  constructor(public content: T) {
    super();
  }
  isError() {
    return false;
  }
  isOk() {
    return true;
  }
  throw(): T {
    return this.content;
  }
  throwDebug(): T {
    return this.content;
  }
}

export async function catchAndReturn<T>(
  action: () => Promise<T> | PromiseLike<T>
): Promise<Result<T>> {
  return Promise.resolve(action)
    .then((it) => Promise.resolve(it()))
    .then((it) => Ok(it))
    .catch((it) => {
      if (it instanceof Errors) {
        return Err(it);
      } else if (it instanceof ResultError) {
        return it;
      } else {
        return Err(Errors.fromCatch(it));
      }
    });
}

let opSync = Deno.core.opSync;

function op(name: string, ...param: any[]) {
  let error = opSync(name, ...param);

  return new Errors(error.type, error.content);
}

interface ResultJson {
  type: "Ok" | "Err";
}

export function ResultFromJson(json: any): Result<any> {
  if (json.type == "Ok") {
    return Ok(json.content);
  } else if (json.type == "Err") {
    return Err(new Errors(json.content.type, json.content.content));
  }
}

export class Errors extends Error {
  constructor(public type: string, public content: any) {
    super(opSync("op_error_to_string", { type, content }));
  }

  static message(it: string) {
    return Errors.fromCatch(new Error(it));
  }

  static fromCatch(it: Error) {
    return new Errors("Custom", {
      message: it.stack || it.toString(),
    });
  }

  intoString() {
    return opSync("op_error_to_string", this);
  }

  intoDebug() {
    return opSync("op_error_to_debug", this);
  }

  close(): Result<any> {
    return ResultFromJson(opSync("op_error_close", this));
  }

  static request_error(url: string, message: string) {
    return op("op_error_request_error", url, message);
  }
  static unexpected_error(url: string, message: string) {
    return op("op_error_unexpected_error", url, message);
  }
  static invalid_url(url: string) {
    return op("op_error_invalid_url", url);
  }
}
