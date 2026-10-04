#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Every SmartArt diagram in a .pptx, as one file each.

    python3 extract_smartart.py deck.pptx [out-dir]

writes out-dir/<deck>-slide<N>-<k>.xml for the k-th diagram on slide N: a
Flat OPC package (pkg:package) holding the diagram's data, layout
definition, quick style, colour set and — when PowerPoint saved one — its
drawing. That is the file a Sliqtly deck takes as a whole PowerPoint
SmartArt (smartart/SaWhole.rgr):

    ![The org chart](media/deck-slide2-1.xml)

Only the standard library is used.
"""

import os
import posixpath
import re
import sys
import zipfile
from xml.sax.saxutils import quoteattr

PKG = "http://schemas.microsoft.com/office/2006/xmlPackage"
REL = "http://schemas.openxmlformats.org/officeDocument/2006/relationships"
TYPES = {
    "dm": "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml",
    "lo": "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml",
    "qs": "application/vnd.openxmlformats-officedocument.drawingml.diagramStyle+xml",
    "cs": "application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml",
    "dr": "application/vnd.ms-office.drawingml.diagramDrawing+xml",
}


def rels_of(z, part):
    """{rId: target part path} for a part."""
    d, f = posixpath.split(part)
    path = posixpath.join(d, "_rels", f + ".rels")
    if path not in z.namelist():
        return {}
    xml = z.read(path).decode("utf-8")
    out = {}
    for m in re.finditer(r"<Relationship\b[^>]*>", xml):
        tag = m.group(0)
        rid = re.search(r'\bId="([^"]+)"', tag)
        tgt = re.search(r'\bTarget="([^"]+)"', tag)
        if rid and tgt and 'TargetMode="External"' not in tag:
            out[rid.group(1)] = posixpath.normpath(posixpath.join(d, tgt.group(1)))
    return out


def body(xml_bytes):
    """A part's XML without its declaration, for pkg:xmlData."""
    text = xml_bytes.decode("utf-8")
    return re.sub(r"^﻿?\s*<\?xml[^>]*\?>\s*", "", text)


def slides(z):
    names = [n for n in z.namelist() if re.match(r"ppt/slides/slide\d+\.xml$", n)]
    return sorted(names, key=lambda n: int(re.search(r"(\d+)", posixpath.basename(n)).group(1)))


def extract(path, out_dir):
    z = zipfile.ZipFile(path)
    stem = os.path.splitext(os.path.basename(path))[0]
    written = []
    for slide in slides(z):
        n = re.search(r"(\d+)", posixpath.basename(slide)).group(1)
        xml = z.read(slide).decode("utf-8")
        rels = rels_of(z, slide)
        for k, m in enumerate(re.finditer(r"<dgm:relIds\b[^>]*>", xml), start=1):
            tag = m.group(0)
            parts = {}
            for key in ("dm", "lo", "qs", "cs"):
                rid = re.search(r"\br:" + key + r'="([^"]+)"', tag)
                if rid and rid.group(1) in rels and rels[rid.group(1)] in z.namelist():
                    parts[key] = rels[rid.group(1)]
            if "dm" not in parts:
                continue
            data = z.read(parts["dm"]).decode("utf-8")
            # the drawing: named from the data part, resolved from the slide
            ext = re.search(r'dataModelExt\b[^>]*\brelId="([^"]+)"', data)
            if ext and ext.group(1) in rels and rels[ext.group(1)] in z.namelist():
                parts["dr"] = rels[ext.group(1)]
            out = ['<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n',
                   f'<pkg:package xmlns:pkg="{PKG}">']
            for key in ("dm", "lo", "qs", "cs", "dr"):
                if key not in parts:
                    continue
                out.append(f'<pkg:part pkg:name={quoteattr("/" + parts[key])} '
                           f'pkg:contentType="{TYPES[key]}"><pkg:xmlData>')
                out.append(body(z.read(parts[key])))
                out.append("</pkg:xmlData></pkg:part>")
            out.append("</pkg:package>\n")
            name = os.path.join(out_dir, f"{stem}-slide{n}-{k}.xml")
            with open(name, "w", encoding="utf-8") as f:
                f.write("".join(out))
            written.append((name, sorted(parts)))
    return written


def main():
    if len(sys.argv) < 2:
        print(__doc__.strip())
        sys.exit(2)
    out_dir = sys.argv[2] if len(sys.argv) > 2 else "."
    os.makedirs(out_dir, exist_ok=True)
    found = extract(sys.argv[1], out_dir)
    if not found:
        print("no SmartArt in", sys.argv[1])
        sys.exit(1)
    for name, parts in found:
        print(name, "(" + ", ".join({"dm": "data", "lo": "layout", "qs": "style",
                                     "cs": "colours", "dr": "drawing"}[p] for p in parts) + ")")


if __name__ == "__main__":
    main()
