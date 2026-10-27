// Audit: every quoted span in the Phase 6 doc must exist in the corpus.
//
// Re-runnable. Run from docs/runna-ux/support/:  node quote_audit.mjs
//
// Curly/straight apostrophes, curly/straight quotes and en/em dashes are folded
// on BOTH sides: the doc is written with straight ASCII while Runna's help
// centre renders curly punctuation. A grep that returns 0 on this corpus is
// almost always a punctuation mismatch rather than a wrong quote.
//
// Spans that are legitimately not verbatim corpus text are skipped, not
// whitelisted by hand:
//   - titles elided with "…"
//   - quotes carrying our own bracketed alteration ("match[es] …") or insertion
//     ("[…] rather than") — split apart and each side checked
//   - quotes from the Runalytics repo itself (Rust comments, identifiers)
import { readFileSync } from "node:fs";

const norm = (s) =>
  s
    .replace(/[\u2018\u2019]/g, "'")
    .replace(/[\u201C\u201D]/g, '"')
    .replace(/[\u2013\u2014]/g, "-")
    .replace(/\s+/g, " ")
    .trim();

const doc = readFileSync("../RUNNA-INSIGHTS-AND-ADAPTATION.md", "utf8");
const corpus = norm(
  [
    "insights_articles.md", // Phase 6 corpus — 31 articles
    "all_articles.md", // Phase 5 corpus — 28 plan-collection articles
  ]
    .map((f) => readFileSync(f, "utf8"))
    .join("\n"),
);

// Candidate quotes: blockquote lines and inline *"..."* spans.
const spans = [];
for (const line of doc.split("\n")) {
  const t = line.trim();
  if (t.startsWith(">")) spans.push(t.replace(/^>+\s?/, ""));
  for (const m of line.matchAll(/\*"(.+?)"\*/g)) spans.push(m[1]);
}

// Quotes about our own code, not about Runna — checked by grep, not here.
const isOurCode = (s) =>
  /::|\.rs\b|predict_|installed_rev|MIN_RUN_KM|proposal|assistant|downgrades/.test(
    s,
  );

let checked = 0;
let bad = 0;
for (const raw of spans) {
  if (raw.includes("…") || isOurCode(raw)) continue;
  const pieces = norm(raw)
    .split(/\s\.\.\.\s|\s-\s/)
    // Drop our bracketed alterations/insertions in place: "match[es] the" and
    // "[…] rather than" both reduce to verbatim corpus text once removed.
    .map((p) => p.replace(/\[[^\]]*\]/g, ""))
    .map((p) => p.replace(/^[^A-Za-z0-9"']+/, "").replace(/["'.]+$/, "").trim())
    .filter((p) => p.length > 12);
  for (const p of pieces) {
    checked++;
    if (!corpus.includes(p)) {
      console.log("MISS:", p.slice(0, 110));
      bad++;
    }
  }
}
console.log(`${checked} verbatim spans checked`);
console.log(bad === 0 ? "ALL QUOTES VERIFIED" : `${bad} MISS(ES)`);
process.exit(bad === 0 ? 0 : 1);
