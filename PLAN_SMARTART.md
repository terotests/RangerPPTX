# PLAN_SMARTART — a SmartArt layout engine

Status (2026-10-04): phases 0 and 1 done. Phase 1's layouts are the four
list and process layouts; `default` moved to phase 2 with the `snake`
algorithm it needs. Phases 2–4 not started. The engine lives in this repository
under `smartart/`; terotests/RangerSmartArt is not used.

## Why

A SmartArt diagram is a data model (the text, as a tree, plus the ids of a
layout, a quick style and a colour set) and, when PowerPoint wrote it, a
drawing part with the shapes PowerPoint computed last time. A diagram written
by anything else — an LLM included — usually has the data model and no
drawing, and without a layout engine it cannot be drawn at all. So the engine
is the feature.

In Sliqtly a SmartArt is **a file in the deck**, referenced from the Markdown
like a picture and placed with the same attributes:

```markdown
![The release process](media/release.xml)
{width=80% layout=process1 colors=colorful1}
```

(Block attributes go on their own line under the block — the Markdown
module's `{…}` syntax, as for lists and fences.)

The Markdown module does not know what SmartArt is. It lays the reference out
as a picture box; Sliqtly draws the diagram into that box.

## What exists (verified 2026-10-04)

| Piece | Where | State |
| --- | --- | --- |
| SmartArt survives a save and undo | `src/PptxParser.rgr` `parseSpTreeInto` → `PptxOpaque`; `src/PptxWriter.rgr` `opaqueText`; `src/PptxEdit.rgr` | Kept as a source span and copied back byte for byte; `ppt/diagrams/*` parts copied through on save. Not drawn, not editable. |
| Fixture and checks | `fixtures/32-unmodelled-content.pptx` (`tools/make_fixtures.py`), `tests/PptxWriterTest.rgr`, `web/playground/smoke.mjs` | Asserts nothing is lost. |
| `graphicFrame` dispatch | `PptxParser.parseGraphicFrame` | Tables and charts recognised; a diagram URI falls through to opaque. |
| 187 preset geometries | Ranger `gallery/office/geom/OfficePresetShapes.rgr` | What a layout node's `<dgm:shape type="…">` names. |
| Text measuring, shrink | Ranger `gallery/office/text/OfficeTextMeasure.rgr`; `normAutofit` in the parser | For the text-fit rules. |
| Display list → PPTX shapes | `src/PptxFromEvg.rgr` | Phase 1 export path. |
| A picture box in the Markdown layout | RangerMarkdown `MdLayout.pictureBlock` | Box size from the store's `VfsStat.pixelW/H` (2:1 of the column when 0); `{width}`, `{height}`, `{align}`, theme `img { max-height }`; emits an `MdBox` of kind 4 with `imagePath`. |
| Pictures into the store | `PresApp.addImage` (web) and `mcp-go/rgr/Check.rgr` (MCP) both call `deck.md.addImage(name bytes type w h)` | The one door a SmartArt file comes in by. |
| A picture on the slide | `PresDeck.slideList` | An `EVGDrawCmd` of kind 2 with `src` = the path. |
| A picture in the PPTX | RangerMarkdown `MdToPptx` (kind 4 → picture shape with `imagePart`) | Sliqtly already replaces shapes after it: `PresApp.stillDiagrams`, `stillCharts`, `slideArt`. |
| MCP picture types | `mcp-go/rgr/Deck.rgr` (png, jpeg, gif, webp, svg) | Gets one more. |

## Format: one XML file

The root is `dgm:dataModel`, the same element as `ppt/diagrams/data1.xml`. The
document point's `prSet` names the layout, quick style and colours, so the
file describes itself; Markdown attributes override them. The smallest file
the engine accepts:

```xml
<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"
               xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <dgm:ptLst>
    <dgm:pt modelId="0" type="doc">
      <dgm:prSet loTypeId="urn:microsoft.com/office/officeart/2005/8/layout/process1"/>
    </dgm:pt>
    <dgm:pt modelId="1"><dgm:t><a:p><a:r><a:t>Plan</a:t></a:r></a:p></dgm:t></dgm:pt>
    <dgm:pt modelId="2"><dgm:t><a:p><a:r><a:t>Build</a:t></a:r></a:p></dgm:t></dgm:pt>
    <dgm:pt modelId="3"><dgm:t><a:p><a:r><a:t>Ship</a:t></a:r></a:p></dgm:t></dgm:pt>
  </dgm:ptLst>
  <dgm:cxnLst>
    <dgm:cxn modelId="10" srcId="0" destId="1" srcOrd="0"/>
    <dgm:cxn modelId="11" srcId="0" destId="2" srcOrd="1"/>
    <dgm:cxn modelId="12" srcId="0" destId="3" srcOrd="2"/>
  </dgm:cxnLst>
</dgm:dataModel>
```

Point type defaults to `node` and connection type to `parOf`, as in the
schema. Model ids may be any strings. `bodyPr`, transition points,
presentation points and `presOf`/`presParOf` connections may be left out and
are generated; a data model PowerPoint saved, which has all of them, is read
as it is. Anything the engine cannot read is refused with the element and the
reason, and the MCP server returns that to the caller.

Later (phase 4), the same reference may name a file holding a whole
PowerPoint diagram (data, layout definition, style, colours, drawing).

## Where the code lives

```text
RangerPPTX/
  smartart/
    SaModel.rgr          points, connections, the document
    SaDataReader.rgr     dataModel XML → SaModel (lenient, errors with element)
    SaDataWriter.rgr     SaModel → dataModel XML (complete: transitions, pres points)
    SaLayoutDef.rgr      the layout definition as data: layoutNode, forEach, choose/if,
                         alg, shape, presOf, constrLst, ruleLst, varLst
    SaLayoutReader.rgr   layoutDef XML → SaLayoutDef
    SaPresTree.rgr       data × definition → presentation tree (iteration, conditions,
                         generated pres points)
    SaConstraints.rgr    constraint resolution
    SaRules.rgr          shrink-until-it-fits
    alg/SaLin.rgr  alg/SaSnake.rgr  alg/SaCycle.rgr  alg/SaHier.rgr  alg/SaPyra.rgr
    alg/SaTx.rgr   alg/SaConn.rgr   alg/SaComposite.rgr   (sp is a no-op)
    SaText.rgr           text fit, shared font size (OfficeTextMeasure)
    SaStyle.rgr          quick styles and colour sets, resolved against a theme
    SaEngine.rgr         the facade: layout(xml, overrides, w, h, theme) → a PptxShape
                         group (isDiagram) plus warnings and a preferred aspect
    SaDrawing.rgr        drawing part written (phase 3); it is READ by PptxParser
    builtin/*.xml        our own layout definitions, written in the layoutDef language
    SaBuiltins.rgr       generated from builtin/ by tools/embed_layouts.mjs — no file
                         I/O at run time, so the Go MCP binary and the browser get them
    tests/               the suites below
    oracle/              PowerPoint-saved decks and what they are checked against
    tools/run_tests.sh   every suite on JavaScript, C++ and Go
```

Constraints on the code:

- Its output is a `PptxShape` group (`isDiagram`, children in the frame's own
  space), the same thing `PptxParser` makes from PowerPoint's drawing part.
  So one representation is drawn (`PptxToEvg`), exported (`PptxWriter`) and
  edited, whether PowerPoint or the engine laid the diagram out; there is no
  second shape type to convert. The data-model layer (`SaModel`,
  `SaDataReader`, `SaDataWriter`) imports `gallery/xml/XmlCore` only. `XmlCore`, not
  `XmlLite`: the MCP build already cannot put both in one compile.
- It compiles to Go, C++ and JavaScript. Sliqtly's MCP server is the editor's
  deck model compiled to Go, and it must lay out, check and render SmartArt
  exactly as the editor does.
- Every function is a function of its input: no clock, no random ids (ids are
  derived from model ids), so a layout is the same on every target and every
  run.

## The engine

```text
dataModel XML ─→ SaModel ─→ presentation tree ─→ constraints ─→ algorithms ─→ rules
                 (+ generated   (layoutDef walk)                               │
                  transitions)                                                 ↓
                         [SaShape] ←─ style + colours ←─ shapes ←─ text fit ←──┘
```

1. **Data model.** Points `doc node asst parTrans sibTrans pres`; connections
   `parOf presOf presParOf`, ordered by `srcOrd`.
2. **Presentation tree.** `layoutNode`, `forEach` (axis `self ch des
   desOrSelf par ancst ancstOrSelf followSib precedSib follow preced root`,
   `ptType`, `st`, `cnt`, `step`, `hideLastTrans`), `choose`/`if`/`else`
   (`cnt pos revPos posEven posOdd var depth maxDepth`; operators; variables
   `dir hierBranch orgChart chMax chPref bulletEnabled animOne animLvl
   resizeHandles`), `presOf`, `alg`, `shape`, `varLst`.
3. **Constraints.** Type, `for`/`forName`, `ptType`, `refType`/`refFor`/
   `refForName`/`refPtType`, `op` (`equ gte lte none`), `fact`, `val`;
   resolved in dependency order; a cycle is an error naming the chain.
4. **Algorithms** with their parameters: `lin` (linDir, secLinDir,
   nodeHorzAlign, nodeVertAlign, fallback), `snake` (grDir, flowDir, contDir,
   bkpt, bkPtFixedVal, off), `cycle` (stAng, spanAng, ctrShpMap, rotPath),
   `hierRoot`/`hierChild` (hierAlign, linDir, secLinDir, chAlign, chDir,
   secChAlign), `pyra` (linDir, txDir, pyraAcctPos, pyraAcctRatio,
   pyraAcctBkgdNode, pyraAcctTxNode, pyraLvlNode), `tx` (txAnchorVert,
   txAnchorHorz, parTxLTRAlign, parTxRTLAlign, stBulletLvl, autoTxRot,
   txBlDir), `conn` (begPts, endPts, connRout, dim, begSty, endSty, srcNode,
   dstNode), `sp`, `composite`.
5. **Rules.** `ruleLst`: reduce by `fact` towards `max`/`val`, in order, until
   it fits.
6. **Text.** Office text metrics; siblings sharing `primFontSz` get one size.
7. **Style.** Quick style labels → line/fill/effect/font refs (3-D ignored);
   colour lists with `meth` cycle/repeat/span and `hueDir`, against the
   slide's theme colours.

### Built-in layouts

A file names its layout by `uniqueId`. We write the definitions ourselves in
the layoutDef language, from the specification and from what PowerPoint draws
(the oracle), not copied from Office. In order:

1. `vList2`, `hList1`, `process1`, `chevron1` (built)
2. `default` (needs `snake`), `cycle2`, `radial1`, `hierarchy1`,
   `orgChart1`, `pyramid1`
3. `venn1`, `matrix1`, `target1`, `funnel1`, `gear1`, `arrow2`, `bList2`,
   `hProcess9`, `lProcess2`, `cycle4`

The same goes for quick styles (`simple1`–`simple5`, `subtle…`) and colour
sets (`accent1_2`, `colorful1`…): data in `SaStyle`. Short names for the
Markdown attributes (`layout=process1`, `colors=colorful1`, `style=simple3`)
are the uniqueId's last segment, so there is no second table to keep in step.

An unknown layout id: the file's drawing part when there is one (a deck
PowerPoint saved), otherwise `vList2` until `default` exists, with a warning
naming the id.

## Sliqtly

All of it in Ranger code (`src/`), so the editor and the MCP server share it.

1. **In.** `PresDeck.addImage(name bytes type w h)` takes over from the two
   direct `deck.md.addImage` calls (`PresApp.addImage`, `mcp-go/rgr/Check.rgr`).
   A SmartArt file (content type
   `application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml`,
   or `.xml` whose root is `dgm:dataModel`) is parsed once and cached; it is
   stored in the Markdown's picture store with a pixel size made from the
   engine's preferred aspect, so `MdLayout.pictureBlock` gives it a box and
   `{width}`, `{height}`, `{align}` work unchanged.
2. **Overrides.** `layout=`, `colors=`, `style=` are read from the image
   node's attributes, found by the box's `srcStart` (the key `MdToPptx`
   already uses to find a picture's box).
3. **On the slide.** `PresDeck.slideList` replaces a kind-2 command whose
   `src` is a SmartArt with the engine's group drawn by `PptxToEvg` into that box, laid out at the
   box's size with the slide's theme. Cached by path, size, overrides and
   theme. Tagged `smartart:<path>:<modelId>` per shape, for the layout report
   and a later node-by-node reveal.
4. **PPTX.** `PresApp.stillSmartArt`, beside `stillDiagrams`, replaces the
   picture shape whose `imagePart` is the SmartArt with the engine's group
   (phase 1) or a native SmartArt frame and its parts (phase 3). The XML must
   never reach the deck as a picture part.
5. **Web host.** `web/picture.js` / `web/main.js`: a SmartArt file is passed
   through as bytes with size 0 instead of being decoded as a picture.
6. **MCP.** `Deck.rgr` accepts the type and `.xml`; the file is parsed at
   create/update and a refusal is returned with the reason; `Check.rgr` warns
   about an unknown layout, text shrunk below a readable size, and more nodes
   than the layout holds; `Report.rgr` lists a SmartArt as an element with its
   labels; `render_slide` draws it (vector commands, unlike SVG); `Tools.rgr`
   names the type; `assets/guide.md` gets the minimal file above and the
   attributes.

7. **PDF.** `PresApp.pdf` draws each page from `deck.slideList`, so step 3
   covers it. Its loop that hands every picture's bytes to the PDF renderer
   (`pdfImage`) must skip SmartArt files.

## Tests

Every suite runs on JavaScript, C++ and Go (`smartart/tools/run_tests.sh`).
Numbers are minimums.

| Suite | What it pins | At least |
| --- | --- | --- |
| DataReader / DataWriter | every point and connection type, defaults, `srcOrd`, generated transitions and pres points, round trip, refusals with element and reason | 50 |
| LayoutReader | every element and attribute of a layoutDef; unknown ones refused | 30 |
| Iteration | each `forEach` axis × `ptType` × `st`/`cnt`/`step`; `hideLastTrans` | 50 |
| Conditions | each function and operator, nested `choose`, variables | 40 |
| Constraints | each type, every reference form, `op`, `fact`, chains, cycles | 70 |
| Rules | order, limits, a rule that cannot be met | 20 |
| Algorithms | each algorithm × parameters, 1–10 children: positions, sizes, spacing, angles as numbers | 120 |
| Text | shared size, wrap, minimum, output font size | 25 |
| Style | quick-style refs, colour methods over node index, theme colours | 30 |
| Layouts | each built-in layout × 1, 2, 3, 5, 8 nodes × 1–3 levels against the oracle, box by box within a tolerance | 20 × 5+ |
| ToEvg / ToPptx | commands in a box, scaling, z-order; groups PowerPoint opens | 25 |
| Drawing | read PowerPoint's drawing part; write ours | 25 |
| Sliqtly | addImage sizing, overrides, slideList substitution, PPTX replacement, MCP refusal and warnings | 30 |

**The oracle.** PowerPoint's drawing part is its own answer for the data next
to it. A corpus saved by PowerPoint — every built-in layout above with the node
counts and depths of the Layouts suite — gives each layout test its expected
boxes. Producing it needs PowerPoint once (a macro that inserts each layout,
fills it and saves); the files then live in `smartart/oracle/`.

## Phase 0, as built

| Piece | File | Tests |
| --- | --- | --- |
| Data model | `smartart/SaModel.rgr` | |
| Reader: PowerPoint's form and the short form; refusals with element and reason | `smartart/SaDataReader.rgr` | `smartart/tests/SaDataTest.rgr`: 110 checks, JavaScript, C++ and Go (`npm run pptx:smartart:test`) |
| Writer | `smartart/SaDataWriter.rgr` | (same suite: round trips) |
| A diagram with a drawing part drawn as a group; saved over its file as the same SmartArt frame (position and id restated); written as shapes once its text was edited, in a new file, or on another slide | `src/PptxParser.rgr` `parseDiagram`, `src/PptxWriter.rgr` `diagramFrameXml`, `PptxShape.isDiagram`/`diagramFrame`/`diagramSig`/`diagramText`, copied in `PptxEdit` and `PptxResolver` | `tests/PptxSmartArtTest.rgr`: 59 checks (`npm run pptx:smartart:deck:test`); fixture `38-smartart-drawing.pptx` |
| Oracle corpus macro | `smartart/oracle/make_corpus.bas`, `oracle/README.md` | not run yet: needs PowerPoint |

The other pptx suites (test, writer, editor, text, chrome, seam, frame, css,
a11y, editor host, evg, office shapes) and the C++ check pass unchanged.

The parser and writer are not verified on Go: `OpcPackage` does not build on
Go with the current compiler (`indexOf` with a start argument named `idx`
expands to a closure that shadows it, `compiler/Lang.rgr`). The data-model
layer, which has no `OpcPackage`, passes on Go.

## Phase 1, as built

| Piece | File | Tests (each on JavaScript, C++ and Go) |
| --- | --- | --- |
| Layout definitions read, refused by name when they use what the specification does not define | `smartart/SaLayoutDef.rgr`, `SaLayoutReader.rgr` | `SaLayoutTest` 81 |
| The program run against the data: every axis, point type, st/cnt/step, hideLastTrans, choose/if (cnt pos revPos posEven posOdd var depth maxDepth), ref, presOf, inherited variables | `SaPresTree.rgr` | `SaPresTreeTest` 77 |
| Constraints: self/ch/des, forName, ptType, references and factors, equ/gte/lte, defaults, what each value was derived from | `SaConstraints.rgr` | `SaConstraintTest` 43 |
| lin (shrink to fit, tied sizes, alignment, all four directions), composite, sp, tx, conn in a line | `SaAlgorithms.rgr` | `SaAlgorithmTest` 31 |
| Text: which points a shape shows, bullet levels, fitting on the half-point grid, rule floors, equ groups, a host's measurer | `SaText.rgr` | `SaTextTest` 36 |
| Colours by style label and colour set (accentN_2, colorful1, colorful2) | `SaStyle.rgr` | in `SaEngineTest` |
| The engine: a PptxShape group; overrides; unknown layout → vList2 with a warning | `SaEngine.rgr` | `SaEngineTest` 221 |
| process1, chevron1, vList2, hList1 | `builtin/*.xml` → `SaBuiltins.rgr` (generated) | in `SaEngineTest`: each at 1, 2, 3, 5, 8 items |
| The preset geometries those layouts draw with, so a host with no files draws them | `SaPresets.rgr` (generated from `presets.txt`) | in `SaRenderTest` |
| A host's palette, drawing into a display list; a SmartArt file as a host meets it | `SaRender.rgr`, `SaFile.rgr` | `SaRenderTest` 34 |
| A diagram with no drawing part in a .pptx is laid out by the engine | `PptxParser.layoutDiagram` | `PptxSmartArtTest` 64 |

`npm run pptx:smartart:test` runs them all (633 checks per target) and fails
while a generated file is stale. Every other pptx suite and the C++ check
pass.

Sliqtly (its own branch): `src/PresSmartArt.rgr`; `PresDeck.addImage`,
`readSmartArt`, `smartArtOf`, `smartArtNotes`, the `slideList` hook;
`PresApp.stillSmartArt` for the PowerPoint export; the MCP server accepts
`.xml`, warns with the engine's messages and reports a SmartArt as a
diagram; the web page hands the file over undecoded; the guide documents
it. PresCheck 17 checks, a Go test in `mcp-go/smartart_test.go`.

Found on the way, and fixed in terotests/Ranger (`compiler/Lang.rgr`): Go's
`indexOfFrom` hid a caller's `idx` and Go lost the elements a callee pushed
onto an array parameter (both ported from claude/nifty-dijkstra-vq2qit);
C++'s `buffer_from_string` read its argument twice. Not fixed: the C++
writer stops on a subclass with no constructor of its own (SaText says so
where a host would hit it).

Not done in phase 1: a measurer with the host's real fonts (the engine
guesses 0.52 em per character, so on a slide the text may come out a little
smaller or larger than it needs); the oracle comparison (no corpus yet).

## Phases

0. **Data model and drawing reader.** `SaModel`, `SaDataReader`/`Writer`,
   `SaDrawing` read, `SaShape`, `SaToEvg`. RangerPPTX recognises the diagram
   frame and draws a PowerPoint-saved SmartArt from its drawing part. Oracle
   harness and the corpus macro. Three-target test runner.
1. **Engine core and Sliqtly.** `SaLayoutDef`/`Reader`, `SaPresTree`,
   `SaConstraints`, `SaRules`, `lin`, `sp`, `composite`, `tx`, `SaText`,
   `SaStyle`, `SaEngine`, `SaToPptx`; layout group 1; Sliqtly steps 1–6.
2. **Remaining algorithms.** `snake`, `cycle`, `hierRoot`/`hierChild`,
   `pyra`, `conn`; layout group 2.
3. **Native SmartArt out.** PPTX export writes data model, layout definition,
   style, colours and our drawing, so PowerPoint opens it as editable
   SmartArt; the RangerPPTX editor edits a diagram's text and lays it out
   again.
4. **Breadth.** Layout group 3, more styles and colour sets, a whole
   PowerPoint diagram as one referenced file.

## Decisions (2026-10-04)

1. **Oracle corpus.** `smartart/oracle/make_corpus.bas` is a PowerPoint
   macro that inserts each built-in layout with the node counts and depths of
   the Layouts suite and saves one deck per layout. It is run once in
   PowerPoint; the owner checks the decks before they become fixtures. Until
   then the Layouts suite has no expected boxes and is not counted.
2. **A built-in layout named without its definition** (does PowerPoint open
   it?) is deferred to phase 3. Until then the PPTX export writes shapes, not
   SmartArt, so the question does not arise.
3. **Not PRO.** SmartArt is drawn for every user, signed in or not, like the
   list layouts (`process`, `swot`, `timeline`, free since Sliqtly #122).
