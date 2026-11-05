import { TypeText } from "./TypeText";
import { LinkedText } from "./LinkedText";

function getVisibleRows(rows, language) {
  const visibleRows = [];
  for (const row of rows) {
    if (row.languages[language]) visibleRows.push(row);
  }
  return visibleRows;
}

function hasAnyDefault(rows, language) {
  for (const row of rows) {
    if (row.languages[language].default !== undefined) return true;
  }
  return false;
}

export function DetailTable({ title, rows, language }) {
  const visibleRows = getVisibleRows(rows, language);
  if (!visibleRows.length) return null;
  const hasDefault = hasAnyDefault(visibleRows, language);
  return (
    <section className="block">
      <h3>{title}</h3>
      <div className="table-wrap">
        <table className="details">
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              {hasDefault && <th>Default</th>}
              <th>Meaning</th>
            </tr>
          </thead>
          <tbody>
            {visibleRows.map((row) => {
              const cell = row.languages[language];
              return (
                <tr key={cell.name}>
                  <td className="name">{cell.name}</td>
                  <td className="type">
                    <TypeText text={cell.type} />
                  </td>
                  {hasDefault && <td className="default">{cell.default ?? "—"}</td>}
                  <td>
                    <LinkedText text={row.meaning} />
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </section>
  );
}
