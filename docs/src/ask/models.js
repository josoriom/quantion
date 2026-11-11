export const models = {
  embedder: { name: "helper-answer-9", dtype: "q8", pooling: "mean" },
  answers: { labels: "answers.json", weights: "answers.bin" },
  folder: "models/",
};

export const limits = {
  question: 250,
  code: 1000,
  answer: 1000,
  entries: 3,
  earlierWeight: 0.5,
  mainKeyBonus: 0.1,
  highProbability: 0.5,
  lowProbability: 0.2,
};

export const refusal = "I only have knowledge about Quant·ion. I'm not sure what you're asking for, sorry!";
