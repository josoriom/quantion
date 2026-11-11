import { TypeText } from "./TypeText";
import { IonicText } from "./IonicText";

export function ReturnsLine({ returns, language }) {
  const type = returns?.languages[language];
  if (!type) return null;
  return (
    <section className="block">
      <h3>Returns</h3>
      <p className="returns">
        <span className="type">
          <TypeText text={type} />
        </span>
        <span>
          <IonicText text={returns.meaning} />
        </span>
      </p>
    </section>
  );
}
