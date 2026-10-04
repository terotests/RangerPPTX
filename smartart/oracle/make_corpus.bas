Attribute VB_Name = "SmartArtCorpus"
' SPDX-License-Identifier: AGPL-3.0-or-later
'
' The SmartArt oracle corpus: PowerPoint's own layout of each built-in
' layout, for the cases the engine's Layouts suite checks.
'
' Run once in desktop PowerPoint (Windows or Mac):
'   1. Alt+F11 (Mac: Tools > Macro > Visual Basic Editor), File > Import File…,
'      pick this file.
'   2. Run MakeSmartArtCorpus (F5 with the cursor in it).
'   3. Pick an empty folder when asked. One deck per layout is saved there,
'      named after the layout (process1.pptx, cycle2.pptx…), plus index.txt.
'
' Every slide holds ONE diagram, in the same frame (720 x 405 pt at 120, 67.5 on
' a 16:9 slide), named "case:<name>" so the harness knows what it is looking
' at. The saved file carries PowerPoint's drawing part for each diagram, and
' that drawing is the expected answer. Nothing else about the deck matters.
'
' ONLY_PLANNED = True makes the layouts in PLAN_SMARTART.md (20 decks);
' False makes every layout this PowerPoint has (~200 decks, minutes).

Option Explicit

Private Const ONLY_PLANNED As Boolean = True
Private Const FRAME_L As Single = 120
Private Const FRAME_T As Single = 67.5
Private Const FRAME_W As Single = 720
Private Const FRAME_H As Single = 405

Private Function PlannedLayouts() As Variant
    PlannedLayouts = Array( _
        "default", "vList2", "hList1", "process1", "chevron1", _
        "cycle2", "radial1", "hierarchy1", "orgChart1", "pyramid1", _
        "venn1", "matrix1", "target1", "funnel1", "gear1", _
        "arrow2", "bList2", "hProcess9", "lProcess2", "cycle4")
End Function

' "urn:microsoft.com/office/officeart/2005/8/layout/process1" -> "process1"
Private Function ShortId(ByVal id As String) As String
    Dim p As Long
    p = InStrRev(id, "/")
    If p > 0 Then ShortId = Mid$(id, p + 1) Else ShortId = id
End Function

Private Function IsPlanned(ByVal shortName As String) As Boolean
    Dim v As Variant
    For Each v In PlannedLayouts()
        If v = shortName Then
            IsPlanned = True
            Exit Function
        End If
    Next v
End Function

Public Sub MakeSmartArtCorpus()
    Dim folder As String
    With Application.FileDialog(msoFileDialogFolderPicker)
        .Title = "Folder for the SmartArt corpus"
        If .Show <> -1 Then Exit Sub
        folder = .SelectedItems(1)
    End With
    If Right$(folder, 1) <> Application.PathSeparator Then folder = folder & Application.PathSeparator

    Dim index As String
    Dim i As Long
    For i = 1 To Application.SmartArtLayouts.Count
        Dim lo As SmartArtLayout
        Set lo = Application.SmartArtLayouts(i)
        Dim nm As String
        nm = ShortId(lo.Id)
        If (Not ONLY_PLANNED) Or IsPlanned(nm) Then
            Dim made As Long
            made = MakeDeck(lo, folder & nm & ".pptx")
            index = index & nm & vbTab & lo.Id & vbTab & lo.Name & vbTab & made & " slides" & vbCrLf
        End If
    Next i

    Dim f As Integer
    f = FreeFile
    Open folder & "index.txt" For Output As #f
    Print #f, index;
    Close #f
    MsgBox "SmartArt corpus written to " & folder
End Sub

' One deck: a slide per case. Returns how many slides it holds.
Private Function MakeDeck(lo As SmartArtLayout, path As String) As Long
    Dim pres As Presentation
    Set pres = Presentations.Add(msoFalse)
    pres.PageSetup.SlideWidth = 960
    pres.PageSetup.SlideHeight = 540

    ' flat lists
    Dim counts As Variant, c As Variant
    counts = Array(1, 2, 3, 5, 8)
    For Each c In counts
        AddCase pres, lo, "flat-" & c, CLng(c), 0, 0, False
    Next c
    ' two levels: 3 items with 2 under each
    AddCase pres, lo, "two-levels", 3, 2, 0, False
    ' three levels: 2, 2 under each, 2 under those
    AddCase pres, lo, "three-levels", 2, 2, 2, False
    ' text that has to wrap and shrink
    AddCase pres, lo, "long-text", 3, 0, 0, True

    pres.SaveAs path, ppSaveAsOpenXMLPresentation
    MakeDeck = pres.Slides.Count
    pres.Close
End Function

Private Sub AddCase(pres As Presentation, lo As SmartArtLayout, caseName As String, _
                    top As Long, under As Long, underUnder As Long, longText As Boolean)
    Dim sld As Slide
    Set sld = pres.Slides.Add(pres.Slides.Count + 1, ppLayoutBlank)
    Dim shp As Shape
    Set shp = sld.Shapes.AddSmartArt(lo, FRAME_L, FRAME_T, FRAME_W, FRAME_H)
    shp.Name = "case:" & caseName

    Dim sa As SmartArt
    Set sa = shp.SmartArt
    ' down to one top-level node, then build the tree from it
    Do While sa.AllNodes.Count > 1
        sa.AllNodes(sa.AllNodes.Count).Delete
    Loop
    Dim nodeA As SmartArtNode
    Set nodeA = sa.AllNodes(1)
    nodeA.TextFrame2.TextRange.Text = Label(1, longText)

    Dim k As Long, j As Long, m As Long
    Dim prev As SmartArtNode
    Set prev = nodeA
    For k = 2 To top
        Set prev = prev.AddNode(msoSmartArtNodeAfter)
        prev.TextFrame2.TextRange.Text = Label(k, longText)
    Next k

    If under > 0 Then
        For k = 1 To sa.Nodes.Count
            Dim parentNode As SmartArtNode
            Set parentNode = sa.Nodes(k)
            For j = 1 To under
                Dim child As SmartArtNode
                Set child = parentNode.AddNode(msoSmartArtNodeBelow)
                child.TextFrame2.TextRange.Text = "Item " & k & "." & j
                For m = 1 To underUnder
                    Dim grand As SmartArtNode
                    Set grand = child.AddNode(msoSmartArtNodeBelow)
                    grand.TextFrame2.TextRange.Text = "Item " & k & "." & j & "." & m
                Next m
            Next j
        Next k
    End If
End Sub

Private Function Label(ByVal n As Long, ByVal longText As Boolean) As String
    If longText Then
        Label = "Item " & n & " has a label long enough that it has to wrap onto more than one line"
    Else
        Label = "Item " & n
    End If
End Function
