import { Browser, BrowserConfig, Tab, TabElement, TabRequest } from "./browser";
import { Ok, Result, ResultFromJson } from "./error";

export class RustBrowser implements Browser {
  constructor(public config: BrowserConfig = {}) {}
  async newTab(): Promise<Result<Tab>> {
    let tab = ResultFromJson(await Deno.core.ops.op_mado_chromium_new_tab());

    if (tab.isError()) {
      return tab;
    }

    return Ok(new RustTab(tab.content));
  }

  async setConfig(config: BrowserConfig): Promise<void> {
    this.config = config;
  }
}

export class RustTab implements Tab {
  constructor(public rid: number) {
    //
  }

  async navigateTo(request: TabRequest): Promise<Result<null>> {
    let result = ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_goto(this.rid, request.url)
    );

    if (result.isError()) {
      return result;
    }

    return result;
  }

  async waitForNavigation(): Promise<Result<null>> {
    return ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_wait_for_navigation(this.rid)
    );
  }

  async content(): Promise<Result<null>> {
    return ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_content(this.rid)
    );
  }

  async waitForElement(selector: string): Promise<Result<null>> {
    let element = ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_wait_for_element(
        this.rid,
        selector
      )
    );

    return this.toTabElement(element);
  }

  async waitForElementByXPath(string: string): Promise<Result<null>> {
    let element = ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_wait_for_element_by_xpath(
        this.rid,
        string
      )
    );

    return this.toTabElement(element);
  }

  toTabElement(element: Result<any>) {
    if (element.isError()) {
      return element;
    }

    return Ok(null);
    // return Ok(new RustTabElement(element.data));

  }

  async click(selector: string): Promise<Result<void>> {
    return ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_click(this.rid, selector)
    );
  }

  async url(): Promise<Result<string>> {
    return Ok(await Deno.core.ops.op_mado_chromium_tab_url(this.rid));
  }

  async evaluate(script: string): Promise<any> {
    return ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_evaluate(this.rid, script)
    );
  }

  async reload(): Promise<any> {
    return ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_reload(this.rid)
    );
  }

  async reloadForce(): Promise<any> {
    return ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_reload_force(this.rid)
    );
  }

  async close(): Promise<Result<void>> {
    console.log("closing");
    return ResultFromJson(
      await Deno.core.ops.op_mado_chromium_tab_close(this.rid)
    );
  }
}

export class RustTabElement implements TabElement {
  constructor(public rid: number) {}

  async click(): Promise<Result<void>> {
    return ResultFromJson(
      await Deno.core.ops.op_mado_chromium_element_click(this.rid)
    );
  }

  async node(): Promise<any> {
    return ResultFromJson(
      await Deno.core.ops.op_mado_chromium_element_node(this.rid)
    );
  }
}
