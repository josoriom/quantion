import { AskChat, AskProvider } from "./components/AskBox";
import { EntryView } from "./components/EntryView";
import { Header } from "./components/Header";
import { LanguageTabs } from "./components/LanguageTabs";
import { Sidebar } from "./components/Sidebar";
import { DocumentationProvider, useDocumentation } from "./state/DocumentationContext";

function Layout() {
  const { state } = useDocumentation();
  if (state.status === "failed") return <p className="status">The documentation could not be loaded.</p>;
  if (!state.document) return <p className="status">Loading…</p>;
  return (
    <div className="layout">
      <Sidebar />
      <main>
        <LanguageTabs />
        <EntryView />
      </main>
    </div>
  );
}

export function App() {
  return (
    <DocumentationProvider>
      <AskProvider>
        <Header />
        <Layout />
        <AskChat />
      </AskProvider>
    </DocumentationProvider>
  );
}
