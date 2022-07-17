import { Errors } from "./error";

function fromPrefix(res: Resource, add: string) {
  return `${res.prefix}_${add}`;
}

export class Resource {
  strong_rid: Array<number>;
  constructor(public rid: number, public prefix: string) {
    this.strong_rid = [rid];
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

  get last_rid() {
    return this.strong_rid.at(-1);
  }

  increment_strong_count() {
    let rid = Deno.core.opSync(fromPrefix(this, "clone"), this.rid);
    this.strong_rid.push(rid);
    return rid;
  }

  decrement_strong_count() {
    Deno.core.opSync(fromPrefix(this, "close"), this.last_rid);

    if (this.strong_rid.length == 0) {
      this.rid = null;
    }
  }
}
