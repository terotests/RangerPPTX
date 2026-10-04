# PLAN_SMARTART — a SmartArt layout engine

Status: plan. Nothing below the "What exists" section is built yet.

## Why

A SmartArt diagram in a `.pptx` is a graphic frame pointing at four or five
parts: the **data model** (the text, as a tree), the **layout definition**
(how to turn that tree into shapes), a **quick style**, a **colour
definition**, and — only when PowerPoint wrote the file — a **drawing** part
holding the shapes PowerPoint computed last time it laid the diagram out.

A file written by anything other than PowerPoint, and that includes an LLM,
usually has the data model and a layout reference and no drawing. Without a
layout engine such a diagram cannot be drawn at all. So the engine is the
feature, not an optional extra.

The authoring surface stays as cheap as the rest of Sliqtly: a Markdown fence
whose body is a list. It is easy to write, easy to diff and easy to index; the
expensive part (the layout language and its algorithms) is ours to carry, once,
behind it.

````markdown
```smartart
layout: process
- Plan
  - goals, budget
- Build
- Ship
```
{colors=colorful1 style=simple3}
````

The same engine also takes SmartArt XML: a data model (with or without its
layout definition) passed to the MCP server as a file and named in the fence.

## What exists (verified 2026-10-04)

| Piece | Where | State |
| --- | --- | --- |
| SmartArt graphic frame survives a save | `src/PptxParser.rgr` `parseSpTreeInto` → `PptxOpaque`; `src/PptxWriter.rgr` `opaqueText` | Kept as a source span and copied back byte for byte; the `ppt/diagrams/*` parts are copied through when the opened package is saved. Not drawn, not editable. |
| …and survives undo | `src/PptxEdit.rgr` | Opaque children travel with the slide snapshot. |
| Fixture and checks | `fixtures/32-unmodelled-content.pptx` (`tools/make_fixtures.py`), `tests/PptxWriterTest.rgr`, `web/playground/smoke.mjs` | The relIds frame and empty data/layout/style/colour parts; asserts nothing is lost. |
| `graphicFrame` dispatch | `PptxParser.parseGraphicFrame` | Tables and charts are recognised; a diagram URI falls through to opaque. The hook for SmartArt goes here. |
| The 187 preset geometries | Ranger `gallery/office/geom/OfficePresetShapes.rgr` + `assets/presets.txt` | Guide-and-path evaluator. A layout node's `<dgm:shape type="chevron">` is one of these. |
| Text measuring and shrink | Ranger `gallery/office/text/OfficeTextMeasure.rgr`; `normAutofit`/`fontScale` in the parser | What the text-fit rules need. |
| Theme resolution, painting, export | `PptxResolver`, `PptxToEvg`, `PptxFromEvg`, `PptxWriter` | A diagram that comes out as `PptxShape`s is drawn, exported to PDF and written like any shape. |
| List-shaped slide layouts | RangerMarkdown `src/MdFigure.rgr` (`process`, `swot`, `timeline`) | The fence family and the `FlowScene` → stage / PDF / PPTX path a `smartart` fence joins. |
| Tree layouts | RangerFlow `layout/TreeLayouts.rgr` | Reference only: SmartArt's hierarchy algorithms have their own, specified, parameters. |

Missing: the data model reader and writer, the drawing-part reader and writer,
the layout definition interpreter (iteration, conditions, presentation points),
the constraint and rule solver, the ten algorithms, text fitting across
siblings, quick style and colour application, and the built-in layouts
themselves.

## Where it lives

Its own repository, `terotests/RangerSmartArt`, cloned to `gallery/smartart`
in a Ranger checkout the way RangerPPTX is cloned to `gallery/pptx`. It
depends on `gallery/xml`, `gallery/office` and `lib/evg` only, never on
`gallery/pptx`, and its output is neutral:

```text
SaShape   preset name + adjust values (or a custom path), box, rotation,
          flip, text (paragraphs, runs, level), font size, style refs
          (lnRef/fillRef/effectRef/fontRef), resolved colours, z-order,
          the data point it presents
```

RangerPPTX turns `SaShape`s into `PptxShape`s and writes the parts;
RangerMarkdown turns them into a `FlowScene`. The engine has to compile to
**Go** as well as JavaScript and C++: Sliqtly's MCP server is Ranger compiled
to Go and lays decks out with the editor's own model.

## The pipeline

```text
fence (text tree) ──┐
                    ├─→ DataModel ─→ presentation tree ─→ constraints ─→ algorithms
dataModel XML ──────┘   (pt, cxn)    (layoutDef walk:      (constrLst,     (lin, snake,
                                      forEach, choose/if,   refs, for,      cycle, hierRoot,
                                      presOf, presParOf)    ptType, op)     hierChild, pyra,
                                                                            tx, conn, sp,
                                                                            composite)
        ─→ rules (ruleLst: shrink until it fits) ─→ text fit (shared primFontSz)
        ─→ shapes (preset geometry) ─→ quick style + colours (method cycle/repeat/span)
        ─→ [SaShape]  ─→ PptxShape / FlowScene / dsp drawing + dataModel parts
```

1. **Data model.** Points `doc`, `node`, `asst`, `parTrans`, `sibTrans`,
   `pres`; connections `parOf`, `presOf`, `presParOf` with `srcOrd`/`destOrd`.
   Read what a file has, generate what an LLM left out (transitions, model ids).
2. **Presentation tree.** Walk the layout definition against the data:
   `layoutNode`, `forEach` (axis `self ch des desOrSelf par ancst
   ancstOrSelf followSib precedSib follow preced root`, `ptType`, `st`,
   `cnt`, `step`, `hideLastTrans`), `choose`/`if`/`else` (functions `cnt pos
   revPos posEven posOdd var depth maxDepth`, operators, variables `dir
   hierBranch orgChart chMax chPref bulletEnabled animOne animLvl
   resizeHandles`), `presOf`, `alg`, `shape`, `varLst`.
3. **Constraints.** Each layout node's `constrLst`: type (position, size,
   spacing, font size, padding, connector distance…), `for`/`forName`,
   `ptType`, `refType`/`refFor`/`refForName`/`refPtType`, `op`
   (`equ gte lte none`), `fact`, `val`. Resolved in dependency order; a cycle
   is an error with the chain named.
4. **Algorithms**, each with its parameters:
   `lin` (linDir, secLinDir, nodeHorzAlign, nodeVertAlign, fallback),
   `snake` (grDir, flowDir, contDir, bkpt, bkPtFixedVal, off),
   `cycle` (stAng, spanAng, ctrShpMap, rotPath),
   `hierRoot` / `hierChild` (hierAlign, linDir, secLinDir, chAlign, chDir,
   secChAlign), `pyra` (linDir, txDir, pyraAcctPos, pyraAcctRatio,
   pyraAcctBkgdNode, pyraAcctTxNode, pyraLvlNode),
   `tx` (txAnchorVert, txAnchorHorz, parTxLTRAlign, parTxRTLAlign,
   stBulletLvl, autoTxRot, txBlDir), `conn` (begPts, endPts, connRout, dim,
   begSty, endSty, srcNode, dstNode), `sp`, `composite`.
5. **Rules.** `ruleLst`: reduce a value by `fact` down to `max`/`val` until
   the content fits, in order.
6. **Text.** Measured with Office text metrics; siblings that share
   `primFontSz` get one size, as PowerPoint does.
7. **Style.** Quick style (`styleLbl` → line/fill/effect/font refs, 3-D
   ignored) and colour definition (`fillClrLst` etc. with `meth` cycle,
   repeat, span, `hueDir`) resolved against the slide's theme.

## Built-in layouts

A file names its layout by `uniqueId`
(`urn:microsoft.com/office/officeart/2005/8/layout/process1`). PowerPoint
writes the full definition into every file it saves; an LLM usually does not.
So the engine ships definitions for the layouts people use, **written by us in
the layoutDef language** from the specification and from what PowerPoint
draws, not copied from Office. Order of work:

1. `default` (vertical list; PowerPoint's own fallback), `vList2`, `hList1`,
   `process1`, `chevron1`
2. `cycle2`, `radial1`, `hierarchy1`, `orgChart1`, `pyramid1`
3. `venn1`, `matrix1`, `target1`, `funnel1`, `gear1`, `arrow2`, `bList2`,
   `hProcess9`, `lProcess2`, `cycle4`

A layout id the engine does not have: draw the file's drawing part if there is
one; otherwise lay the data out with `default` and say so (the slot and the
MCP warnings name the layout it did not know).

Friendly names for the fence (`process` → `process1`, `cycle` → `cycle2`,
`org` → `orgChart1`, `pyramid` → `pyramid1`, `list` → `vList2`…) are a table
in one file, so the guide and the engine cannot disagree.

## Tests

Every layer gets its own suite, run on JavaScript, C++ and Go. The counts are
the floor, not the target.

| Suite | What it pins | At least |
| --- | --- | --- |
| DataModel | read/write round trip of every point and connection type, `srcOrd` order, generated transitions and ids, malformed input refused with the reason | 40 |
| Fence | list → data model: levels, empty items, continuation lines, options, unknown layout names | 30 |
| Iteration | each `forEach` axis × `ptType` × `st`/`cnt`/`step`; `hideLastTrans` | 50 |
| Conditions | each `if` function and operator, nested `choose`, variables | 40 |
| Constraints | each constraint type, every reference form, `op` and `fact`, chains, cycles reported | 70 |
| Rules | shrink order, limits, a rule that cannot be met | 20 |
| Algorithms | each algorithm × its parameters, 1–10 children: positions, sizes, spacing, angles, as numbers | 120 |
| Text fit | shared font size, wrap, minimum size, `normAutofit` in the output | 25 |
| Style | quick style refs, colour methods over node index, theme colours | 30 |
| Layouts | each built-in layout with 1, 2, 3, 5, 8 nodes and 1–3 levels against the oracle (below), box by box within a tolerance | 20 × 5+ |
| Drawing part | read PowerPoint's drawing → shapes; write ours → PowerPoint opens it unchanged | 25 |
| Integration | RangerPPTX opens, draws, edits and saves a SmartArt deck; Sliqtly fence → stage, PDF, PPTX; MCP create with a fence and with XML | 30 |

**The oracle.** PowerPoint's drawing part is PowerPoint's own answer for the
data next to it. A corpus of decks saved by PowerPoint — every layout above,
with the node counts and depths in the table — gives every layout test its
expected boxes without anyone typing a number. Producing it needs PowerPoint
once (a macro that inserts each layout, fills it and saves); the files are then
fixtures. The same reader is phase 0's renderer.

## Phases

0. **Repository, data model, drawing reader.** RangerPPTX recognises the
   diagram frame and draws a PowerPoint-saved SmartArt from its drawing part
   instead of skipping it. Oracle harness and corpus in place.
1. **Interpreter core.** Presentation tree, constraints, rules, `lin`, `sp`,
   `composite`, `tx`, text fit, style. Layouts of group 1. The `smartart`
   fence in RangerMarkdown; `files` in the MCP server accepts `.xml`; the
   guide documents both; `Check.rgr` warns about unknown layouts and text
   that was shrunk below a readable size.
2. **Remaining algorithms.** `snake`, `cycle`, `hierRoot`/`hierChild`,
   `pyra`, `conn`. Layouts of group 2.
3. **Writing SmartArt.** PPTX export writes data model, layout definition,
   style, colours and our drawing part, so PowerPoint opens it as editable
   SmartArt; RangerPPTX edits a diagram's text and lays it out again.
4. **Breadth.** Layout group 3, more quick styles and colour sets.

## Open questions

1. Repository name and whether it is private, like RangerPPTX.
2. Who can run PowerPoint once to produce the oracle corpus.
3. Whether a file that names a built-in layout without carrying its
   definition opens as SmartArt in PowerPoint, or whether phase 3 must write a
   full definition — decides how much of each layout we write out.
4. Whether `smartart` is a PRO layout like `process`/`swot`/`timeline`.
