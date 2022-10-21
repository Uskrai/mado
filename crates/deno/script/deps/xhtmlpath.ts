import xpath from "xpath-ts";
import xmldom from "../js/xmldom";

// API wrapper for xpath and xmldom
export class XHTMLPath {
  doc: Node;
  isHtml: boolean;
  constructor(text: string, mimeType = "text/html") {
    this.doc = new xmldom.DOMParser({
      errorHandler: {
        // warning: (msg) => {console.log(msg)},
        // error: (msg) => {console.log(msg)},
        // fatalError: (msg) => {}
      },
    }).parseFromString(text, mimeType);

    this.isHtml = mimeType == "text/html";
  }

  static fromNode(node: Node, mimeType = "text/html") {
    let a = new XHTMLPath("", mimeType);
    a.doc = node;

    return a;
  }

  option() {
    return {
      node: this.doc,
      isHtml: this.isHtml,
    };
  }

  select(expression: string) {
    return xpath.parse(expression)?.select(this.option());
  }

  // evaluate(expression: string) {
  //   return xpath.parse(expression)?.evaluate(this.option());
  // }

  selectString(expression: string) {
    return xpath.parse(expression)?.evaluateString(this.option());
  }

  selectTextData(expression: string) {
    return this.select(expression).map((it: any) => {
      return it.data;
    });
  }

  selectText(expression: string, name: string) {
    return this.select(expression).map((it) => {
      return it[name];
    });
  }

  // selectTextHref(expression: string) {
  //   return this.select(expression).map((it: Element) => {
  //     return {
  //       href: it.getAttribute("href"),
  //       text: it.textContent,
  //     };
  //   });
  // }
}
