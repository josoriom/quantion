# Limits of the Ask helper

The helper runs in the browser with one small embedding model (MiniLM, 23 MB). It has no language model, so every answer is a template filled from the docs. These are the known limits, each confirmed by `npm run ask-eval`.

- **No reasoning or free text.** Answers are templates. "Why" questions only work when `guide.json` covers them. Example: "Why does my peak have two apexes?" gets the refusal, not an explanation.
- **One question per message.** The question becomes one vector, so two questions blur together. Example: "How do I open a file and find peaks above 1000?" answers only one of them.
- **Numbers need a word next to them.** A bare number is placed by its context words and a few priors (rt is under 60 min, m/z is over 20, tolerances are small). Example: "Noise level of the EIC at 445.12" is read as `mz`, but "Set it to 445.12" may go to the wrong option.
- **Synonyms and typos are a fixed list.** Only the words in `words.js` are mapped (mass → `mz`, chromatogram → `eic`, cvParams → metadata…). Typos are fixed only for words of 5+ letters within 1–2 edits of an API name. Example: "intesity" is fixed, "intnsty" is not.
- **Function names in plain words are not names.** "find peaks" is a hint, "find_peaks" or "findPeaks" is a name and always wins. Example: "How do I find peaks for SRM transitions?" goes to `find_transitions`; "How do I use find_peaks for transitions?" stays on `find_peaks`.
- **Follow-ups need a cue.** A message continues the earlier topic only if it starts with "or", "and", "what about", refers back (it, that, same…) or is a short fragment of up to 6 words. A full new question starts fresh. Example: after `parse_ion`, "or path in local laptop" keeps `parse_ion`; "How do I read a path?" searches again.
- **A follow-up keeps a wrong function.** If the first answer picked the wrong function, "Can it use 4 threads?" keeps it. Use the function chip under the answer to switch.
- **Guide items are hand-written.** Workflow questions are matched by keywords and by embedding against the questions in `guide.json`. A phrasing far from those falls back to the usage search. Example: "What is the plan for a metabolomics study?" is not covered.
- **Knowledge = this docs version.** The index is built from `public/api/<version>.json`. Nothing outside the docs is known, and older versions share the same guide text.
- **Option mentions without a value are weak.** When no number is given, the option is only named if its description is close to the question. Example: "Read an extra binary array from each scan" names `get_scans` but not `arrays`.
- **Nested options are ambiguous.** `find_peaks` carries baseline and noise options, so "Run airPLS for 20 iterations" goes to `find_peaks` (`max_iterations`), not `calculate_baseline`.
- **Snippets are edited templates, not run.** Values replace literals in the first call of the docs example. Rust snippets are not compiled, and `get_scans` ranges cannot be set in Rust. `dispose` has no Rust example, so the `parse_ion` example (with `drop`) is shown instead.
- **Input variants exist for three functions.** Path / URL / bytes forms are stored for `parse_ion`, `parse_mzml` and `load_model` only.
- **Off-topic detection is a margin.** A question is refused when its best descriptive match in the docs is not clearly above 40 everyday sentences. Everyday sentences that reuse a function name as a plain word (peak, target, calibrate) are the hardest; unambiguous MS words (m/z, ppm, EIC, mzML…) are never refused.
- **The first question downloads the model (23 MB) once.** If the model cannot load, commands, definitions and code review still work; usage questions get the refusal with the starter chips.
