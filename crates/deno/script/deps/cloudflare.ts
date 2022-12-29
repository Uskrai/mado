import { Browser, Tab } from "./browser";
import { Result } from "./error";

function isIUAM(content: string) {
  return content.match("\\/cdn-cgi\\/images\\/trace\\/managed\\/");
}

class Shared {
  public tab?: Tab;
}

export class Cloudflare {
  public tab?: Tab;
  constructor(public browser: Browser) {
  }

  async text(url: string): Promise<Result<string>> {
    return await this.get(url).get();
  }

  get(url: string): CloudflareRequestBuilder {
    return new CloudflareRequestBuilder(this, url);
  }

  async close() {
    return await this.tab?.close();
  }
}

export class CloudflareRequestBuilder {
  get tab() {
    return this.cf.tab!;
  }
  set tab(tab: Tab) {
    this.cf.tab = tab;
  }

  public waitForXPath?: string;
  public waitForElementExpression?: string;

  public constructor(public cf: Cloudflare, public url: string) {}

  public waitForElementByXPath(expression: string) {
    this.waitForXPath = expression;
    return this;
  }

  public waitForElement(expression: string) {
    this.waitForElementExpression = expression;
    return this;
  }

  public async load() {
    if (this.tab == null) {
      let tab = await this.cf.browser.newTab();

      if (tab.isError()) {
        return tab.map((_) => "");
      }

      this.tab = tab.data;
    }

    let tab = this.tab!;

    console.log(this.url);
    let result = await tab.navigateTo({ url: this.url });

    await tab.waitForNavigation();
    if (result.isError()) {
      return result.map((_) => "");
    }

    return await tab.content();
  }

  public async get(): Promise<Result<string>> {
    let result = await this.load();

    while (true) {
      if (result.isError()) {
        return result;
      }

      let data = result.data;

      let shouldreload = data.length == 0 || isIUAM(data);
      console.log(data.length, isIUAM(data), shouldreload);

      if (!shouldreload) {
        return result;
      }

      if (this.waitForXPath != null) {
        await this.tab!.waitForElementByXPath(this.waitForXPath);
      }

      if (this.waitForElementExpression != null) {
        await this.tab!.waitForElementByXPath(this.waitForElementExpression)
      }

      result = await this.tab!.content();

      // if (result.isOk()) {
      //   console.log(result.data)
      // }
      // console.log(result);
    }
  }
}
