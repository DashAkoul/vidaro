import { useMemo } from "react";

import ClipboardToast from "./components/ClipboardToast";
import ErrorBoundary from "./components/ErrorBoundary";
import Sidebar from "./components/Sidebar";
import StatusBar from "./components/StatusBar";
import ToolsOverlay from "./components/ToolsOverlay";
import Topbar from "./components/Topbar";
import { AppProvider, useApp } from "./lib/app-context";
import { I18nContext, translate, type I18n } from "./lib/i18n";
import History from "./views/History";
import NewDownload from "./views/NewDownload";
import Queue from "./views/Queue";
import Settings from "./views/Settings";

export default function App() {
  return (
    <ErrorBoundary>
      <AppProvider>
        <Shell />
      </AppProvider>
    </ErrorBoundary>
  );
}

function Shell() {
  const { settings, tab } = useApp();

  const i18n = useMemo<I18n>(() => {
    const lang = settings.language;
    return {
      lang,
      dir: lang === "fa" ? "rtl" : "ltr",
      t: (key, vars) => translate(lang, key, vars),
    };
  }, [settings.language]);

  return (
    <I18nContext.Provider value={i18n}>
      <div className="app">
        <Sidebar />

        <div className="main-col">
          <Topbar />
          <main className="content">
            {tab === "new" && <NewDownload />}
            {tab === "queue" && <Queue />}
            {tab === "history" && <History />}
            {tab === "settings" && <Settings />}
          </main>
          <StatusBar />
        </div>

        <ClipboardToast />
        <ToolsOverlay />
      </div>
    </I18nContext.Provider>
  );
}
