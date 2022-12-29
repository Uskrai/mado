import { Result } from "./error";

export interface BrowserConfig {
  userAgent?: string;
}

export interface Browser {
  newTab(): Promise<Result<Tab>>;

  setConfig(config: BrowserConfig): Promise<void>;
}

export interface TabRequest {
  url: string;
  user_agent?: string;
}

export interface Tab {
  content(): Promise<Result<string>>;
  url(): Promise<Result<string>>;

  navigateTo(request: TabRequest): Promise<Result<null>>;
  waitForNavigation(): Promise<Result<null>>;
  waitForElement(string: string): Promise<Result<null>>;
  waitForElementByXPath(string: string): Promise<Result<null>>;
  click(selector: string): Promise<Result<void>>;

  evaluate(script: string): Promise<any>;

  reload(): Promise<any>;
  reloadForce(): Promise<any>;

  close(): Promise<any>;
}

export interface TabElement {
  click(): Promise<Result<void>>;
  node(): Promise<any>;
}

