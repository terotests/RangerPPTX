# The SmartArt oracle corpus

PowerPoint saves its own layout of every SmartArt diagram in the deck, as a
drawing part beside the diagram's data. For a diagram whose data we also give
the engine, that drawing is the expected answer: the box each shape should
have, its geometry and its text size. This directory holds decks PowerPoint
saved for that purpose.

## Making it

`make_corpus.bas` is a PowerPoint macro. Import it in the VBA editor of
desktop PowerPoint, run `MakeSmartArtCorpus`, and pick an empty folder. It
saves one deck per layout (`process1.pptx`, `cycle2.pptx`…) and `index.txt`.

Each deck has eight slides, one diagram each, in the same 720 × 405 pt frame,
named `case:<name>`:

| Case | Data |
| --- | --- |
| `flat-1`, `flat-2`, `flat-3`, `flat-5`, `flat-8` | that many items, one level |
| `two-levels` | 3 items, 2 under each |
| `three-levels` | 2 items, 2 under each, 2 under those |
| `long-text` | 3 items whose text has to wrap |

With `ONLY_PLANNED = True` (the default) it makes the 20 layouts listed in
`PLAN_SMARTART.md`; `False` makes every layout the PowerPoint has.

## Checking it

The decks are reviewed by the repository owner before they are committed
here. A deck that opens with a diagram showing anything other than the case
it is named for is not committed.

## Using it

The Layouts suite (phase 1) reads each deck with `PptxParser`, takes the
diagram's data (`SaDataReader`), lays it out with the engine into the same
frame, and compares box by box with the drawing. Until the decks are here
the Layouts suite is not counted.
