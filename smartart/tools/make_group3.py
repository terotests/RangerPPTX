#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Writes smartart/builtin/ for layout group 3 (PLAN_SMARTART.md, phase 4).

    python3 gallery/pptx/smartart/tools/make_group3.py

venn1, matrix1, target1, funnel1, gear1, arrow2, bList2, hProcess9,
lProcess2 and cycle4, in the layoutDef language, written for this engine
from the specification and from what PowerPoint draws — not copied from
Office. Several place each item by its position (the third ring, the second
gear), which the language says with a `choose` per position; that is what
this script writes out, so the files stay readable and the numbers stay in
one place. The files it writes are what the engine embeds
(tools/embed_layouts.mjs); edit this script, not them, and run both.
"""

import os

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "..", "builtin")
NS = 'xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"'
U = "urn:microsoft.com/office/officeart/2005/8/layout"
RULE = '<dgm:ruleLst><dgm:rule type="primFontSz" val="5" fact="NaN" max="NaN"/></dgm:ruleLst>'


def margins(ref="primFontSz", f=0.3):
    return "".join(f'<dgm:constr type="{t}" refType="{ref}" fact="{f}"/>' for t in ("tMarg", "bMarg", "lMarg", "rMarg"))


def c(type_, for_="", name="", ref="", fact=None, val=None, op="", refFor="", refName=""):
    a = f'<dgm:constr type="{type_}"'
    if for_:
        a += f' for="{for_}"'
    if name:
        a += f' forName="{name}"'
    if ref:
        a += f' refType="{ref}"'
    if refFor:
        a += f' refFor="{refFor}"'
    if refName:
        a += f' refForName="{refName}"'
    if op:
        a += f' op="{op}"'
    if fact is not None:
        a += f' fact="{fact:g}"'
    if val is not None:
        a += f' val="{val:g}"'
    return a + "/>"


def box(name, w, h, l=None, t=None, cx=None, cy=None, wref="w", href="h", xref="w", yref="h"):
    """A child of a composite: its size and where it goes, as fractions."""
    s = c("w", "ch", name, wref, w) + c("h", "ch", name, href, h)
    if l is not None:
        s += c("l", "ch", name, xref, l)
    if t is not None:
        s += c("t", "ch", name, yref, t)
    if cx is not None:
        s += c("ctrX", "ch", name, xref, cx)
    if cy is not None:
        s += c("ctrY", "ch", name, yref, cy)
    return s


def node(name, style, shape, presof='<dgm:presOf axis="desOrSelf" ptType="node"/>', tx="", constrs=None, rot=0, extra=""):
    shp = f'<dgm:shape type="{shape}"' + (f' rot="{rot}"' if rot else "") + "/>"
    if shape == "none":
        shp = '<dgm:shape type="none"/>'
    con = margins() if constrs is None else constrs
    return (f'<dgm:layoutNode name="{name}" styleLbl="{style}"><dgm:alg type="tx">{tx}</dgm:alg>{shp}'
            f"{presof}<dgm:constrLst>{con}</dgm:constrLst>{RULE}{extra}</dgm:layoutNode>")


def deco(name, style, shape, rot=0):
    """A shape with no text: geometry only."""
    r = f' rot="{rot}"' if rot else ""
    return f'<dgm:layoutNode name="{name}" styleLbl="{style}"><dgm:alg type="tx"/><dgm:shape type="{shape}"{r}/></dgm:layoutNode>'


def p(type_, val):
    return f'<dgm:param type="{type_}" val="{val}"/>'


def layout(short, title, comment, root):
    return (f'<?xml version="1.0" encoding="UTF-8" standalone="yes"?>\n'
            f"<!-- {short}, \"{title}\": {comment}\n"
            f"     Written for this engine by tools/make_group3.py; see default.xml. -->\n"
            f'<dgm:layoutDef {NS} uniqueId="{U}/{short}">\n'
            f'  <dgm:title val="{title}"/>\n  {root}\n</dgm:layoutDef>\n')


def by_pos(n_max, body_of):
    """forEach over the items, a branch per position (1 … n_max)."""
    out = '<dgm:forEach name="items" axis="ch" ptType="node" cnt="%d"><dgm:choose name="position">' % n_max
    for k in range(1, n_max + 1):
        tag = "if" if k < n_max else "else"
        cond = f' name="p{k}" func="pos" op="equ" val="{k}"' if tag == "if" else f' name="p{k}"'
        out += f"<dgm:{tag}{cond}>{body_of(k)}</dgm:{tag}>"
    return out + "</dgm:choose></dgm:forEach>"


def by_cnt(n_max, constrs_of):
    """Constraints chosen by how many items there are (1 … n_max)."""
    out = '<dgm:choose name="count">'
    for n in range(1, n_max + 1):
        tag = "if" if n < n_max else "else"
        cond = f' name="n{n}" axis="ch" ptType="node" func="cnt" op="equ" val="{n}"' if tag == "if" else f' name="n{n}"'
        out += f"<dgm:{tag}{cond}><dgm:constrLst>{constrs_of(n)}</dgm:constrLst></dgm:{tag}>"
    return out + "</dgm:choose>"


FONT = lambda name: c("primFontSz", "ch", name, op="equ", val=65)
FONTD = lambda name: c("primFontSz", "des", name, op="equ", val=65)


def venn1():
    root = ('<dgm:layoutNode name="venn"><dgm:varLst><dgm:dir/><dgm:resizeHandles val="exact"/></dgm:varLst>'
            '<dgm:choose name="two"><dgm:if name="side" axis="ch" ptType="node" func="cnt" op="equ" val="2">'
            f'<dgm:alg type="cycle">{p("stAng", 270)}{p("spanAng", 360)}</dgm:alg></dgm:if>'
            f'<dgm:else name="round"><dgm:alg type="cycle">{p("stAng", 0)}{p("spanAng", 360)}</dgm:alg></dgm:else></dgm:choose>'
            "<dgm:shape/><dgm:constrLst>"
            + c("w", "ch", "node", "w", 0.34) + c("h", "ch", "node", "w", refFor="ch", refName="node")
            + c("sibSp", ref="w", refFor="ch", refName="node", fact=-0.3) + FONT("node")
            + '</dgm:constrLst><dgm:forEach name="items" axis="ch" ptType="node">'
            + node("node", "vennNode1", "ellipse", constrs=margins("w", 0.25))
            + "</dgm:forEach></dgm:layoutNode>")
    return layout("venn1", "Basic Venn", "the items as overlapping circles, see-through where they meet.", root)


def matrix1():
    quads = ""
    anchors = {1: ("t", "l"), 2: ("t", "r"), 3: ("b", "l"), 4: ("b", "r")}
    def quad(k):
        v, h = anchors[k]
        return node("quad", "node1", "rect", tx=p("txAnchorVert", v) + p("parTxLTRAlign", h))
    grid = ('<dgm:layoutNode name="grid"><dgm:alg type="snake">' + p("bkpt", "fixed") + p("bkPtFixedVal", 2) + p("off", "off")
            + "</dgm:alg><dgm:shape/><dgm:constrLst>"
            + c("w", "ch", "quad", "w", 0.5) + c("h", "ch", "quad", "h", 0.5) + c("sibSp", ref="w", fact=0.0)
            + "</dgm:constrLst>"
            + by_pos(4, quad).replace('name="items"', 'name="quadrants"')
            + "</dgm:layoutNode>")
    title = node("title", "fgAcc1", "roundRect", presof='<dgm:presOf axis="self"/>')
    root = ('<dgm:layoutNode name="matrix"><dgm:varLst><dgm:resizeHandles val="exact"/></dgm:varLst><dgm:alg type="composite"/><dgm:shape/>'
            "<dgm:constrLst>" + box("grid", 1.0, 1.0, l=0.0, t=0.0) + box("title", 0.36, 0.3, cx=0.5, cy=0.5)
            + FONTD("quad") + c("primFontSz", "ch", "title", op="equ", val=65) + "</dgm:constrLst>"
            '<dgm:forEach name="theTitle" axis="ch" ptType="node" cnt="1">' + grid + title + "</dgm:forEach></dgm:layoutNode>")
    return layout("matrix1", "Titled Matrix",
                  "the first item a title in the middle, the four under it the quadrants round it.", root)


def target1():
    N = 5
    def ring(k):
        return node(f"ring{k}", "node1", "ellipse", tx=p("txAnchorVert", "t"), constrs=None)
    def sizes(n):
        out = ""
        band = 0.98 / (2 * n)
        for k in range(1, N + 1):
            if k > n:
                continue
            d = 0.98 * (1 - (k - 1) / n)
            out += box(f"ring{k}", d, d, cx=0.5, cy=0.5, wref="h", href="h", xref="w", yref="h")
            # the text in the ring's own band, above the next ring
            out += c("tMarg", "ch", f"ring{k}", "h", band * 0.12)
            out += c("bMarg", "ch", f"ring{k}", "h", 0.0 if k == n else d - band)
            side = band * 1.2 if k < n else d * 0.15
            out += c("lMarg", "ch", f"ring{k}", "h", side) + c("rMarg", "ch", f"ring{k}", "h", side)
        return out
    root = ('<dgm:layoutNode name="target"><dgm:varLst><dgm:resizeHandles val="exact"/></dgm:varLst><dgm:alg type="composite"/><dgm:shape/>'
            + by_cnt(N, sizes)
            # one size for every ring: one equ over them all
            + "<dgm:constrLst>" + c("primFontSz", "ch", op="equ", val=65) + "</dgm:constrLst>"
            + by_pos(N, lambda k: node(f"ring{k}", "node1", "ellipse", tx=p("txAnchorVert", "t"), constrs=""))
            + "</dgm:layoutNode>")
    return layout("target1", "Basic Target", "the items as rings round one centre, the first the outermost (five at most).", root)


def funnel1():
    mouth = ('<dgm:layoutNode name="mouth"><dgm:alg type="snake">' + p("off", "ctr") + "</dgm:alg><dgm:shape/><dgm:constrLst>"
             + c("w", "ch", "item", "h", 0.9) + c("h", "ch", "item", "h", 0.9) + c("sibSp", ref="w", fact=0.02)
             + '</dgm:constrLst><dgm:forEach name="poured" axis="ch" ptType="node"><dgm:choose name="notLast">'
             '<dgm:if name="poured" func="revPos" op="gt" val="1">'
             + node("item", "node1", "ellipse", constrs=margins("w", 0.12))
             + "</dgm:if><dgm:else name=\"last\"/></dgm:choose></dgm:forEach></dgm:layoutNode>")
    root = ('<dgm:layoutNode name="funnel"><dgm:varLst><dgm:resizeHandles val="exact"/></dgm:varLst><dgm:alg type="composite"/><dgm:shape/>'
            "<dgm:constrLst>" + box("shape", 0.5, 0.62, cx=0.5, t=0.0) + box("mouth", 0.38, 0.22, cx=0.5, t=0.06)
            + box("arrow", 0.05, 0.1, cx=0.5, t=0.65) + box("result", 0.36, 0.2, cx=0.5, t=0.78)
            + FONTD("item") + c("primFontSz", "ch", "result", op="equ", val=65) + "</dgm:constrLst>"
            + deco("shape", "alignAcc1", "funnel") + mouth + deco("arrow", "sibTrans2D1", "downArrow")
            + '<dgm:forEach name="out" axis="ch" ptType="node"><dgm:choose name="isLast">'
            '<dgm:if name="last" func="revPos" op="equ" val="1">'
            + node("result", "node1", "roundRect", presof='<dgm:presOf axis="desOrSelf" ptType="node"/>')
            + "</dgm:if><dgm:else name=\"others\"/></dgm:choose></dgm:forEach></dgm:layoutNode>")
    return layout("funnel1", "Funnel", "the items poured into a funnel, the last what comes out of it.", root)


def gear1():
    # in a square, its side the frame's height: three gears that mesh
    # centres a little closer than the radii add up to, so the teeth mesh
    spots = {1: (0.56, 0.32, 0.56, "gear9"), 2: (0.40, 0.61, 0.27, "gear6"), 3: (0.32, 0.66, 0.72, "gear6")}
    inner = "<dgm:constrLst>"
    for k, (d, cx, cy, _) in spots.items():
        inner += box(f"gear{k}", d, d, cx=cx, cy=cy)
    inner += c("primFontSz", "ch", op="equ", val=65) + "</dgm:constrLst>"
    gears = ('<dgm:layoutNode name="gears"><dgm:alg type="composite"/><dgm:shape/>' + inner
             + by_pos(3, lambda k: node(f"gear{k}", "node1", spots[k][3], constrs=margins("w", 0.24)))
             + "</dgm:layoutNode>")
    root = ('<dgm:layoutNode name="gear"><dgm:varLst><dgm:resizeHandles val="exact"/></dgm:varLst><dgm:alg type="composite"/><dgm:shape/>'
            "<dgm:constrLst>" + box("gears", 1.0, 1.0, cx=0.5, cy=0.5, wref="h", href="h") + "</dgm:constrLst>"
            + gears + "</dgm:layoutNode>")
    return layout("gear1", "Gear", "three items as meshing gears (three at most).", root)


def arrow2():
    N = 5
    def spots(n):
        out = ""
        for k in range(1, n + 1):
            t = (k - 0.5) / n
            x = 0.14 + 0.66 * t
            y = 0.86 - 0.70 * (t ** 1.3)
            out += box(f"dot{k}", 0.05, 0.05, cx=x, cy=y, wref="w", href="w")
            out += box(f"label{k}", 0.2, 0.16, l=x + 0.025, t=max(0.0, y - 0.17))
        return out
    def item(k):
        return (deco(f"dot{k}", "node1", "ellipse")
                + node(f"label{k}", "revTx", "none", presof='<dgm:presOf axis="desOrSelf" ptType="node"/>',
                       tx=p("txAnchorVert", "b") + p("parTxLTRAlign", "l"), constrs=margins("primFontSz", 0.1)))
    fonts = c("primFontSz", "ch", op="equ", val=65)
    root = ('<dgm:layoutNode name="arrow"><dgm:varLst><dgm:resizeHandles val="exact"/></dgm:varLst><dgm:alg type="composite"/><dgm:shape/>'
            "<dgm:constrLst>" + box("swoosh", 1.0, 1.0, l=0.0, t=0.0) + fonts + "</dgm:constrLst>"
            + by_cnt(N, spots) + deco("swoosh", "alignAcc1", "swooshArrow")
            + by_pos(N, item) + "</dgm:layoutNode>")
    return layout("arrow2", "Upward Arrow", "the items as points rising along a sweeping arrow (five at most).", root)


def bList2():
    cell = ('<dgm:layoutNode name="cell"><dgm:alg type="composite"/><dgm:shape/><dgm:constrLst>'
            + box("text", 1.0, 0.72, l=0.0, t=0.28) + box("accent", 0.42, 0.42, l=0.03, t=0.0, wref="h", href="h", xref="w", yref="h")
            + "</dgm:constrLst>"
            + node("text", "node1", "roundRect", constrs=c("tMarg", ref="h", fact=0.22) + c("bMarg", ref="primFontSz", fact=0.3)
                   + c("lMarg", ref="primFontSz", fact=0.3) + c("rMarg", ref="primFontSz", fact=0.3))
            + deco("accent", "fgAcc1", "ellipse") + "</dgm:layoutNode>")
    root = ('<dgm:layoutNode name="list"><dgm:varLst><dgm:dir/><dgm:resizeHandles val="exact"/></dgm:varLst>'
            '<dgm:alg type="snake">' + p("grDir", "tL") + p("flowDir", "row") + p("contDir", "sameDir") + p("off", "ctr") + "</dgm:alg><dgm:shape/>"
            "<dgm:constrLst>" + c("w", "ch", "cell", "w") + c("h", "ch", "cell", "w", refFor="ch", refName="cell", fact=0.62)
            + c("sibSp", ref="w", refFor="ch", refName="cell", fact=0.08) + c("secSibSp", ref="w", refFor="ch", refName="cell", fact=0.06)
            + FONTD("text") + "</dgm:constrLst>"
            '<dgm:forEach name="items" axis="ch" ptType="node">' + cell + "</dgm:forEach></dgm:layoutNode>")
    return layout("bList2", "Bending Picture Accent List",
                  "the items as blocks in rows that wrap, each with a round accent where its picture goes (drawn without the picture).", root)


def hProcess9():
    row = ('<dgm:layoutNode name="row"><dgm:alg type="lin"/><dgm:shape/><dgm:constrLst>'
           + c("w", "ch", "node", "w", 0.3) + c("h", "ch", "node", "h")
           + c("sibSp", ref="w", refFor="ch", refName="node", fact=0.12) + FONT("node")
           + '</dgm:constrLst><dgm:forEach name="items" axis="ch" ptType="node">'
           + node("node", "node1", "roundRect") + "</dgm:forEach></dgm:layoutNode>")
    root = ('<dgm:layoutNode name="process"><dgm:varLst><dgm:dir/><dgm:resizeHandles val="exact"/></dgm:varLst><dgm:alg type="composite"/><dgm:shape/>'
            "<dgm:constrLst>" + box("arrow", 0.9, 0.84, cx=0.5, cy=0.5) + box("row", 1.0, 0.38, cx=0.5, cy=0.5) + "</dgm:constrLst>"
            + deco("arrow", "bgShp", "rightArrow") + row + "</dgm:layoutNode>")
    return layout("hProcess9", "Continuous Block Process", "the steps as blocks along one wide arrow.", root)


def lProcess2():
    kids = ('<dgm:layoutNode name="kids"><dgm:alg type="lin">' + p("linDir", "fromT") + p("nodeVertAlign", "t") + "</dgm:alg><dgm:shape/><dgm:constrLst>"
            + c("w", "ch", "child", "w") + c("h", "ch", "child", "w", refFor="ch", refName="child", fact=0.42)
            + c("sibSp", ref="w", refFor="ch", refName="child", fact=0.06)
            + '</dgm:constrLst><dgm:forEach name="under" axis="ch" ptType="node">'
            + node("child", "node1", "roundRect", presof='<dgm:presOf axis="desOrSelf" ptType="node"/>')
            + "</dgm:forEach></dgm:layoutNode>")
    group = ('<dgm:layoutNode name="group"><dgm:alg type="composite"/><dgm:shape/><dgm:constrLst>'
             + box("bg", 1.0, 1.0, l=0.0, t=0.0) + box("header", 1.0, 0.18, l=0.0, t=0.0) + box("kids", 0.86, 0.78, cx=0.5, t=0.2)
             + "</dgm:constrLst>"
             + deco("bg", "bgShp", "roundRect")
             + node("header", "revTx", "none", presof='<dgm:presOf axis="self"/>', tx=p("txAnchorVert", "mid"))
             + kids + "</dgm:layoutNode>")
    root = ('<dgm:layoutNode name="lists"><dgm:varLst><dgm:dir/><dgm:resizeHandles val="exact"/></dgm:varLst><dgm:alg type="lin"/><dgm:shape/>'
            "<dgm:constrLst>" + c("w", "ch", "group", "w", 0.32) + c("h", "ch", "group", "h")
            + c("sibSp", ref="w", refFor="ch", refName="group", fact=0.08)
            + FONTD("header") + FONTD("child") + "</dgm:constrLst>"
            '<dgm:forEach name="groups" axis="ch" ptType="node">' + group + "</dgm:forEach></dgm:layoutNode>")
    return layout("lProcess2", "Grouped List", "each item a column, its heading on top and the items under it as blocks.", root)


def cycle4():
    rots = {1: 0, 2: 90, 3: 270, 4: 180}
    wedges = ('<dgm:layoutNode name="wedges"><dgm:alg type="snake">' + p("bkpt", "fixed") + p("bkPtFixedVal", 2) + "</dgm:alg><dgm:shape/><dgm:constrLst>"
              + c("w", "ch", "wedge", "w", 0.5) + c("h", "ch", "wedge", "h", 0.5) + c("sibSp", ref="w", fact=0.02)
              + "</dgm:constrLst>"
              + by_pos(4, lambda k: deco("wedge", "node1", "pieWedge", rot=rots[k])).replace('name="items"', 'name="shapes"')
              + "</dgm:layoutNode>")
    aligns = {1: ("t", "l"), 2: ("t", "r"), 3: ("b", "l"), 4: ("b", "r")}
    labels = ('<dgm:layoutNode name="labels"><dgm:alg type="snake">' + p("bkpt", "fixed") + p("bkPtFixedVal", 2) + "</dgm:alg><dgm:shape/><dgm:constrLst>"
              + c("w", "ch", "label", "w", 0.5) + c("h", "ch", "label", "h", 0.5) + c("sibSp", ref="w", fact=0.02)
              + "</dgm:constrLst>"
              + by_pos(4, lambda k: node("label", "node1", "none", tx=p("txAnchorVert", aligns[k][0]) + p("parTxLTRAlign", aligns[k][1]),
                                       constrs=margins("w", 0.12))).replace('name="items"', 'name="texts"')
              + "</dgm:layoutNode>")
    core = ('<dgm:layoutNode name="core"><dgm:alg type="composite"/><dgm:shape/><dgm:constrLst>'
            + box("wedges", 1.0, 1.0, l=0.0, t=0.0) + box("labels", 1.0, 1.0, l=0.0, t=0.0) + FONTD("label")
            + "</dgm:constrLst>" + wedges + labels + "</dgm:layoutNode>")
    root = ('<dgm:layoutNode name="cycle"><dgm:varLst><dgm:resizeHandles val="exact"/></dgm:varLst><dgm:alg type="composite"/><dgm:shape/>'
            "<dgm:constrLst>" + box("core", 1.0, 1.0, cx=0.5, cy=0.5, wref="h", href="h") + "</dgm:constrLst>"
            + core + "</dgm:layoutNode>")
    return layout("cycle4", "Cycle Matrix", "four items as the quarters of one circle (four at most).", root)


def main():
    for f in (venn1, matrix1, target1, funnel1, gear1, arrow2, bList2, hProcess9, lProcess2, cycle4):
        xml = f()
        with open(os.path.join(OUT, f.__name__ + ".xml"), "w", encoding="utf-8") as fh:
            fh.write(xml)
        print("wrote builtin/" + f.__name__ + ".xml")


if __name__ == "__main__":
    main()
