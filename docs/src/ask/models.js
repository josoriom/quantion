export const models = {
  embedder: { name: "helper", dtype: "q8" },
  folder: "models/",
};

export const limits = {
  question: 250,
  code: 1000,
  answer: 1000,
  entries: 3,
  earlierWeight: 0.5,
  nameBonus: 0.08,
  nameWeightCap: 3,
  keyBonus: 0.15,
  minScore: 0.5,
  topicMargin: 0.1,
  mainKeyBonus: 0.1,
  guideScore: 0.6,
  guideLead: 0.05,
  highScore: 0.62,
  highGap: 0.08,
  highMargin: 0.2,
  lowGap: 0.02,
};

export const refusal = "I only have knowledge about Quant·ion. I'm not sure what you're asking for, sorry!";

