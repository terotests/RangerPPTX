#!/usr/bin/env node
// builtin/*.xml → src/smartart/sa_builtins.rs: the layout definitions as
// string constants, in Ranger's Rust syntax like the rest of the crate,
// so no target reads a file at run time (the Go MCP binary and the browser
// have no builtin/ directory). Comments and the whitespace between tags are
// dropped; the XML is otherwise as written.
//
// And src/smartart/sa_presets.rs: the preset geometries those layouts draw with (every
// shape type they name, and the four arrows a connector can be), cut from
// Ranger's gallery/office/geom/assets/presets.txt. The painter has no
// geometry of its own without that catalogue — a chevron would be a
// rectangle — and a host that is not the deck editor has nowhere to load the
// whole file from.
//
//   node gallery/pptx/smartart/tools/embed_layouts.mjs          write it
//   node gallery/pptx/smartart/tools/embed_layouts.mjs --check  fail if stale
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const dir = path.join(here, "..", "builtin");
const out = path.join(here, "..", "..", "src", "smartart", "sa_builtins.rs");
const presetsOut = path.join(here, "..", "..", "src", "smartart", "sa_presets.rs");
const catalogue = path.join(here, "..", "..", "..", "office", "geom", "assets", "presets.txt");

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

let src = `// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaBuiltins — the built-in layout definitions (GENERATED, do not edit)
//! ==============================================================================
//!
//! Made from smartart/builtin/*.xml by smartart/tools/embed_layouts.mjs. Edit
//! the XML and run the tool; npm run pptx:smartart:test fails while this file
//! is out of date.
//! ==============================================================================

use ranger::prelude::*;

pub struct SaBuiltins {}

impl SaBuiltins {
    pub fn new() -> Self {
        Self {}
    }
    pub fn names() -> String {
        ${literal(names.join(", "))}.to_string()
    }
    pub fn layoutXml(name: &str) -> String {
`;
for (const n of names) src += `        if name == ${literal(n)} {\n            return SaBuiltins::${n}();\n        }\n`;
src += `        String::new()\n    }\n`;
for (const [i, n] of names.entries()) {
  const xml = compact(fs.readFileSync(path.join(dir, files[i]), "utf8"));
  // in pieces, so no single literal is enormous
  const pieces = xml.match(/[\s\S]{1,160}(?=<|$)|[\s\S]{1,160}/g) || [];
  src += `    pub fn ${n}() -> String {\n        let mut s: String = String::new();\n`;
  for (const p of pieces) src += `        s.push_str(${literal(p)});\n`;
  src += `        s\n    }\n`;
}
src += `}\n`;

// --- the presets ---------------------------------------------------------
const wanted = new Set(["rect", "roundRect", "rightArrow", "leftArrow", "upArrow", "downArrow", "line"]);
for (const f of files) {
  const xml = fs.readFileSync(path.join(dir, f), "utf8");
  for (const m of xml.matchAll(/<dgm:shape[^>]*\stype="([^"]+)"/g)) {
    if (m[1] !== "none" && m[1] !== "conn") wanted.add(m[1]);
  }
}
const lines = fs.readFileSync(catalogue, "utf8").split(/\r?\n/);
const blocks = new Map();
let cur = null;
for (const ln of lines) {
  if (ln.startsWith("#")) {
    cur = ln.slice(1).trim();
    blocks.set(cur, [ln]);
  } else if (cur !== null) {
    blocks.get(cur).push(ln);
  }
}
const missing = [...wanted].filter((n) => !blocks.has(n));
if (missing.length) throw new Error("presets.txt has no " + missing.join(", "));
const presetNames = [...wanted].sort();
let psrc = `// SPDX-License-Identifier: AGPL-3.0-or-later
//! ==============================================================================
//! SaPresets — the preset geometries the built-in layouts draw with (GENERATED)
//! ==============================================================================
//!
//! Cut from gallery/office/geom/assets/presets.txt by
//! smartart/tools/embed_layouts.mjs: ${presetNames.join(", ")}.
//! SaRender loads them into the painter it draws a diagram with.
//! ==============================================================================

use ranger::prelude::*;

pub struct SaPresets {}

impl SaPresets {
    pub fn new() -> Self {
        Self {}
    }
    pub fn names() -> String {
        ${literal(presetNames.join(", "))}.to_string()
    }
    pub fn text() -> String {
        let mut s: String = String::new();
`;
for (const n of presetNames) {
  for (const ln of blocks.get(n)) {
    if (ln.trim() === "") continue;
    psrc += `        s.push_str(${literal(ln).slice(0, -1)}\\n");\n`;
  }
}
psrc += `        s\n    }\n}\n`;

const outputs = [[out, src, "sa_builtins.rs"], [presetsOut, psrc, "sa_presets.rs"]];
if (process.argv.includes("--check")) {
  let stale = false;
  for (const [file, text, name] of outputs) {
    const have = fs.existsSync(file) ? fs.readFileSync(file, "utf8") : "";
    if (have !== text) {
      console.error(name + " is out of date: run node gallery/pptx/smartart/tools/embed_layouts.mjs");
      stale = true;
    }
  }
  if (stale) process.exit(1);
  console.log("sa_builtins.rs and sa_presets.rs are up to date");
} else {
  for (const [file, text] of outputs) fs.writeFileSync(file, text);
  console.log(`wrote sa_builtins.rs (${names.join(", ")}) and sa_presets.rs (${presetNames.join(", ")})`);
}
