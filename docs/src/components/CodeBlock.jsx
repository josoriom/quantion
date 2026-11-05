import { splitIntoTokens } from "../highlight";
import { CopyButton } from "./CopyButton";

export function CodeBlock({ title, code, language }) {
  return (
    <section className="block">
      <h3>
        {title}
        <CopyButton text={code} label={`Copy ${title}`} />
      </h3>
      <pre>
        {splitIntoTokens(code, language).map((token, index) =>
          token.kind ? (
            <span key={index} className={`token-${token.kind}`}>
              {token.text}
            </span>
          ) : (
            token.text
          ),
        )}
      </pre>
    </section>
  );
}
