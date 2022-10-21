import { Result, ResultFromJson } from "./error";

function fromPrefix(res: Resource, add: string) {
  return `${res.prefix}_${add}`;
}

export class Resource {
  constructor(public rid: number, public prefix: string) {
  }

  // get rid() {
  //     let rid = this.strong_rid.at(0);
  //
  //     if (rid == null) {
  //         throw "Resource is empty";
  //     }
  //
  //     return rid;
  // }

  increment_strong_count() {
    let rid = ResultFromJson(Deno.core.opSync(fromPrefix(this, "clone"), this.rid)).data;
    return rid;
  }

  decrement_strong_count(): Result<void>  {
    return ResultFromJson(Deno.core.opSync(fromPrefix(this, "close"), this.rid));
  }
}
