const addresses = {
  Ionic: "https://github.com/phenological/ionic",
  "josoriom/cipher": "https://github.com/josoriom/cipher",
};

export function IonicText({ text }) {
  const parts = text.split(/\b(Ionic|josoriom\/cipher)\b/);
  const output = [];
  for (let index = 0; index < parts.length; index++) {
    const part = parts[index];
    const isLink = index % 2 === 1;
    if (isLink) {
      output.push(
        <a key={index} className="prose-link" href={addresses[part]} target="_blank" rel="noreferrer">
          {part}
        </a>,
      );
    } else if (part) {
      output.push(part);
    }
  }
  return output;
}
