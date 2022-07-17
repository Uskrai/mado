export abstract class ResultBase<O, E> {
  content: O | E;

  isError(): boolean {
    return !this.isOk();
  }
  abstract isOk(): boolean;

  abstract throw(): O;
  abstract throwDebug(): O;

  okOrNull(): this | null {
    if (this.isOk()) {
      return this;
    }
    return null;
  }

  get data(): O {
    return this.throw();
  }
}

export type Result<T, E> = ResultBase<T, E> | ResultBase<T, E>;

const Ok = <T>(data: T): Result<T, never> => {
  return new ResultOk(data);
};

const Err = (error: Errors): Result<never, Errors> => {
  return new ResultError<never>(error);
};

export class ResultError<T> extends ResultBase<T, Errors> {
  type = "Err";
  constructor(public content: Errors) {
    super();
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

export class ResultOk<T, E> extends ResultBase<T, E> {
  type = "Ok";
  constructor(public content: T) {
    super();
  }
  isOk(): boolean {
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
  action: () => Promise<T>
): Promise<Result<T, Errors>> {
  return await action()
    .then((it) => Ok(it))
    .catch((it) => {
      if (it instanceof Errors) {
        return Err(it);
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

export function ResultFromJson(json: any): Result<any, Errors> {
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

  close() {
    return opSync("op_error_close", this);
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
