import { createContext, useContext, useEffect, useRef, useState } from "react";

import { createAsker, createClassifier, createEmbedder, findOptionsType, getStarters, listUnderstood, writeAnswer, writePlacement } from "../ask/ask";
import { commandList } from "../ask/commands";
import { writeExplanation } from "../ask/meaning";
import { limits, models, refusal } from "../ask/models";
import phrases from "../ask/phrases.json";
import { languageLabels } from "../ask/review";
import { runtimeFiles } from "../ask/runtime";
import { buildTemplateSnippet } from "../ask/snippet";
import { getDisplayName } from "../names";
import { actions, findApiFile, findEntry, findLanguage } from "../state/documentationState";
import { useDocumentation } from "../state/DocumentationContext";
import { CodeBlock } from "./CodeBlock";

let embedderPromise = null;
let classifierPromise = null;
const loaded = new Map();
const modelFailure = "The search model could not load, so I can only run commands, definitions and code checks for now.";

function getDevice() {
  if (typeof navigator !== "undefined" && navigator.gpu) return "webgpu";
  return "wasm";
}

function trackProgress(onProgress) {
  return (event) => {
    if (event.status !== "progress" || !event.total) return;
    loaded.set(event.file, { loaded: event.loaded, total: event.total });
    let done = 0;
    let total = 0;
    for (const item of loaded.values()) {
      done += item.loaded;
      total += item.total;
    }
    onProgress(Math.round((100 * done) / total));
  };
}

function loadEmbedder(onProgress) {
  if (!embedderPromise) {
    const base = import.meta.env.BASE_URL;
    embedderPromise = createEmbedder({
      device: getDevice(),
      onProgress: trackProgress(onProgress),
      modelFolder: `${base}${models.folder}`,
      runtimeFiles,
    });
    embedderPromise.catch(() => {
      embedderPromise = null;
    });
  }
  return embedderPromise;
}

async function readClassifier() {
  const folder = `${import.meta.env.BASE_URL}${models.folder}${models.embedder.name}/`;
  const [labels, weights] = await Promise.all([fetch(`${folder}${models.answers.labels}`), fetch(`${folder}${models.answers.weights}`)]);
  if (!labels.ok || !weights.ok) throw new Error("No Ask answer list");
  return createClassifier(await labels.json(), await weights.arrayBuffer());
}

function loadClassifier() {
  if (!classifierPromise) {
    classifierPromise = readClassifier();
    classifierPromise.catch(() => {
      classifierPromise = null;
    });
  }
  return classifierPromise;
}

function hasItem(entry, name) {
  return [...(entry.arguments ?? []), ...(entry.fields ?? [])].some((item) =>
    Object.values(item.languages ?? {}).some((language) => language?.name === name),
  );
}

function findLinkTarget(name, document, owners) {
  const bare = name.replace(/\(\)$/, "");
  const entry = document.entries.find((item) => item.name === bare || getDisplayName(item, "js") === bare);
  if (entry) return entry.name;
  const group = document.groups.find((item) => item.name.toLowerCase() === bare.toLowerCase());
  if (group) return group.name;
  for (const owner of owners) {
    const ownerEntry = findEntry(document, owner);
    if (!ownerEntry) continue;
    if (hasItem(ownerEntry, bare)) return ownerEntry.name;
    const options = findOptionsType(ownerEntry, document);
    if (options && hasItem(options, bare)) return options.name;
  }
  return document.entries.find((item) => hasItem(item, bare))?.name ?? null;
}

function AnswerText({ text, owners = [] }) {
  const { state, dispatch } = useDocumentation();
  return text.split(/(`[^`]+`|https?:\/\/[^\s`]+[^\s`.,)])/).map((part, position) => {
    if (/^https?:\/\//.test(part)) {
      return (
        <a key={position} className="text-link" href={part} target="_blank" rel="noreferrer">
          {part}
        </a>
      );
    }
    if (!part.startsWith("`")) return part;
    const name = part.slice(1, -1);
    const target = findLinkTarget(name, state.document, owners.filter(Boolean));
    if (!target) return <code key={position}>{name}</code>;
    return (
      <button key={position} type="button" className="ask-code-link" onClick={() => dispatch({ type: actions.entryChosen, entryName: target })}>
        <code>{name}</code>
      </button>
    );
  });
}

function DocsLink({ link }) {
  const { dispatch } = useDocumentation();
  return (
    <button type="button" className="text-link" onClick={() => dispatch({ type: actions.entryChosen, entryName: link.entryName })}>
      {link.label}
    </button>
  );
}

function Source({ link }) {
  if (!link) return null;
  return (
    <p className="ask-sources">
      For more information, see <DocsLink link={link} />.
    </p>
  );
}

function Chips({ items, label, onPick, className = "" }) {
  if (!items || items.length === 0) return null;
  return (
    <div className={`ask-chips ${className}`}>
      {label && <span className="ask-chips-label">{label}</span>}
      {items.map((item, position) => (
        <button key={`${item.label}-${position}`} type="button" className="ask-chip" onClick={() => onPick(item)}>
          {item.label}
        </button>
      ))}
    </div>
  );
}

function QuestionChips({ items, label }) {
  const { askQuestion } = useContext(AskContext);
  return <Chips items={items} label={label} onPick={(item) => askQuestion(item.question ?? item.label)} />;
}

function ExplanationResult({ explanation }) {
  const { state } = useDocumentation();
  const { main, link } = writeExplanation(explanation, state.document, state.language);
  return (
    <div className="ask-reply">
      <p className="ask-answer">
        <AnswerText text={main} owners={[link?.entryName]} />
      </p>
      <Source link={link} />
    </div>
  );
}

function ReviewResult({ review, link }) {
  const { state } = useDocumentation();
  const problems = review.fixes.length + review.notes.length;
  return (
    <div className="ask-reply">
      <p className="ask-answer">
        {problems === 0
          ? `No problems found: every Quant·ion function and option in this ${languageLabels[review.language]} code exists in the API.`
          : `Found ${problems} problem${problems > 1 ? "s" : ""} in this ${languageLabels[review.language]} code.`}
      </p>
      {problems > 0 && (
        <ul className="ask-meanings">
          {review.fixes.map((fix) => (
            <li key={`${fix.from}-${fix.to}`}>
              <AnswerText text={`\`${fix.from}\` → \`${fix.to}\`: ${fix.reason}`} owners={review.entries.map((item) => item.name)} />
            </li>
          ))}
          {review.notes.map((note) => (
            <li key={note}>
              <AnswerText text={note} owners={review.entries.map((item) => item.name)} />
            </li>
          ))}
        </ul>
      )}
      {review.fixes.length > 0 && <CodeBlock title={`Corrected ${languageLabels[review.language]} code`} code={review.code} language={review.language} />}
      {state.language !== review.language && <p className="ask-status">Checked as {languageLabels[review.language]}, from the code's syntax.</p>}
      <Source link={link ?? (review.entries[0] && { entryName: review.entries[0].name, label: getDisplayName(review.entries[0], review.language) })} />
    </div>
  );
}

function pickPhrase(list, text) {
  return list[[...text].reduce((sum, character) => sum + character.charCodeAt(0), 0) % list.length];
}

function Unsure({ message, result }) {
  const { state } = useDocumentation();
  const alternatives = (result.entries ?? []).map((entry) => ({ label: getDisplayName(entry, state.language), question: `How do I use ${entry.name}?` }));
  return (
    <div className="ask-reply">
      <p className="ask-answer">{pickPhrase(phrases.unsure, message.question)}</p>
      <QuestionChips items={alternatives} />
    </div>
  );
}

function Refusal({ result }) {
  const { state } = useDocumentation();
  const alternatives = (result.entries ?? []).map((entry) => ({ label: getDisplayName(entry, state.language), question: `How do I use ${entry.name}?` }));
  return (
    <div className="ask-reply">
      <p className="ask-answer">{refusal}</p>
      {result.noModel && <p className="ask-status">{modelFailure}</p>}
      <QuestionChips items={alternatives} label="Did you mean:" />
      <QuestionChips items={getStarters()} label="Try one of these:" />
    </div>
  );
}

function InstallList({ languages }) {
  const { state } = useDocumentation();
  const shown = languages.filter((item) => item.key === state.language);
  return (
    <ul className="ask-install">
      {shown.map((item) => (
        <li key={item.key}>
          <strong>{item.label}</strong>
          {item.registry ? (
            <p>
              <a className="text-link" href={item.registry.address} target="_blank" rel="noreferrer">
                {item.registry.label}
              </a>
              : <code>{item.command}</code>
            </p>
          ) : (
            <p>{item.pending}</p>
          )}
          {item.github && (
            <p>
              <a className="text-link" href={item.github.address} target="_blank" rel="noreferrer">
                GitHub
              </a>
              : <code>{item.github.command}</code>
            </p>
          )}
          {item.platforms && <p className="ask-install-note">{item.platforms}</p>}
        </li>
      ))}
    </ul>
  );
}

function GuideResult({ result }) {
  const { state } = useDocumentation();
  const answer = result.answer ?? result.guide.answer;
  const functions = result.entries
    .filter((entry) => !answer.includes(`\`${entry.name}\``))
    .map((entry) => ({ entryName: entry.name, label: getDisplayName(entry, state.language) }));
  return (
    <div className="ask-reply">
      <p className="ask-answer">
        <AnswerText text={answer} owners={result.guide.functions} />
      </p>
      {functions.length > 0 && (
        <p className="ask-sources">
          Functions:{" "}
          {functions.map((link) => (
            <DocsLink key={link.entryName} link={link} />
          ))}
        </p>
      )}
      {result.guide.languages && <InstallList languages={result.guide.languages} />}
      {result.guide.links && (
        <p className="ask-sources">
          {result.guide.links.map((link) => (
            <a key={link.address} className="text-link" href={link.address} target="_blank" rel="noreferrer">
              {link.label} ↗
            </a>
          ))}
        </p>
      )}
      <Source link={result.link} />
      <QuestionChips items={result.followUps} />
    </div>
  );
}

function FaqResult({ result }) {
  return (
    <div className="ask-reply">
      <p className="ask-answer">
        <AnswerText text={result.faq.note} owners={[result.faq.target.split("@").pop()]} />
      </p>
      <Source link={result.link} />
    </div>
  );
}

function ListResult({ result }) {
  const { state } = useDocumentation();
  const { option, entries } = result.list;
  const names = entries.map((entry) => `\`${getDisplayName(entry, state.language)}\``);
  const text = entries.length === 1 ? `Only ${names[0]} takes \`${option}\`.` : `These functions take \`${option}\`: ${names.join(", ")}.`;
  return (
    <div className="ask-reply">
      <p className="ask-answer">
        <AnswerText text={text} owners={entries.map((entry) => entry.name)} />
      </p>
      <QuestionChips items={result.followUps} label="Show me:" />
    </div>
  );
}

function CommandResult({ result }) {
  const { state, dispatch } = useDocumentation();
  const { askQuestion } = useContext(AskContext);
  const { command } = result;
  const languageLabel = findLanguage(state.document, state.language)?.label ?? state.language;
  useEffect(() => {
    if (command.command === "docs" && command.target) dispatch({ type: actions.entryChosen, entryName: command.target.entryName });
  }, [command, dispatch]);
  if (command.command === "help" || command.command === "unknown") {
    return (
      <div className="ask-reply">
        {command.failure && <p className="ask-answer">{command.failure}</p>}
        <ul className="ask-meanings">
          {command.commands.map((item) => (
            <li key={item.name}>
              <code>{item.usage}</code> {item.meaning}
            </li>
          ))}
        </ul>
        {command.examples && <Chips items={command.examples.map((question) => ({ label: question }))} label="Try:" onPick={(item) => askQuestion(item.label)} />}
      </div>
    );
  }
  if (command.command === "guide") {
    return (
      <div className="ask-reply">
        <p className="ask-answer">What do you want to do?</p>
        <Chips items={command.items.map((item) => ({ label: item.questions[0], question: item.questions[0] }))} onPick={(item) => askQuestion(item.question)} />
      </div>
    );
  }
  if (command.command === "docs") {
    return (
      <div className="ask-reply">
        <p className="ask-answer">{command.target ? <>Opened the docs page <DocsLink link={{ entryName: command.target.entryName, label: command.label }} />.</> : <AnswerText text={command.failure} />}</p>
      </div>
    );
  }
  if (command.command === "example") {
    const link = command.entry ? { entryName: command.entry.name, label: getDisplayName(command.entry, state.language) } : null;
    return (
      <div className="ask-reply">
        {command.failure && (
          <p className="ask-answer">
            <AnswerText text={command.failure} />
          </p>
        )}
        {command.snippet?.valid && <CodeBlock title={`Template from the docs · ${languageLabel} · ${link.label}`} code={command.snippet.code} language={state.language} />}
        {command.snippet && !command.snippet.valid && <p className="ask-answer">I cannot place those values in the {languageLabel} template ({command.snippet.reason}).</p>}
        <Source link={link} />
      </div>
    );
  }
  if (command.command === "fix") {
    if (command.failure) {
      return (
        <div className="ask-reply">
          <p className="ask-answer">
            <AnswerText text={command.failure} />
          </p>
        </div>
      );
    }
    return <ReviewResult review={command.review} />;
  }
  return null;
}

function ValueChip({ chip, onChange }) {
  const [isEditing, setIsEditing] = useState(false);
  const [text, setText] = useState(String(chip.value));
  if (!isEditing) {
    return (
      <button type="button" className="ask-chip ask-chip-understood" title="Change this value" onClick={() => setIsEditing(true)}>
        {chip.label}
      </button>
    );
  }
  function finish() {
    setIsEditing(false);
    const value = typeof chip.value === "number" ? Number(text) : typeof chip.value === "boolean" ? text.trim().toLowerCase() === "true" : text.trim();
    if (typeof chip.value === "number" && !Number.isFinite(value)) return;
    onChange(value);
  }
  return (
    <form
      className="ask-chip ask-chip-edit"
      onSubmit={(event) => {
        event.preventDefault();
        finish();
      }}
    >
      <span>{chip.name} =</span>
      <input type="text" value={text} autoFocus aria-label={`New value for ${chip.name}`} onChange={(event) => setText(event.target.value)} onBlur={finish} />
    </form>
  );
}

function Understood({ message }) {
  const { state } = useDocumentation();
  const { chooseFunction, changeValue } = useContext(AskContext);
  const [isPicking, setIsPicking] = useState(false);
  const chips = listUnderstood(message.result, state.document, state.language);
  const alternatives = message.result.entries.slice(1);
  return (
    <div className="ask-chips ask-understood">
      <span className="ask-chips-label">I read:</span>
      {chips.map((chip) =>
        chip.kind === "function" ? (
          <span key="function" className="ask-pick">
            <button type="button" className="ask-chip ask-chip-understood" title="Change the function" onClick={() => setIsPicking((current) => !current)}>
              {chip.label}
            </button>
            {isPicking && alternatives.length > 0 && (
              <span className="ask-pick-menu">
                {alternatives.map((entry) => (
                  <button
                    key={entry.name}
                    type="button"
                    className="ask-chip"
                    onClick={() => {
                      setIsPicking(false);
                      chooseFunction(message.id, entry.name);
                    }}
                  >
                    {getDisplayName(entry, state.language)}
                  </button>
                ))}
              </span>
            )}
          </span>
        ) : (
          <ValueChip key={`${chip.owner}.${chip.name}`} chip={chip} onChange={(value) => changeValue(message.id, chip.owner, chip.name, value)} />
        ),
      )}
    </div>
  );
}

function UsageResult({ message }) {
  const { state } = useDocumentation();
  const { chooseFunction } = useContext(AskContext);
  const { result } = message;
  const { language } = state;
  const { confidence } = result.reason;
  if (confidence === "low") return result.entries?.length ? <Unsure message={message} result={result} /> : <Refusal result={result} />;
  const languageLabel = findLanguage(state.document, language)?.label ?? language;
  const entry = result.entries[0];
  const snippet = buildTemplateSnippet(result.values, result.templateValues, state.document, language);
  const variantCode = result.variant && !result.variant.refused ? result.variant[language] : null;
  const hasOwnValues = Object.keys(result.values ?? {}).length > 1 || Object.keys(result.templateValues ?? {}).length > 0;
  let code = null;
  let codeEntry = entry;
  if (variantCode && !hasOwnValues) code = variantCode;
  else if (snippet.valid) {
    code = snippet.code;
    codeEntry = findEntry(state.document, snippet.function);
  } else if (result.example) {
    code = result.example.languages[language].example;
    codeEntry = result.example;
  }
  return (
    <div className="ask-reply">
      <p className="ask-answer">
        {pickPhrase(phrases.sure, message.question)}{" "}
        <AnswerText text={result.faq ? result.faq.note : writeAnswer(result, state.document, language)} owners={result.entries.map((item) => item.name)} />
      </p>
      {confidence === "medium" && (
        <Chips items={result.alternatives} label="Did you mean:" onPick={(item) => chooseFunction(message.id, item.entryName)} />
      )}
      <Understood message={message} />
      {code && <CodeBlock title={`Template from the docs · ${languageLabel} · ${getDisplayName(codeEntry, language)}`} code={code} language={language} />}
      <Source link={result.link} />
      <QuestionChips items={result.followUps} />
    </div>
  );
}

function AskResult({ message }) {
  const { state } = useDocumentation();
  const { language } = state;
  const { result } = message;
  const { route } = result.reason;
  if (route === "refused") return <Refusal result={result} />;
  if (route === "command") return <CommandResult result={result} />;
  if (route === "list") return <ListResult result={result} />;
  if (route === "faq" || route === "unsupported") return <FaqResult result={result} />;
  if (route === "review") return <ReviewResult review={result.review} link={result.link} />;
  if (route === "definition") return <ExplanationResult explanation={result.explanation} />;
  if (route === "guide") return <GuideResult result={result} />;
  if (route === "placement") {
    const placement = writePlacement(result.placement, language);
    const languageLabel = findLanguage(state.document, language)?.label ?? language;
    return (
      <div className="ask-reply">
        <p className="ask-answer">
          <AnswerText text={placement.text} owners={[placement.entry?.name, result.link?.entryName]} />
        </p>
        {placement.line && <CodeBlock title={`Template from the docs · ${languageLabel} · ${getDisplayName(placement.entry, language)}`} code={placement.line} language={language} />}
        <Source link={result.link} />
        <QuestionChips items={result.followUps} />
      </div>
    );
  }
  return <UsageResult message={message} />;
}

function AskMessage({ message, progress }) {
  return (
    <li className="ask-message">
      <p className="ask-question">{message.question}</p>
      {message.status === "busy" && (
        <p className="ask-status">{progress > 0 && progress < 100 ? `Loading the search model (first time only)… ${progress}%` : "Searching…"}</p>
      )}
      {message.failure && <p className="ask-status ask-rejected">{message.failure}</p>}
      {message.result && <AskResult message={message} />}
    </li>
  );
}

const AskContext = createContext(null);

function getPrevious(messages) {
  for (let position = messages.length - 1; position >= 0; position -= 1) {
    const { question, result } = messages[position];
    if (!result || result.reason.route === "command") continue;
    return { question, questionVector: result.questionVector, functionName: result.entries[0]?.name, option: result.option ?? null };
  }
  return null;
}

function withValue(result, owner, name, value) {
  if (result.values?.function === owner) return { ...result, values: { ...result.values, [name]: value } };
  return { ...result, templateValues: { ...result.templateValues, [owner]: { ...(result.templateValues?.[owner] ?? {}), [name]: value } } };
}

export function AskProvider({ children }) {
  const { state } = useDocumentation();
  const [isOpen, setIsOpen] = useState(false);
  const [messages, setMessages] = useState([]);
  const [progress, setProgress] = useState(0);
  const [modelFailed, setModelFailed] = useState(false);
  const askerRef = useRef({ key: null, asker: null });
  const apiFile = findApiFile(state.versions, state.version);
  const isBusy = messages.some((message) => message.status === "busy");

  function updateMessage(id, change) {
    setMessages((current) => current.map((message) => (message.id === id ? { ...message, ...change } : message)));
  }

  async function getAsker() {
    let embed = null;
    let classify = null;
    try {
      [embed, classify] = await Promise.all([loadEmbedder(setProgress), loadClassifier()]);
      setModelFailed(false);
    } catch (error) {
      console.error("Ask helper could not load its model", error);
      setModelFailed(true);
    }
    const key = `${apiFile}:${embed ? "model" : "none"}`;
    const current = askerRef.current;
    if (current.key !== key || current.document !== state.document) askerRef.current = { key, document: state.document, asker: createAsker({ embed, classify, document: state.document }) };
    return askerRef.current.asker;
  }

  async function askQuestion(asked) {
    if (!asked || !state.document || isBusy) return;
    const id = Date.now();
    setIsOpen(true);
    setMessages((current) => [...current, { id, question: asked, status: "busy" }]);
    try {
      const asker = await getAsker();
      updateMessage(id, { status: "done", result: await asker(asked, state.language, getPrevious(messages)) });
    } catch (error) {
      updateMessage(id, { status: "done", failure: error.message });
    }
  }

  async function chooseFunction(id, entryName) {
    const message = messages.find((item) => item.id === id);
    if (!message?.result || !askerRef.current.asker?.reread) return;
    const result = await askerRef.current.asker.reread(message.question, message.result, entryName, state.language);
    updateMessage(id, { result });
  }

  function changeValue(id, owner, name, value) {
    setMessages((current) => current.map((message) => (message.id === id ? { ...message, result: withValue(message.result, owner, name, value) } : message)));
  }

  function restart() {
    setMessages([]);
  }

  const value = { isOpen, setIsOpen, messages, progress, isBusy, modelFailed, askQuestion, chooseFunction, changeValue, restart };
  return <AskContext.Provider value={value}>{children}</AskContext.Provider>;
}

function CommandMenu({ text, onPick, highlighted }) {
  const typed = text.slice(1).split(/\s/)[0].toLowerCase();
  const items = commandList.filter((item) => item.name.startsWith(typed));
  if (items.length === 0 || /\s/.test(text.trim())) return null;
  return (
    <ul className="ask-menu" role="listbox">
      {items.map((item, position) => (
        <li key={item.name}>
          <button type="button" role="option" aria-selected={position === highlighted} className={position === highlighted ? "ask-menu-item ask-menu-active" : "ask-menu-item"} onMouseDown={(event) => event.preventDefault()} onClick={() => onPick(item)}>
            <code>{item.usage}</code>
            <span>{item.meaning}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}

function AskInput({ className, autoFocus }) {
  const { isBusy, askQuestion } = useContext(AskContext);
  const [question, setQuestion] = useState("");
  const [highlighted, setHighlighted] = useState(0);
  const showsMenu = question.startsWith("/") && !/\s/.test(question.trim());
  const menuItems = showsMenu ? commandList.filter((item) => item.name.startsWith(question.slice(1).toLowerCase())) : [];
  const maxLength = /^\/fix\b/.test(question) ? limits.code : limits.question;

  function submit(event) {
    event.preventDefault();
    const asked = question.trim();
    if (!asked || isBusy) return;
    setQuestion("");
    askQuestion(asked);
  }

  function pick(item) {
    setQuestion(item.name === "help" || item.name === "guide" ? `/${item.name}` : `/${item.name} `);
    setHighlighted(0);
  }

  function onKeyDown(event) {
    if (!showsMenu || menuItems.length === 0) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setHighlighted((current) => (current + 1) % menuItems.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setHighlighted((current) => (current - 1 + menuItems.length) % menuItems.length);
    } else if (event.key === "Tab" || (event.key === "Enter" && question !== `/${menuItems[highlighted].name}`)) {
      event.preventDefault();
      pick(menuItems[highlighted]);
    }
  }

  return (
    <div className="ask-input-holder">
      {showsMenu && <CommandMenu text={question} onPick={pick} highlighted={Math.min(highlighted, Math.max(menuItems.length - 1, 0))} />}
      <form className={`ask-field ${className}`} onSubmit={submit}>
        <input
          type="text"
          className="ask-input"
          placeholder="Ask, paste a snippet, or type / for commands…"
          aria-label="Ask a usage question"
          maxLength={maxLength}
          value={question}
          autoFocus={autoFocus}
          onChange={(event) => {
            setQuestion(event.target.value);
            setHighlighted(0);
          }}
          onKeyDown={onKeyDown}
        />
        <span className="ask-count" aria-live="polite">
          {question.length}/{maxLength}
        </span>
        <button type="submit" className="ask-button" disabled={isBusy || !question.trim()}>
          {isBusy ? "…" : "Ask"}
        </button>
      </form>
    </div>
  );
}

function AgentIcon() {
  return (
    <svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M12 3v3" />
      <circle cx="12" cy="2.5" r="0.8" fill="currentColor" />
      <rect x="4" y="6" width="16" height="12" rx="4" />
      <circle cx="9" cy="12" r="1.3" fill="currentColor" stroke="none" />
      <circle cx="15" cy="12" r="1.3" fill="currentColor" stroke="none" />
      <path d="M9.5 15.5h5" />
      <path d="M2 11v3M22 11v3" />
    </svg>
  );
}

const sizeKey = "ask-panel-size";
const smallest = { width: 360, height: 360 };
const resizeStep = 24;

function readSize() {
  try {
    const size = JSON.parse(localStorage.getItem(sizeKey));
    return size && Number.isFinite(size.width) && Number.isFinite(size.height) ? size : null;
  } catch {
    return null;
  }
}

function saveSize(size) {
  try {
    if (size) localStorage.setItem(sizeKey, JSON.stringify(size));
    else localStorage.removeItem(sizeKey);
  } catch {}
}

function fitSize(size) {
  return {
    width: Math.round(Math.min(Math.max(size.width, smallest.width), window.innerWidth - 32)),
    height: Math.round(Math.min(Math.max(size.height, smallest.height), window.innerHeight - 48)),
  };
}

function usePanelSize(panel) {
  const [size, setSize] = useState(readSize);

  function change(next) {
    const fitted = next ? fitSize(next) : null;
    setSize(fitted);
    saveSize(fitted);
  }

  function startDrag(event) {
    if (!panel.current) return;
    event.preventDefault();
    const box = panel.current.getBoundingClientRect();
    const start = { x: event.clientX, y: event.clientY, width: box.width, height: box.height };
    const handle = event.currentTarget;
    handle.setPointerCapture(event.pointerId);
    function move(moveEvent) {
      change({ width: start.width + start.x - moveEvent.clientX, height: start.height + start.y - moveEvent.clientY });
    }
    function stop() {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", stop);
      handle.removeEventListener("pointercancel", stop);
    }
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", stop);
    handle.addEventListener("pointercancel", stop);
  }

  function onKeyDown(event) {
    if (!panel.current) return;
    const box = panel.current.getBoundingClientRect();
    const steps = { ArrowLeft: [resizeStep, 0], ArrowRight: [-resizeStep, 0], ArrowUp: [0, resizeStep], ArrowDown: [0, -resizeStep] };
    const step = steps[event.key];
    if (!step) return;
    event.preventDefault();
    change({ width: box.width + step[0], height: box.height + step[1] });
  }

  return { size, startDrag, onKeyDown, reset: () => change(null) };
}

export function AskChat() {
  const { isOpen, setIsOpen, messages, progress, modelFailed, restart } = useContext(AskContext);
  const listEnd = useRef(null);
  const panel = useRef(null);
  const resize = usePanelSize(panel);

  useEffect(() => {
    if (!isOpen) return undefined;
    function closeOutside(event) {
      if (!panel.current || panel.current.contains(event.target)) return;
      if (event.target.closest?.('[role="tablist"][aria-label="Language"]')) return;
      setIsOpen(false);
    }
    document.addEventListener("pointerdown", closeOutside);
    return () => document.removeEventListener("pointerdown", closeOutside);
  }, [isOpen, setIsOpen]);

  useEffect(() => {
    listEnd.current?.scrollIntoView({ block: "end" });
  }, [messages, isOpen]);

  if (!isOpen) {
    return (
      <button type="button" className="ask-open" aria-label="Open the Quant·ion assistant" title="Ask about Quant·ion" onClick={() => setIsOpen(true)}>
        <AgentIcon />
      </button>
    );
  }

  return (
    <section ref={panel} className="ask-panel" aria-label="Ask about Quant·ion" style={resize.size ? { width: resize.size.width, height: resize.size.height, maxHeight: "none" } : undefined}>
      <button
        type="button"
        className="ask-resize"
        aria-label="Resize the chat: drag, or use the arrow keys. Double-click to reset."
        title="Drag to resize · double-click to reset"
        onPointerDown={resize.startDrag}
        onKeyDown={resize.onKeyDown}
        onDoubleClick={resize.reset}
      />
      <header className="ask-header">
        <span className="ask-title">
          <AgentIcon />
          Ask about Quant·ion
        </span>
        <button type="button" className="ask-restart" onClick={restart} disabled={messages.length === 0}>
          Restart the conversation
        </button>
        <button type="button" className="ask-close" aria-label="Close" onClick={() => setIsOpen(false)}>
          ×
        </button>
      </header>
      <ul className="ask-messages">
        {messages.length === 0 && (
          <li className="ask-hint">
            <p className="ask-notice" role="note">
              This helper only answers about how to use the Quant·ion toolkit.
            </p>
            <p>Ask, or pick one:</p>
            <QuestionChips items={getStarters()} />
            <p className="ask-tip">Questions with enough context get better answers.</p>
          </li>
        )}
        {modelFailed && <li className="ask-hint">{modelFailure}</li>}
        {messages.map((message) => (
          <AskMessage key={message.id} message={message} progress={progress} />
        ))}
        <li ref={listEnd} aria-hidden="true" />
      </ul>
      <AskInput className="ask-chat-input" autoFocus />
      <p className="ask-warning">Quant·ion usage only. Not perfect: provide enough context and double-check the answers.</p>
    </section>
  );
}
