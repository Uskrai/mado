import { Browser, Tab } from "./browser";
import { Errors, Ok, Result } from "./error";

import reaperscans_perfect_surgeon from "../resource/reaperscans/perfect-surgeon-page-2.html";
import reaperscans_exlife_hunter from "../resource/reaperscans/exlife_hunter_page_3.html";
import reaperscans_academys_undercover_professor from "../resource/reaperscans/academys_undercover_professor_page_2.html";
// import reaperscans_whats_wrong_with_being_the_villainess_page_1 from "../resource/reaperscans/whats_wrong_with_being_the_villainess_page_1.html";
import reaperscans_whats_wrong_with_being_the_villainess_page_2 from "../resource/reaperscans/whats_wrong_with_being_the_villainess_page_2.html";
import reaperscans_past_life_returner_page_2 from "../resource/reaperscans/past_life_regressor_page_2.html";
import reaperscans_novel_extra_page_2 from "../resource/reaperscans/novel_extra_page_2.html";
import { sleep } from ".";
// import reaperscans_whats_wrong_with_being_the_villainess_chapter_34 from "../resource/reaperscans/whats_wrong_with_being_the_villainess_chapter_34.html";


function isIUAM(content: string) {
  return content.match("\\/cdn-cgi\\/images\\/trace\\/managed\\/");
}

const cache = {
  "https://reaperscans.com/comics/2995-perfect-surgeon?page=2": reaperscans_perfect_surgeon,
  "https://reaperscans.com/comics/7833-990k-ex-life-hunter?page=3": reaperscans_exlife_hunter,
  "https://reaperscans.com/comics/1770-academys-undercover-professor?page=2": reaperscans_academys_undercover_professor,
  // "https://reaperscans.com/comics/3407-whats-wrong-with-being-the-villainess/chapters/80420050-chapter-34": reaperscans_whats_wrong_with_being_the_villainess_chapter_34,
  // "https://reaperscans.com/comics/3407-whats-wrong-with-being-the-villainess": reaperscans_whats_wrong_with_being_the_villainess_page_1,
  "https://reaperscans.com/comics/3407-whats-wrong-with-being-the-villainess?page=2": reaperscans_whats_wrong_with_being_the_villainess_page_2,
  "https://reaperscans.com/comics/4450-past-life-returner?page=2": reaperscans_past_life_returner_page_2,
  "https://reaperscans.com/comics/8556-the-novels-extra?page=2": reaperscans_novel_extra_page_2,
};

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
    if (cache[this.url] != null) {
      return Ok(cache[this.url]);
    }

    if (this.tab == null) {
      console.log([ "new_tab" ]);
      let tab = await this.cf.browser.newTab();
      console.log([ "after", tab ]);

      if (tab.isError()) {
        return tab.map((_) => "");
      }

      this.tab = tab.data;
    }

    let tab = this.tab!;

    console.log(`navigate to ${this.url}`)
    let result = await tab.navigateTo({ url: this.url });

    await tab.waitForNavigation();
    if (result.isError()) {
      return result.map((_) => "");
    }

    await this.wait();
    return await tab.content();
  }

  async wait() {
      if (this.waitForXPath != null) {
        await this.tab!.waitForElementByXPath(this.waitForXPath);

      }

      if (this.waitForElementExpression != null) {
        await this.tab!.waitForElementByXPath(this.waitForElementExpression)

      }
  }

  public async get(): Promise<Result<string>> {
    let result = await this.load();

    let count = 0;
    let retry_count = 0;
    while (true) {
      let shouldreload = false;
      if (result.isError()) {
        return result;
      }

      let data = result.data;

      if (data.length == 0) {
        shouldreload = true;
      }

      if (isIUAM(data)) {
        console.log("data iuam", data);

        await sleep(10000);
        shouldreload = true;
      }
      // let shouldreload = data.length == 0 || isIUAM(data);

      if (!shouldreload) {
        await sleep(2000);

        return this.tab!.content();
      }

      await this.wait();

      if (count == 10) {
        // if (retry_count == 10) {
        //   throw Errors.message("Retry count reached");
        // }
        result = await this.load();
        count = 0;
        retry_count += 1;
        continue;
      }

      count += 1;

      result = await this.tab!.content();

    }
  }
}
