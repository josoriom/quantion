import { actions } from "../state/documentationState";
import { useDocumentation } from "../state/DocumentationContext";

export function LanguageTabs() {
  const { state, dispatch } = useDocumentation();
  return (
    <div className="tabs" role="tablist" aria-label="Language">
      {state.document.languages.map((language) => (
        <button
          key={language.key}
          type="button"
          role="tab"
          aria-selected={state.language === language.key}
          onClick={() => dispatch({ type: actions.languageChosen, language: language.key })}
        >
          {language.label}
        </button>
      ))}
    </div>
  );
}
