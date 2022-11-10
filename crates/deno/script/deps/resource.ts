import { Result, ResultFromJson } from "./error";

function fromPrefix(res: Resource, add: string) {
  return `${res.prefix}_${add}`;
}

function opFromPrefix(res: Resource, add: string) {
  return Deno.core.ops[fromPrefix(res, add)];
}

export class Resource {
  constructor(public rid: number, public prefix: string) {}

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
    let rid = ResultFromJson(opFromPrefix(this, "clone")(this.rid)).data;
    return rid;
  }

  decrement_strong_count(): Result<void> {
    return ResultFromJson(opFromPrefix(this, "close")(this.rid));
  }
}
