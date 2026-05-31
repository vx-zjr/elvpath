import {
  AlertTriangle,
  CheckCircle2,
  Download,
  Globe2,
  Languages,
  Play,
  RefreshCw,
  Search,
  Settings2,
  ShieldCheck,
  Terminal
} from "lucide-react";
import { useMemo, useState } from "react";

import catalogJson from "../src-tauri/catalog/catalog.json";
import { copy, getInitialLocale, text } from "./i18n";
import type {
  Catalog,
  CatalogItem,
  DetectionResult,
  Locale,
  NetworkMode,
  NetworkProfile,
  OperatingSystem,
  PlannedStep
} from "./types";

type ExecutionStatus = "dry-run" | "completed" | "failed" | "needs-privilege";

interface ExecutionResult {
  stepId: string;
  status: ExecutionStatus;
  message: string;
  logPath?: string;
}

interface ExecutionLogEvent {
  stepId: string;
  line: string;
}

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

const catalog = catalogJson as Catalog;
const defaultPlatform: OperatingSystem = detectPlatform();
type StackTab = "frontend" | "backend";

const frontendItemIds = new Set(["nodejs", "vscode", "flutter", "android-studio"]);

export function App() {
  const [locale, setLocale] = useState<Locale>(getInitialLocale());
  const [activeStackTab, setActiveStackTab] = useState<StackTab>("backend");
  const [activeCategory, setActiveCategory] = useState(catalog.categories[0]?.id ?? "languages");
  const [query, setQuery] = useState("");
  const [selectedIds, setSelectedIds] = useState<string[]>([]);
  const [networkMode, setNetworkMode] = useState<NetworkMode>("official");
  const [proxyUrl, setProxyUrl] = useState("");
  const [detections, setDetections] = useState<Record<string, DetectionResult>>({});
  const [executionResults, setExecutionResults] = useState<ExecutionResult[]>([]);
  const [terminalLines, setTerminalLines] = useState<string[]>([]);
  const [isExecuting, setIsExecuting] = useState(false);
  const [statusLine, setStatusLine] = useState("");

  const t = copy[locale];
  const selectedItems = useMemo(
    () => selectedIds.map((id) => catalog.items.find((item) => item.id === id)).filter(Boolean) as CatalogItem[],
    [selectedIds]
  );
  const stackItems = useMemo(
    () => catalog.items.filter((item) => itemMatchesStack(item, activeStackTab)),
    [activeStackTab]
  );
  const filteredItems = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return stackItems.filter((item) => {
      const inCategory = item.category === activeCategory;
      const searchable = [
        item.id,
        item.name.en,
        item.name.zh,
        item.summary.en,
        item.summary.zh,
        ...item.tags
      ]
        .join(" ")
        .toLowerCase();
      return inCategory && (!normalizedQuery || searchable.includes(normalizedQuery));
    });
  }, [activeCategory, query, stackItems]);

  const networkProfile: NetworkProfile = {
    mode: networkMode,
    mirrorRegion: networkMode === "china-mirror" ? "cn" : undefined,
    proxyUrl: networkMode === "custom-proxy" && proxyUrl.trim() ? proxyUrl.trim() : undefined
  };
  const planSteps = useMemo(
    () => buildPreviewSteps(selectedItems, networkProfile, defaultPlatform),
    [networkProfile, selectedItems]
  );

  function addItem(itemId: string) {
    setSelectedIds((current) => (current.includes(itemId) ? current : [...current, itemId]));
  }

  function removeItem(itemId: string) {
    setSelectedIds((current) => current.filter((id) => id !== itemId));
    setExecutionResults([]);
  }

  async function scanEnvironment() {
    if (!window.__TAURI_INTERNALS__) {
      const mock = Object.fromEntries(
        catalog.items.slice(0, 8).map((item) => [
          item.id,
          {
            itemId: item.id,
            status: item.id === "git" ? "installed" : "unknown",
            version: item.id === "git" ? "2.x" : undefined,
            source: item.id === "git" ? "PATH" : undefined,
            satisfiesRequested: item.id === "git",
            conflicts: []
          } satisfies DetectionResult
        ])
      );
      setDetections(mock);
      setStatusLine(locale === "zh" ? "\u6d4f\u89c8\u5668\u9884\u89c8\u4f7f\u7528\u6a21\u62df\u68c0\u6d4b\u7ed3\u679c\u3002" : "Browser preview uses simulated scan results.");
      return;
    }

    const { invoke } = await import("@tauri-apps/api/core");
    const results = await invoke<DetectionResult[]>("scan_environment");
    setDetections(Object.fromEntries(results.map((result) => [result.itemId, result])));
    setStatusLine(locale === "zh" ? "\u68c0\u6d4b\u5b8c\u6210\u3002" : "Scan completed.");
  }

  async function runInstallPlan() {
    if (planSteps.length === 0) {
      setStatusLine(locale === "zh" ? "\u8bf7\u5148\u4ece\u76ee\u5f55\u4e2d\u6dfb\u52a0\u8981\u5b89\u88c5\u7684\u5de5\u5177\u3002" : "Add at least one tool before starting installation.");
      return;
    }

    setIsExecuting(true);
    setExecutionResults([]);
    setTerminalLines([
      `$ ${t.startInstall}`,
      `${t.network}: ${networkSummary(networkProfile, locale)}`
    ]);
    setStatusLine(t.runningInstall);

    if (!window.__TAURI_INTERNALS__) {
      const dryResults = planSteps.map((step) => ({
        stepId: step.id,
        status: "dry-run" as const,
        message: dryRunMessage(step, networkProfile)
      }));
      setExecutionResults(dryResults);
      setTerminalLines((current) => [
        ...current,
        ...planSteps.flatMap((step) => terminalPreviewLines(step, networkProfile, locale))
      ]);
      setStatusLine(t.browserPreviewCannotInstall);
      setIsExecuting(false);
      return;
    }

    let unlisten: (() => void) | undefined;

    try {
      const { invoke } = await import("@tauri-apps/api/core");
      const { listen } = await import("@tauri-apps/api/event");
      unlisten = await listen<ExecutionLogEvent>("elvpath-install-log", (event) => {
        setTerminalLines((current) => [...current, event.payload.line]);
      });
      const results: ExecutionResult[] = [];

      for (const step of planSteps) {
        setStatusLine(`${t.runningInstall}: ${text(step.title, locale)}`);
        setTerminalLines((current) => [
          ...current,
          "",
          `> ${text(step.title, locale)}`,
          step.url ? `url: ${step.url}` : "url: n/a",
          step.command ? `command: ${step.command} ${step.args.join(" ")}` : "command: generated installer script"
        ]);
        const result = await invoke<ExecutionResult>("execute_install_step", {
          request: {
            step,
            dryRun: false,
            networkProfile
          }
        });
        results.push(result);
        setExecutionResults([...results]);
        setTerminalLines((current) => [
          ...current,
          `[${statusLabel(result.status, locale)}] ${result.stepId}`,
          ...(result.status === "failed" || result.status === "needs-privilege" ? [result.message] : [])
        ]);

        if (result.status === "failed" || result.status === "needs-privilege") {
          setStatusLine(result.status === "needs-privilege" ? t.installBlockedPrivilege : t.installFailed);
          setIsExecuting(false);
          return;
        }
      }

      setStatusLine(t.installFinished);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setExecutionResults((current) => [
        ...current,
        {
          stepId: "client",
          status: "failed",
          message
        }
      ]);
      setTerminalLines((current) => [...current, `[${statusLabel("failed", locale)}] client`, message]);
      setStatusLine(t.installFailed);
    } finally {
      unlisten?.();
      setIsExecuting(false);
    }
  }

  return (
    <main className="app-shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">E</div>
          <div>
            <h1>ElvPath</h1>
            <p>{locale === "zh" ? "\u5f00\u53d1\u73af\u5883\u90e8\u7f72\u5668" : "Dev environment deployer"}</p>
          </div>
        </div>

        <nav className="category-nav" aria-label="Catalog categories">
          {catalog.categories.map((category) => (
            <button
              className={category.id === activeCategory ? "category active" : "category"}
              key={category.id}
              onClick={() => setActiveCategory(category.id)}
              type="button"
            >
              <span>{text(category.name, locale)}</span>
              <small>{stackItems.filter((item) => item.category === category.id).length}</small>
            </button>
          ))}
        </nav>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div className="topbar-primary">
            <label className="search-box">
              <Search aria-hidden="true" size={18} />
              <span className="sr-only">{t.searchCatalog}</span>
              <input
                aria-label={t.searchCatalog}
                onChange={(event) => setQuery(event.target.value)}
                placeholder={t.searchCatalog}
                value={query}
              />
            </label>
            <div className="stack-tabs" role="tablist" aria-label={t.stackTabs}>
              <button
                aria-selected={activeStackTab === "frontend"}
                className={activeStackTab === "frontend" ? "active" : ""}
                onClick={() => setActiveStackTab("frontend")}
                role="tab"
                type="button"
              >
                {t.frontend}
              </button>
              <button
                aria-selected={activeStackTab === "backend"}
                className={activeStackTab === "backend" ? "active" : ""}
                onClick={() => setActiveStackTab("backend")}
                role="tab"
                type="button"
              >
                {t.backend}
              </button>
            </div>
          </div>
          <div className="toolbar">
            <button className="icon-button" onClick={scanEnvironment} title={t.scan} type="button">
              <RefreshCw aria-hidden="true" size={17} />
              <span>{t.scan}</span>
            </button>
            <button
              className="icon-button"
              onClick={() => setLocale((current) => (current === "en" ? "zh" : "en"))}
              title="Language"
              type="button"
            >
              <Languages aria-hidden="true" size={17} />
              <span>{locale === "en" ? "\u4e2d\u6587" : "English"}</span>
            </button>
          </div>
        </header>

        <div className="content-grid">
          <section className="catalog-panel">
            <div className="section-heading">
              <div>
                <h2>{text(catalog.categories.find((category) => category.id === activeCategory)?.name, locale)}</h2>
                <p>{text(catalog.categories.find((category) => category.id === activeCategory)?.description, locale)}</p>
              </div>
              <span className="platform-pill">{platformLabel(defaultPlatform)}</span>
            </div>

            <div className="tool-list">
              {filteredItems.map((item) => {
                const detected = detections[item.id];
                const selected = selectedIds.includes(item.id);
                return (
                  <article className="tool-row" key={item.id}>
                    <div className="tool-main">
                      <div className="tool-title-line">
                        <h3>{text(item.name, locale)}</h3>
                        {detected ? (
                          <span className={`status ${detected.status}`}>
                            {detected.status === "installed" ? <CheckCircle2 size={15} /> : <AlertTriangle size={15} />}
                            {detected.status}
                          </span>
                        ) : null}
                      </div>
                      <p>{text(item.summary, locale)}</p>
                      <div className="tag-strip">
                        {item.tags.slice(0, 4).map((tag) => (
                          <span key={tag}>{tag}</span>
                        ))}
                      </div>
                    </div>
                    <div className="tool-actions">
                      <select aria-label={`${text(item.name, locale)} ${t.version}`} defaultValue={defaultVersion(item)}>
                        {item.versions.map((version) => (
                          <option key={version.id} value={version.id}>
                            {version.label}
                          </option>
                        ))}
                      </select>
                      <button
                        className={selected ? "secondary-button" : "primary-button"}
                        onClick={() => (selected ? removeItem(item.id) : addItem(item.id))}
                        type="button"
                      >
                        <Download aria-hidden="true" size={16} />
                        {selected ? t.selected : `${t.add} ${text(item.name, locale)}`}
                      </button>
                    </div>
                  </article>
                );
              })}
            </div>
          </section>

          <aside className="plan-panel">
            <section className="network-panel" aria-label={t.network}>
              <div className="mini-heading">
                <Globe2 aria-hidden="true" size={17} />
                <span>{t.network}</span>
              </div>
              <div className="segmented">
                <button className={networkMode === "official" ? "active" : ""} onClick={() => setNetworkMode("official")} type="button">
                  {t.official}
                </button>
                <button className={networkMode === "china-mirror" ? "active" : ""} onClick={() => setNetworkMode("china-mirror")} type="button">
                  {t.mirror}
                </button>
                <button className={networkMode === "custom-proxy" ? "active" : ""} onClick={() => setNetworkMode("custom-proxy")} type="button">
                  {t.proxy}
                </button>
              </div>
              {networkMode === "custom-proxy" ? (
                <label className="proxy-field">
                  <span>{t.proxyUrl}</span>
                  <input
                    aria-label={t.proxyUrl}
                    onChange={(event) => setProxyUrl(event.target.value)}
                    placeholder="http://127.0.0.1:7890"
                    value={proxyUrl}
                  />
                </label>
              ) : null}
            </section>

            <section className="install-plan">
              <div className="plan-heading">
                <div>
                  <h2>{t.installPlan}</h2>
                  <p>
                    {selectedItems.length} {t.selected}
                  </p>
                </div>
                <button
                  aria-busy={isExecuting}
                  aria-label={t.startInstall}
                  className={isExecuting ? "icon-only running" : "icon-only"}
                  onClick={runInstallPlan}
                  title={t.startInstall}
                  type="button"
                >
                  <Play aria-hidden="true" size={17} />
                </button>
              </div>

              <div className="plan-steps">
                {planSteps.length === 0 ? (
                  <div className="empty-plan">
                    <Settings2 aria-hidden="true" size={24} />
                    <span>{locale === "zh" ? "\u4ece\u76ee\u5f55\u4e2d\u6dfb\u52a0\u5de5\u5177\u3002" : "Add tools from the catalog."}</span>
                  </div>
                ) : (
                  planSteps.map((step) => (
                    <article className="plan-step" key={step.id}>
                      <div className="step-title">
                        <Terminal aria-hidden="true" size={16} />
                        <strong>{text(step.title, locale)}</strong>
                      </div>
                      <dl>
                        <div>
                          <dt>{t.source}</dt>
                          <dd>{step.source === "official" ? t.officialSource : step.source}</dd>
                        </div>
                        <div>
                          <dt>URL</dt>
                          <dd>{step.url ?? "n/a"}</dd>
                        </div>
                        <div>
                          <dt>{t.checksum}</dt>
                          <dd>{step.checksum ?? "n/a"}</dd>
                        </div>
                        <div>
                          <dt>{t.requiresAdmin}</dt>
                          <dd>{step.permissions.requiresAdmin ? "Yes" : "No"}</dd>
                        </div>
                      </dl>
                      {step.configChanges.length > 0 ? (
                        <div className="config-list">
                          <ShieldCheck aria-hidden="true" size={15} />
                          <span>
                            {t.configChanges}: {step.configChanges.map((change) => change.target).join(", ")}
                          </span>
                        </div>
                      ) : null}
                    </article>
                  ))
                )}
              </div>

              {executionResults.length > 0 ? (
                <section className="execution-log" aria-label="Execution log" aria-live="polite">
                  {executionResults.map((result) => (
                    <article className={`execution-result ${result.status}`} key={result.stepId}>
                      <div>
                        <strong>{result.stepId}</strong>
                        <span>{statusLabel(result.status, locale)}</span>
                      </div>
                      <p>{result.message}</p>
                    </article>
                  ))}
                </section>
              ) : null}
            </section>

            <section className="terminal-panel" aria-label={t.installTerminal} aria-live="polite">
              <div className="mini-heading">
                <Terminal aria-hidden="true" size={17} />
                <span>{t.installTerminal}</span>
              </div>
              <pre>{terminalLines.length > 0 ? terminalLines.join("\n") : t.terminalReady}</pre>
            </section>

            {statusLine ? (
              <p className="status-line" role="status">
                {statusLine}
              </p>
            ) : null}
          </aside>
        </div>
      </section>
    </main>
  );
}

function buildPreviewSteps(
  selectedItems: CatalogItem[],
  networkProfile: NetworkProfile,
  platform: OperatingSystem
): PlannedStep[] {
  return selectedItems.flatMap((item) =>
    item.installSteps
      .filter((step) => step.platforms.includes(platform) || step.platforms.length === 0)
      .map((step) => ({
        id: `${item.id}:${step.id}`,
        itemId: item.id,
        title: step.title,
        kind: step.kind,
        source: step.source,
        url: networkProfile.mode === "china-mirror" ? step.mirrorUrl ?? step.url : step.url,
        checksum: step.checksum,
        command: step.command,
        args: step.args,
        permissions: step.permissions,
        configChanges: item.configSteps
      }))
  );
}

function defaultVersion(item: CatalogItem): string {
  return item.versions.find((version) => version.default)?.id ?? item.versions[0]?.id ?? "latest";
}

function itemMatchesStack(item: CatalogItem, stackTab: StackTab): boolean {
  const isFrontend = frontendItemIds.has(item.id);
  return stackTab === "frontend" ? isFrontend : !isFrontend;
}

function terminalPreviewLines(
  step: PlannedStep,
  networkProfile: NetworkProfile,
  locale: Locale
): string[] {
  return [
    "",
    `> ${text(step.title, locale)}`,
    step.url ? `url: ${step.url}` : "url: n/a",
    step.command ? `command: ${step.command} ${step.args.join(" ")}` : "command: generated installer script",
    dryRunMessage(step, networkProfile)
  ];
}

function dryRunMessage(step: PlannedStep, networkProfile?: NetworkProfile): string {
  const proxy = networkProfile?.proxyUrl ? ` via proxy ${networkProfile.proxyUrl}` : "";
  if (step.command && step.url) {
    return `Dry run: download ${step.url}${proxy}, then run \`${step.command} ${step.args.join(" ")}\``;
  }
  if (step.command) {
    return `Dry run: run \`${step.command} ${step.args.join(" ")}\``;
  }
  if (step.url) {
    return `Dry run: download ${step.url}${proxy}`;
  }
  return "Dry run: configuration-only step";
}

function networkSummary(networkProfile: NetworkProfile, locale: Locale): string {
  if (networkProfile.mode === "custom-proxy") {
    const proxy = networkProfile.proxyUrl || (locale === "zh" ? "\u672a\u586b\u5199" : "not set");
    return `${locale === "zh" ? "\u4ee3\u7406" : "proxy"} ${proxy}`;
  }
  if (networkProfile.mode === "china-mirror") {
    return locale === "zh" ? "\u56fd\u5185\u955c\u50cf" : "China mirrors";
  }
  return locale === "zh" ? "\u5b98\u65b9\u6e90" : "official";
}

function statusLabel(status: ExecutionStatus, locale: Locale): string {
  const labels: Record<ExecutionStatus, { en: string; zh: string }> = {
    "dry-run": { en: "Dry run", zh: "\u5e72\u8dd1" },
    completed: { en: "Completed", zh: "\u5df2\u5b8c\u6210" },
    failed: { en: "Failed", zh: "\u5931\u8d25" },
    "needs-privilege": { en: "Needs admin", zh: "\u9700\u8981\u7ba1\u7406\u5458\u6743\u9650" }
  };
  return labels[status][locale];
}

function detectPlatform(): OperatingSystem {
  const platform = navigator.platform.toLowerCase();
  if (platform.includes("mac")) return "macos";
  if (platform.includes("win")) return "windows";
  return "linux";
}

function platformLabel(platform: OperatingSystem): string {
  if (platform === "macos") return "macOS";
  if (platform === "windows") return "Windows";
  return "Linux";
}
