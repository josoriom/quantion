import { getDisplayName } from "../names";
import { actions, findEntry, findGroup, findLanguage, pages } from "../state/documentationState";
import { useDocumentation } from "../state/DocumentationContext";
import { CodeBlock } from "./CodeBlock";
import { DetailTable } from "./DetailTable";
import { GitHubIcon } from "./GitHubLinks";
import { GroupView } from "./GroupView";
import { RegistryLink } from "./RegistryLink";
import { ReturnsLine } from "./ReturnsLine";
import { VersionsView } from "./VersionsView";
import { IonicText } from "./IonicText";
import { ExperimentalBadge } from "./ExperimentalBadge";

function getWarning(entry, code) {
  if (code?.warning) return code.warning;
  return entry.warning;
}

function GroupBadge({ group }) {
  const { dispatch } = useDocumentation();
  return (
    <button type="button" className="badge" onClick={() => dispatch({ type: actions.entryChosen, entryName: group })}>
      {group}
    </button>
  );
}

function EntryHeading({ title, group, summary, children }) {
  return (
    <>
      <div className="entry-heading">
        <h2>{title}</h2>
        {group && <GroupBadge group={group} />}
        {children}
      </div>
      <p className="summary">
        <IonicText text={summary} />
      </p>
    </>
  );
}

function getInstallLanguage(language) {
  if (language === "r") return "r";
  return "shell";
}

function Overview({ overview, language }) {
  const languageLabel = findLanguage(useDocumentation().state.document, language).label;
  return (
    <>
      <EntryHeading title={overview.title} summary={overview.summary}>
        <RegistryLink language={language} />
      </EntryHeading>
      <CodeBlock title={`Install for ${languageLabel}`} code={overview.install[language]} language={getInstallLanguage(language)} />
      <section className="block">
        <h3>Projects</h3>
        <div className="projects">
          {overview.projects.map((project) => (
            <a key={project.name} className="project" href={project.address} target="_blank" rel="noreferrer">
              <span className="project-name">
                <GitHubIcon />
                {project.name}
              </span>
              <span className="project-description">{project.description}</span>
            </a>
          ))}
        </div>
      </section>
    </>
  );
}

function Entry({ entry, language }) {
  const languageLabel = findLanguage(useDocumentation().state.document, language).label;
  const code = entry.languages[language];
  const isType = entry.kind === "type";
  return (
    <>
      <EntryHeading title={getDisplayName(entry, language)} group={entry.group} summary={entry.summary}>
        {entry.experimental && <ExperimentalBadge />}
      </EntryHeading>
      {getWarning(entry, code) && (
        <p className="warning" role="note">
          <strong>Warning: </strong>
          <IonicText text={getWarning(entry, code)} />
        </p>
      )}
      {!code ? (
        <p className="missing">{`${entry.name} is not part of the ${languageLabel} API.`}</p>
      ) : (
        <>
          <CodeBlock title={`${languageLabel} ${isType ? "definition" : "signature"}`} code={code.signature} language={language} />
          {isType ? (
            <DetailTable title="Fields" rows={entry.fields} language={language} />
          ) : (
            <>
              <DetailTable title="Arguments" rows={entry.arguments} language={language} />
              <ReturnsLine returns={entry.returns} language={language} />
            </>
          )}
          {code.example && <CodeBlock title={`${languageLabel} example`} code={code.example} language={language} />}
          {code.output && <CodeBlock title={`${languageLabel} output`} code={code.output} language="text" />}
          {entry.expect && code.output && <p className="note">{entry.expect}</p>}
        </>
      )}
    </>
  );
}

export function EntryView() {
  const { state } = useDocumentation();
  if (state.entryName === pages.versions) return <VersionsView />;
  const entry = findEntry(state.document, state.entryName);
  if (entry) return <Entry entry={entry} language={state.language} />;
  const group = findGroup(state.document, state.entryName);
  if (group) return <GroupView group={group} />;
  return <Overview overview={state.document.overview} language={state.language} />;
}
