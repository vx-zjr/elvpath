import type { Locale, LocalizedText } from "./types";

export const copy = {
  en: {
    add: "Add",
    allPlatforms: "All platforms",
    browserPreviewCannotInstall: "Browser preview cannot install; dry run completed.",
    buildPlan: "Build plan",
    backend: "Backend",
    checksum: "Checksum",
    configChanges: "Config changes",
    detected: "Detected",
    dryRun: "Dry run",
    frontend: "Frontend",
    installBlockedPrivilege: "Installation paused: administrator permission is required.",
    installFailed: "Installation stopped after a failed step.",
    installFinished: "Installation workflow finished.",
    installPlan: "Install Plan",
    installTerminal: "Install terminal",
    mirror: "China mirrors",
    network: "Network",
    official: "Official",
    officialSource: "Official source",
    proxy: "Proxy",
    proxyUrl: "HTTP(S) proxy",
    refresh: "Refresh",
    requiresAdmin: "Requires admin",
    runningInstall: "Running installation plan",
    scan: "Scan",
    searchCatalog: "Search catalog",
    selected: "Selected",
    settings: "Settings",
    source: "Source",
    stackTabs: "Development side",
    startInstall: "Start installation",
    terminalReady: "Terminal output will appear here.",
    version: "Version"
  },
  zh: {
    add: "\u6dfb\u52a0",
    allPlatforms: "\u5168\u90e8\u5e73\u53f0",
    browserPreviewCannotInstall: "\u6d4f\u89c8\u5668\u9884\u89c8\u4e0d\u80fd\u771f\u5b9e\u5b89\u88c5\uff1b\u5df2\u5b8c\u6210\u5e72\u8dd1\u3002",
    buildPlan: "\u751f\u6210\u8ba1\u5212",
    backend: "\u540e\u7aef",
    checksum: "\u6821\u9a8c",
    configChanges: "\u914d\u7f6e\u53d8\u66f4",
    detected: "\u68c0\u6d4b\u7ed3\u679c",
    dryRun: "\u5e72\u8dd1",
    frontend: "\u524d\u7aef",
    installBlockedPrivilege: "\u5b89\u88c5\u5df2\u6682\u505c\uff1a\u9700\u8981\u7ba1\u7406\u5458\u6743\u9650\u3002",
    installFailed: "\u5b89\u88c5\u5728\u5931\u8d25\u6b65\u9aa4\u540e\u505c\u6b62\u3002",
    installFinished: "\u5b89\u88c5\u6d41\u7a0b\u5df2\u5b8c\u6210\u3002",
    installPlan: "\u5b89\u88c5\u8ba1\u5212",
    installTerminal: "\u5b89\u88c5\u7ec8\u7aef",
    mirror: "\u56fd\u5185\u955c\u50cf",
    network: "\u7f51\u7edc",
    official: "\u5b98\u65b9",
    officialSource: "\u5b98\u65b9\u6765\u6e90",
    proxy: "\u4ee3\u7406",
    proxyUrl: "HTTP(S) \u4ee3\u7406",
    refresh: "\u5237\u65b0",
    requiresAdmin: "\u9700\u8981\u7ba1\u7406\u5458\u6743\u9650",
    runningInstall: "\u6b63\u5728\u6267\u884c\u5b89\u88c5\u8ba1\u5212",
    scan: "\u68c0\u6d4b",
    searchCatalog: "\u641c\u7d22\u76ee\u5f55",
    selected: "\u5df2\u9009\u62e9",
    settings: "\u8bbe\u7f6e",
    source: "\u6765\u6e90",
    stackTabs: "\u5f00\u53d1\u65b9\u5411",
    startInstall: "\u5f00\u59cb\u5b89\u88c5",
    terminalReady: "\u5b89\u88c5\u8f93\u51fa\u4f1a\u663e\u793a\u5728\u8fd9\u91cc\u3002",
    version: "\u7248\u672c"
  }
} as const;

export function text(value: LocalizedText | undefined, locale: Locale): string {
  if (!value) return "";
  return value[locale] || value.en;
}

export function getInitialLocale(): Locale {
  if (typeof navigator === "undefined") return "en";
  return navigator.language.toLowerCase().startsWith("zh") ? "zh" : "en";
}
