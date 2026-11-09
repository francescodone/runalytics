import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import ts from "typescript";

const directory = new URL("../src/desk/", import.meta.url);
const locales = new URL("locales/", directory);
const english = JSON.parse(await readFile(new URL("en.json", locales), "utf8"));
const keys = Object.keys(english).sort();
const placeholders = value => [...value.matchAll(/\{\{\w+\}\}/g)].map(match => match[0]).sort();
const files = (await readdir(locales)).filter(file => file.endsWith(".json"));
for (const file of files) {
  const catalog = JSON.parse(await readFile(new URL(file, locales), "utf8"));
  assert.deepEqual(Object.keys(catalog).sort(), keys, `${file}: catalog keys differ from English`);
  for (const key of keys) {
    assert.equal(typeof catalog[key], "string", `${file}: ${key} is not a string`);
    assert.ok(catalog[key].trim(), `${file}: ${key} is empty`);
    assert.deepEqual(placeholders(catalog[key]), placeholders(english[key]), `${file}: ${key} has different interpolation values`);
    assert.ok(!/__7\d{3}__/.test(catalog[key]), `${file}: ${key} contains a translation marker`);
  }
}
let calls = 0;
for (const file of (await readdir(directory)).filter(file => /\.tsx?$/.test(file))) {
  const source = await readFile(new URL(file, directory), "utf8");
  const ast = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, file.endsWith("tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const visit = node => {
    if (ts.isCallExpression(node) && node.expression.getText(ast) === "t" && node.arguments[0] && ts.isStringLiteral(node.arguments[0])) {
      calls++;
      assert.ok(Object.prototype.hasOwnProperty.call(english, node.arguments[0].text), `${file}: missing English key ${node.arguments[0].text}`);
      const parent = node.parent;
      assert.ok(!(ts.isCallExpression(parent) && /(?:matchMedia|querySelector|addEventListener|removeEventListener)$/.test(parent.expression.getText(ast))), `${file}: machine API strings must not be translated`);
      assert.ok(!(ts.isBinaryExpression(parent) && ts.isPropertyAccessExpression(parent.left) && ["className", "id"].includes(parent.left.name.text)), `${file}: machine identifiers must not be translated`);
    }
    ts.forEachChild(node, visit);
  };
  visit(ast);
}
console.log(`i18n verified: ${files.length} catalogs, ${keys.length} keys each, ${calls} static translation calls; placeholders match.`);