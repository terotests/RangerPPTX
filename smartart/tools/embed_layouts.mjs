#!/usr/bin/env node
// builtin/*.xml → SaBuiltins.rgr: the layout definitions as string constants,
// so no target reads a file at run time (the Go MCP binary and the browser
// have no builtin/ directory). Comments and the whitespace between tags are
// dropped; the XML is otherwise as written.
//
//   node gallery/pptx/smartart/tools/embed_layouts.mjs          write it
//   node gallery/pptx/smartart/tools/embed_layouts.mjs --check  fail if stale
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const dir = path.join(here, "..", "builtin");
const out = path.join(here, "..", "SaBuiltins.rgr");

const files = fs.readdirSync(dir).filter((f) => f.endsWith(".xml")).sort();
const names = files.map((f) => f.slice(0, -4));

function compact(xml) {
  return xml
    .replace(/<\?xml[^>]*\?>/, "")
    .replace(/<!--[\s\S]*?-->/g, "")
    .replace(/>\s+</g, "><")
    .trim();
}

function literal(s) {
  if (/[\\\n\r\t]/.test(s)) throw new Error("a layout holds a backslash or a control character; this generator does not escape those");
  return '"' + s.replace(/"/g, '\\"') + '"';
}

let src = `; SPDX-License-Identifier: AGPL-3.0-or-later
; ==============================================================================
; SaBuiltins — the built-in layout definitions (GENERATED, do not edit)
; ==============================================================================
;
; Made from smartart/builtin/*.xml by smartart/tools/embed_layouts.mjs. Edit
; the XML and run the tool; npm run pptx:smartart:test fails while this file
; is out of date.
; ==============================================================================

class SaBuiltins {

    sfn names:string () {
        return ${literal(names.join(", "))}
    }

    sfn layoutXml:string (name:string) {
`;
for (const n of names) src += `        if (name == ${literal(n)}) {\n            return (SaBuiltins.${n}())\n        }\n`;
src += `        return ""\n    }\n`;
for (const [i, n] of names.entries()) {
  const xml = compact(fs.readFileSync(path.join(dir, files[i]), "utf8"));
  // in pieces, so no single literal is enormous
  const pieces = xml.match(/[\s\S]{1,160}(?=<|$)|[\s\S]{1,160}/g) || [];
  src += `\n    sfn ${n}:string () {\n        def s:string ""\n`;
  for (const p of pieces) src += `        s = (s + ${literal(p)})\n`;
  src += `        return s\n    }\n`;
}
src += `}\n`;

if (process.argv.includes("--check")) {
  const have = fs.existsSync(out) ? fs.readFileSync(out, "utf8") : "";
  if (have !== src) {
    console.error("SaBuiltins.rgr is out of date: run node gallery/pptx/smartart/tools/embed_layouts.mjs");
    process.exit(1);
  }
  console.log("SaBuiltins.rgr is up to date");
} else {
  fs.writeFileSync(out, src);
  console.log(`wrote SaBuiltins.rgr (${names.join(", ")})`);
}
