// SPDX-License-Identifier: AGPL-3.0-or-later
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
        "arrow2, bList2, chevron1, cycle2, cycle4, default, funnel1, gear1, hList1, hProcess9, hierarchy1, lProcess2, matrix1, orgChart1, process1, pyramid1, radial1, target1, vList2, venn1".to_string()
    }
    pub fn layoutXml(name: &str) -> String {
        if name == "arrow2" {
            return SaBuiltins::arrow2();
        }
        if name == "bList2" {
            return SaBuiltins::bList2();
        }
        if name == "chevron1" {
            return SaBuiltins::chevron1();
        }
        if name == "cycle2" {
            return SaBuiltins::cycle2();
        }
        if name == "cycle4" {
            return SaBuiltins::cycle4();
        }
        if name == "default" {
            return SaBuiltins::default();
        }
        if name == "funnel1" {
            return SaBuiltins::funnel1();
        }
        if name == "gear1" {
            return SaBuiltins::gear1();
        }
        if name == "hList1" {
            return SaBuiltins::hList1();
        }
        if name == "hProcess9" {
            return SaBuiltins::hProcess9();
        }
        if name == "hierarchy1" {
            return SaBuiltins::hierarchy1();
        }
        if name == "lProcess2" {
            return SaBuiltins::lProcess2();
        }
        if name == "matrix1" {
            return SaBuiltins::matrix1();
        }
        if name == "orgChart1" {
            return SaBuiltins::orgChart1();
        }
        if name == "process1" {
            return SaBuiltins::process1();
        }
        if name == "pyramid1" {
            return SaBuiltins::pyramid1();
        }
        if name == "radial1" {
            return SaBuiltins::radial1();
        }
        if name == "target1" {
            return SaBuiltins::target1();
        }
        if name == "vList2" {
            return SaBuiltins::vList2();
        }
        if name == "venn1" {
            return SaBuiltins::venn1();
        }
        String::new()
    }
    pub fn arrow2() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/arrow2\">");
        s.push_str("<dgm:title val=\"Upward Arrow\"/><dgm:layoutNode name=\"arrow\"><dgm:varLst><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:alg type=\"composite\">");
        s.push_str("<dgm:param type=\"ar\" val=\"1.8\"/></dgm:alg><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"swoosh\" refType=\"w\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"swoosh\" refType=\"h\" fact=\"1\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"swoosh\" refType=\"w\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"swoosh\" refType=\"h\" fact=\"0\"/><dgm:constr type=\"primFontSz\" for=\"ch\" op=\"equ\" val=\"65\"/></dgm:constrLst>");
        s.push_str("<dgm:choose name=\"count\"><dgm:if name=\"n1\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"1\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.47\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot1\" refType=\"h\" fact=\"0.412767\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.3\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.476417\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.495\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.523583\"/>");
        s.push_str("</dgm:constrLst></dgm:if><dgm:if name=\"n2\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"2\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.15\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot1\" refType=\"h\" fact=\"0.701703\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.63\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.223297\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.175\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.776703\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.78\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot2\" refType=\"h\" fact=\"0.282326\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label2\" refType=\"w\" fact=\"0.195\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label2\" refType=\"h\" fact=\"0.576222\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label2\" refType=\"w\" fact=\"0.805\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label2\" refType=\"h\" fact=\"0.423778\"/>");
        s.push_str("</dgm:constrLst></dgm:if><dgm:if name=\"n3\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"3\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.15\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot1\" refType=\"h\" fact=\"0.701703\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.315\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.223297\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.175\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.776703\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.465\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot2\" refType=\"h\" fact=\"0.415718\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label2\" refType=\"w\" fact=\"0.315\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label2\" refType=\"h\" fact=\"0.474028\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label2\" refType=\"w\" fact=\"0.49\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label2\" refType=\"h\" fact=\"0.525972\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot3\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot3\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot3\" refType=\"w\" fact=\"0.78\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot3\" refType=\"h\" fact=\"0.282326\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label3\" refType=\"w\" fact=\"0.195\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label3\" refType=\"h\" fact=\"0.576222\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label3\" refType=\"w\" fact=\"0.805\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label3\" refType=\"h\" fact=\"0.423778\"/>");
        s.push_str("</dgm:constrLst></dgm:if><dgm:if name=\"n4\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"4\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.15\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot1\" refType=\"h\" fact=\"0.701703\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.21\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.223297\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.175\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.776703\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.36\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot2\" refType=\"h\" fact=\"0.486399\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label2\" refType=\"w\" fact=\"0.21\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label2\" refType=\"h\" fact=\"0.416153\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label2\" refType=\"w\" fact=\"0.385\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label2\" refType=\"h\" fact=\"0.583847\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot3\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot3\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot3\" refType=\"w\" fact=\"0.57\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot3\" refType=\"h\" fact=\"0.360296\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label3\" refType=\"w\" fact=\"0.21\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label3\" refType=\"h\" fact=\"0.518265\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label3\" refType=\"w\" fact=\"0.595\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label3\" refType=\"h\" fact=\"0.481735\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"dot4\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"dot4\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot4\" refType=\"w\" fact=\"0.78\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot4\" refType=\"h\" fact=\"0.282326\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label4\" refType=\"w\" fact=\"0.195\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label4\" refType=\"h\" fact=\"0.576222\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"label4\" refType=\"w\" fact=\"0.805\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"label4\" refType=\"h\" fact=\"0.423778\"/>");
        s.push_str("</dgm:constrLst></dgm:if><dgm:else name=\"n5\"><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot1\" refType=\"w\" fact=\"0.15\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot1\" refType=\"h\" fact=\"0.701703\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.1575\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.223297\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"label1\" refType=\"w\" fact=\"0.175\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"label1\" refType=\"h\" fact=\"0.776703\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot2\" refType=\"w\" fact=\"0.3075\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot2\" refType=\"h\" fact=\"0.529125\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"label2\" refType=\"w\" fact=\"0.1575\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"label2\" refType=\"h\" fact=\"0.380847\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"label2\" refType=\"w\" fact=\"0.3325\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"label2\" refType=\"h\" fact=\"0.619153\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"dot3\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"dot3\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot3\" refType=\"w\" fact=\"0.465\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot3\" refType=\"h\" fact=\"0.415718\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"label3\" refType=\"w\" fact=\"0.1575\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"label3\" refType=\"h\" fact=\"0.474028\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"label3\" refType=\"w\" fact=\"0.49\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"label3\" refType=\"h\" fact=\"0.525972\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"dot4\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"dot4\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot4\" refType=\"w\" fact=\"0.6225\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot4\" refType=\"h\" fact=\"0.337153\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"label4\" refType=\"w\" fact=\"0.1575\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"label4\" refType=\"h\" fact=\"0.536186\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"label4\" refType=\"w\" fact=\"0.6475\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"label4\" refType=\"h\" fact=\"0.463814\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"dot5\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"dot5\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"dot5\" refType=\"w\" fact=\"0.78\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"dot5\" refType=\"h\" fact=\"0.282326\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"label5\" refType=\"w\" fact=\"0.1575\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"label5\" refType=\"h\" fact=\"0.576222\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"label5\" refType=\"w\" fact=\"0.805\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"label5\" refType=\"h\" fact=\"0.423778\"/></dgm:constrLst></dgm:else></dgm:choose>");
        s.push_str("<dgm:layoutNode name=\"swoosh\" styleLbl=\"alignAcc1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"swooshArrow\"/></dgm:layoutNode>");
        s.push_str("<dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\" cnt=\"5\"><dgm:choose name=\"position\"><dgm:if name=\"p1\" func=\"pos\" op=\"equ\" val=\"1\">");
        s.push_str("<dgm:layoutNode name=\"dot1\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"ellipse\"/></dgm:layoutNode><dgm:layoutNode name=\"label1\" styleLbl=\"revTx\">");
        s.push_str("<dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/><dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"none\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.1\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.1\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:if name=\"p2\" func=\"pos\" op=\"equ\" val=\"2\"><dgm:layoutNode name=\"dot2\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/>");
        s.push_str("<dgm:shape type=\"ellipse\"/></dgm:layoutNode><dgm:layoutNode name=\"label2\" styleLbl=\"revTx\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/>");
        s.push_str("<dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"none\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.1\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.1\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.1\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:if name=\"p3\" func=\"pos\" op=\"equ\" val=\"3\">");
        s.push_str("<dgm:layoutNode name=\"dot3\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"ellipse\"/></dgm:layoutNode><dgm:layoutNode name=\"label3\" styleLbl=\"revTx\">");
        s.push_str("<dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/><dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"none\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.1\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.1\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:if name=\"p4\" func=\"pos\" op=\"equ\" val=\"4\"><dgm:layoutNode name=\"dot4\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/>");
        s.push_str("<dgm:shape type=\"ellipse\"/></dgm:layoutNode><dgm:layoutNode name=\"label4\" styleLbl=\"revTx\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/>");
        s.push_str("<dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"none\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.1\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.1\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.1\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:else name=\"p5\">");
        s.push_str("<dgm:layoutNode name=\"dot5\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"ellipse\"/></dgm:layoutNode><dgm:layoutNode name=\"label5\" styleLbl=\"revTx\">");
        s.push_str("<dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/><dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"none\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.1\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.1\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:else></dgm:choose></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn bList2() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/bList2\">");
        s.push_str("<dgm:title val=\"Bending Picture Accent List\"/><dgm:layoutNode name=\"list\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst>");
        s.push_str("<dgm:alg type=\"snake\"><dgm:param type=\"grDir\" val=\"tL\"/><dgm:param type=\"flowDir\" val=\"row\"/><dgm:param type=\"contDir\" val=\"sameDir\"/>");
        s.push_str("<dgm:param type=\"off\" val=\"ctr\"/></dgm:alg><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"cell\" refType=\"w\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"cell\" refType=\"w\" refFor=\"ch\" refForName=\"cell\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"cell\" fact=\"0.08\"/>");
        s.push_str("<dgm:constr type=\"secSibSp\" refType=\"w\" refFor=\"ch\" refForName=\"cell\" fact=\"0.06\"/><dgm:constr type=\"primFontSz\" for=\"des\" forName=\"text\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\"><dgm:layoutNode name=\"cell\"><dgm:alg type=\"composite\"/><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"text\" refType=\"w\" fact=\"1\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"text\" refType=\"h\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"text\" refType=\"w\" fact=\"0\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"text\" refType=\"h\" fact=\"0\"/></dgm:constrLst>");
        s.push_str("<dgm:layoutNode name=\"text\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"roundRect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:layoutNode></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn chevron1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/chevron1\">");
        s.push_str("<dgm:title val=\"Basic Chevron Process\"/><dgm:layoutNode name=\"diagram\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst>");
        s.push_str("<dgm:choose name=\"direction\"><dgm:if name=\"forward\" func=\"var\" arg=\"dir\" op=\"equ\" val=\"norm\"><dgm:alg type=\"lin\"/></dgm:if><dgm:else name=\"backward\">");
        s.push_str("<dgm:alg type=\"lin\"><dgm:param type=\"linDir\" val=\"fromR\"/></dgm:alg></dgm:else></dgm:choose><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"node\" refType=\"w\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"node\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.4\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"-0.1\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"node\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"steps\" axis=\"ch\" ptType=\"node\"><dgm:layoutNode name=\"node\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"chevron\">");
        s.push_str("<dgm:adjLst><dgm:adj idx=\"1\" val=\"0.5\"/></dgm:adjLst></dgm:shape><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.15\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.15\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.2\"/><dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.2\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn cycle2() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/cycle2\">");
        s.push_str("<dgm:title val=\"Basic Cycle\"/><dgm:layoutNode name=\"cycle\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:choose name=\"direction\">");
        s.push_str("<dgm:if name=\"forward\" func=\"var\" arg=\"dir\" op=\"equ\" val=\"norm\"><dgm:alg type=\"cycle\"><dgm:param type=\"stAng\" val=\"0\"/><dgm:param type=\"spanAng\" val=\"360\"/>");
        s.push_str("</dgm:alg></dgm:if><dgm:else name=\"backward\"><dgm:alg type=\"cycle\"><dgm:param type=\"stAng\" val=\"0\"/><dgm:param type=\"spanAng\" val=\"-360\"/></dgm:alg></dgm:else>");
        s.push_str("</dgm:choose><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"node\" refType=\"w\" fact=\"0.2\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"node\" refType=\"w\" refFor=\"ch\" refForName=\"node\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"sibTrans\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.35\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"sibTrans\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.25\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.4\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"node\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\"><dgm:layoutNode name=\"node\" styleLbl=\"node1\"><dgm:varLst><dgm:bulletEnabled val=\"1\"/>");
        s.push_str("</dgm:varLst><dgm:alg type=\"tx\"/><dgm:shape type=\"ellipse\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.15\"/><dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.15\"/><dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.08\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.08\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode><dgm:forEach name=\"arrows\" axis=\"followSib\" ptType=\"sibTrans\" hideLastTrans=\"0\" cnt=\"1\"><dgm:layoutNode name=\"sibTrans\" styleLbl=\"sibTrans2D1\">");
        s.push_str("<dgm:alg type=\"conn\"/><dgm:shape type=\"conn\"><dgm:adjLst><dgm:adj idx=\"1\" val=\"0.6\"/><dgm:adj idx=\"2\" val=\"0.5\"/></dgm:adjLst></dgm:shape></dgm:layoutNode>");
        s.push_str("</dgm:forEach></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn cycle4() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/cycle4\">");
        s.push_str("<dgm:title val=\"Cycle Matrix\"/><dgm:layoutNode name=\"cycle\"><dgm:varLst><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:alg type=\"composite\"/><dgm:shape/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"core\" refType=\"h\" fact=\"1\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"core\" refType=\"h\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"core\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"core\" refType=\"h\" fact=\"0.5\"/></dgm:constrLst>");
        s.push_str("<dgm:layoutNode name=\"core\"><dgm:alg type=\"composite\"/><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"wedges\" refType=\"w\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"wedges\" refType=\"h\" fact=\"1\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"wedges\" refType=\"w\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"wedges\" refType=\"h\" fact=\"0\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"labels\" refType=\"w\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"labels\" refType=\"h\" fact=\"1\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"labels\" refType=\"w\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"labels\" refType=\"h\" fact=\"0\"/><dgm:constr type=\"primFontSz\" for=\"des\" forName=\"label\" op=\"equ\" val=\"65\"/></dgm:constrLst>");
        s.push_str("<dgm:layoutNode name=\"wedges\"><dgm:alg type=\"snake\"><dgm:param type=\"bkpt\" val=\"fixed\"/><dgm:param type=\"bkPtFixedVal\" val=\"2\"/></dgm:alg><dgm:shape/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"wedge\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"wedge\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" fact=\"0.02\"/></dgm:constrLst><dgm:forEach name=\"shapes\" axis=\"ch\" ptType=\"node\" cnt=\"4\"><dgm:choose name=\"position\">");
        s.push_str("<dgm:if name=\"p1\" func=\"pos\" op=\"equ\" val=\"1\"><dgm:layoutNode name=\"wedge\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"pieWedge\"/></dgm:layoutNode>");
        s.push_str("</dgm:if><dgm:if name=\"p2\" func=\"pos\" op=\"equ\" val=\"2\"><dgm:layoutNode name=\"wedge\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"pieWedge\" rot=\"90\"/>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:if name=\"p3\" func=\"pos\" op=\"equ\" val=\"3\"><dgm:layoutNode name=\"wedge\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/>");
        s.push_str("<dgm:shape type=\"pieWedge\" rot=\"270\"/></dgm:layoutNode></dgm:if><dgm:else name=\"p4\"><dgm:layoutNode name=\"wedge\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/>");
        s.push_str("<dgm:shape type=\"pieWedge\" rot=\"180\"/></dgm:layoutNode></dgm:else></dgm:choose></dgm:forEach></dgm:layoutNode><dgm:layoutNode name=\"labels\">");
        s.push_str("<dgm:alg type=\"snake\"><dgm:param type=\"bkpt\" val=\"fixed\"/><dgm:param type=\"bkPtFixedVal\" val=\"2\"/></dgm:alg><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"label\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"label\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" fact=\"0.02\"/></dgm:constrLst><dgm:forEach name=\"texts\" axis=\"ch\" ptType=\"node\" cnt=\"4\"><dgm:choose name=\"position\">");
        s.push_str("<dgm:if name=\"p1\" func=\"pos\" op=\"equ\" val=\"1\"><dgm:layoutNode name=\"label\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"none\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.31\"/><dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.31\"/><dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.05\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:if name=\"p2\" func=\"pos\" op=\"equ\" val=\"2\">");
        s.push_str("<dgm:layoutNode name=\"label\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"none\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.31\"/><dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.31\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.05\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:if name=\"p3\" func=\"pos\" op=\"equ\" val=\"3\"><dgm:layoutNode name=\"label\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg>");
        s.push_str("<dgm:shape type=\"none\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.31\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.31\"/>");
        s.push_str("</dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:else name=\"p4\">");
        s.push_str("<dgm:layoutNode name=\"label\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"none\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.31\"/><dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.05\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.31\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:else></dgm:choose></dgm:forEach></dgm:layoutNode></dgm:layoutNode></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn default() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/default\">");
        s.push_str("<dgm:title val=\"Basic Block List\"/><dgm:layoutNode name=\"diagram\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst>");
        s.push_str("<dgm:choose name=\"direction\"><dgm:if name=\"forward\" func=\"var\" arg=\"dir\" op=\"equ\" val=\"norm\"><dgm:alg type=\"snake\"><dgm:param type=\"grDir\" val=\"tL\"/>");
        s.push_str("<dgm:param type=\"flowDir\" val=\"row\"/><dgm:param type=\"contDir\" val=\"sameDir\"/><dgm:param type=\"off\" val=\"ctr\"/></dgm:alg></dgm:if><dgm:else name=\"backward\">");
        s.push_str("<dgm:alg type=\"snake\"><dgm:param type=\"grDir\" val=\"tR\"/><dgm:param type=\"flowDir\" val=\"row\"/><dgm:param type=\"contDir\" val=\"sameDir\"/>");
        s.push_str("<dgm:param type=\"off\" val=\"ctr\"/></dgm:alg></dgm:else></dgm:choose><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"node\" refType=\"w\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"node\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.6\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"secSibSp\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.1\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"node\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\"><dgm:layoutNode name=\"node\" styleLbl=\"node1\"><dgm:varLst><dgm:bulletEnabled val=\"1\"/>");
        s.push_str("</dgm:varLst><dgm:alg type=\"tx\"/><dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn funnel1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/funnel1\">");
        s.push_str("<dgm:title val=\"Funnel\"/><dgm:layoutNode name=\"funnel\"><dgm:varLst><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:alg type=\"composite\">");
        s.push_str("<dgm:param type=\"ar\" val=\"1.0\"/></dgm:alg><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"shape\" refType=\"w\" fact=\"0.9\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"shape\" refType=\"h\" fact=\"0.64\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"shape\" refType=\"h\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"shape\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"mouth\" refType=\"w\" fact=\"0.84\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"mouth\" refType=\"h\" fact=\"0.28\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"mouth\" refType=\"h\" fact=\"0.03\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"mouth\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"arrow\" refType=\"w\" fact=\"0.06\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"arrow\" refType=\"h\" fact=\"0.09\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"arrow\" refType=\"h\" fact=\"0.66\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"arrow\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"result\" refType=\"w\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"result\" refType=\"h\" fact=\"0.2\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"result\" refType=\"h\" fact=\"0.79\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"result\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"primFontSz\" for=\"des\" forName=\"item\" op=\"equ\" val=\"65\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"result\" op=\"equ\" val=\"65\"/></dgm:constrLst><dgm:layoutNode name=\"shape\" styleLbl=\"alignAcc1\">");
        s.push_str("<dgm:alg type=\"tx\"/><dgm:shape type=\"funnel\"/></dgm:layoutNode><dgm:layoutNode name=\"mouth\"><dgm:alg type=\"snake\"><dgm:param type=\"off\" val=\"ctr\"/></dgm:alg>");
        s.push_str("<dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"item\" refType=\"h\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"item\" refType=\"h\" fact=\"1\"/><dgm:constr type=\"sibSp\" refType=\"w\" fact=\"0\"/></dgm:constrLst>");
        s.push_str("<dgm:forEach name=\"poured\" axis=\"ch\" ptType=\"node\"><dgm:choose name=\"notLast\"><dgm:if name=\"poured\" func=\"revPos\" op=\"gt\" val=\"1\">");
        s.push_str("<dgm:layoutNode name=\"item\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"ellipse\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.1\"/><dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.1\"/><dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.1\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:else name=\"last\"/></dgm:choose></dgm:forEach></dgm:layoutNode><dgm:layoutNode name=\"arrow\" styleLbl=\"sibTrans2D1\">");
        s.push_str("<dgm:alg type=\"tx\"/><dgm:shape type=\"downArrow\"/></dgm:layoutNode><dgm:forEach name=\"out\" axis=\"ch\" ptType=\"node\"><dgm:choose name=\"isLast\">");
        s.push_str("<dgm:if name=\"last\" func=\"revPos\" op=\"equ\" val=\"1\"><dgm:layoutNode name=\"result\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"roundRect\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:else name=\"others\"/></dgm:choose></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn gear1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/gear1\">");
        s.push_str("<dgm:title val=\"Gear\"/><dgm:layoutNode name=\"gear\"><dgm:varLst><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:alg type=\"composite\"/><dgm:shape/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"gears\" refType=\"h\" fact=\"0.962963\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"gears\" refType=\"h\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"gears\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"gears\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("</dgm:constrLst><dgm:layoutNode name=\"gears\"><dgm:alg type=\"composite\"/><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"gear1\" refType=\"h\" fact=\"0.691358\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"gear1\" refType=\"h\" fact=\"0.691358\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"gear1\" refType=\"w\" fact=\"0.358974\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"gear1\" refType=\"h\" fact=\"0.604938\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"gear2\" refType=\"h\" fact=\"0.493827\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"gear2\" refType=\"h\" fact=\"0.493827\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"gear2\" refType=\"w\" fact=\"0.730769\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"gear2\" refType=\"h\" fact=\"0.246914\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"gear3\" refType=\"h\" fact=\"0.395062\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"gear3\" refType=\"h\" fact=\"0.395062\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"gear3\" refType=\"w\" fact=\"0.794872\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"gear3\" refType=\"h\" fact=\"0.802469\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"ch\" op=\"equ\" val=\"65\"/></dgm:constrLst><dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\" cnt=\"3\"><dgm:choose name=\"position\">");
        s.push_str("<dgm:if name=\"p1\" func=\"pos\" op=\"equ\" val=\"1\"><dgm:layoutNode name=\"gear1\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"gear9\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.17\"/><dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.17\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.17\"/><dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.17\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:if name=\"p2\" func=\"pos\" op=\"equ\" val=\"2\">");
        s.push_str("<dgm:layoutNode name=\"gear2\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"gear6\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.17\"/><dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.17\"/><dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.17\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.17\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:else name=\"p3\"><dgm:layoutNode name=\"gear3\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"gear6\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.17\"/><dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.17\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.17\"/><dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.17\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:else></dgm:choose></dgm:forEach></dgm:layoutNode>");
        s.push_str("</dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn hList1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/hList1\">");
        s.push_str("<dgm:title val=\"Horizontal Bullet List\"/><dgm:layoutNode name=\"diagram\"><dgm:varLst><dgm:dir/><dgm:animLvl val=\"lvl\"/><dgm:resizeHandles val=\"exact\"/>");
        s.push_str("</dgm:varLst><dgm:choose name=\"direction\"><dgm:if name=\"forward\" func=\"var\" arg=\"dir\" op=\"equ\" val=\"norm\"><dgm:alg type=\"lin\">");
        s.push_str("<dgm:param type=\"nodeVertAlign\" val=\"t\"/></dgm:alg></dgm:if><dgm:else name=\"backward\"><dgm:alg type=\"lin\"><dgm:param type=\"linDir\" val=\"fromR\"/>");
        s.push_str("<dgm:param type=\"nodeVertAlign\" val=\"t\"/></dgm:alg></dgm:else></dgm:choose><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"composite\" refType=\"w\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"composite\" refType=\"h\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" fact=\"0.05\"/><dgm:constr type=\"primFontSz\" for=\"des\" forName=\"parTx\" op=\"equ\" val=\"65\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"des\" forName=\"desTx\" op=\"equ\" val=\"65\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"des\" forName=\"desTx\" refType=\"primFontSz\" refFor=\"des\" refForName=\"parTx\" op=\"lte\" fact=\"0.78\"/></dgm:constrLst>");
        s.push_str("<dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\"><dgm:layoutNode name=\"composite\"><dgm:alg type=\"composite\"/><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"parTx\" val=\"0\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"parTx\" val=\"0\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"parTx\" refType=\"w\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"parTx\" refType=\"h\" fact=\"0.25\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"desTx\" val=\"0\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"desTx\" refType=\"h\" fact=\"0.25\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"desTx\" refType=\"w\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"desTx\" refType=\"h\" fact=\"0.75\"/></dgm:constrLst>");
        s.push_str("<dgm:layoutNode name=\"parTx\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"rect\"/><dgm:presOf axis=\"self\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode><dgm:layoutNode name=\"desTx\" styleLbl=\"alignAccFollowNode1\">");
        s.push_str("<dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/><dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"rect\"/>");
        s.push_str("<dgm:presOf axis=\"des\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.2\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.2\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.2\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.2\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:layoutNode></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn hProcess9() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/hProcess9\">");
        s.push_str("<dgm:title val=\"Continuous Block Process\"/><dgm:layoutNode name=\"process\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst>");
        s.push_str("<dgm:alg type=\"composite\"/><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"arrow\" refType=\"w\" fact=\"0.9\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"arrow\" refType=\"h\" fact=\"0.84\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"arrow\" refType=\"w\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"arrow\" refType=\"h\" fact=\"0.5\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"row\" refType=\"w\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"row\" refType=\"h\" fact=\"0.38\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"row\" refType=\"w\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"row\" refType=\"h\" fact=\"0.5\"/></dgm:constrLst><dgm:layoutNode name=\"arrow\" styleLbl=\"bgShp\"><dgm:alg type=\"tx\"/>");
        s.push_str("<dgm:shape type=\"rightArrow\"/></dgm:layoutNode><dgm:layoutNode name=\"row\"><dgm:alg type=\"lin\"/><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"node\" refType=\"w\" fact=\"0.3\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"node\" refType=\"h\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.12\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"node\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\"><dgm:layoutNode name=\"node\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg>");
        s.push_str("<dgm:shape type=\"roundRect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:forEach></dgm:layoutNode></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn hierarchy1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/hierarchy1\">");
        s.push_str("<dgm:title val=\"Hierarchy\"/><dgm:layoutNode name=\"hierChild1\"><dgm:varLst><dgm:orgChart val=\"1\"/><dgm:chPref val=\"1\"/><dgm:dir/><dgm:animOne val=\"branch\"/>");
        s.push_str("<dgm:animLvl val=\"lvl\"/><dgm:resizeHandles/></dgm:varLst><dgm:alg type=\"hierChild\"/><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"des\" forName=\"node\" refType=\"w\" fact=\"0.2\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"des\" forName=\"node\" refType=\"w\" refFor=\"des\" refForName=\"node\" fact=\"0.62\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"des\" refForName=\"node\" fact=\"0.15\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" for=\"des\" forName=\"hierChild2\" refType=\"w\" refFor=\"des\" refForName=\"node\" fact=\"0.15\"/>");
        s.push_str("<dgm:constr type=\"sp\" for=\"des\" forName=\"hierRoot1\" refType=\"h\" refFor=\"des\" refForName=\"node\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"des\" forName=\"node\" op=\"equ\" val=\"65\"/></dgm:constrLst><dgm:forEach name=\"branches\" axis=\"ch\" ptType=\"node\">");
        s.push_str("<dgm:forEach name=\"toBranch\" axis=\"precedSib\" ptType=\"parTrans\" cnt=\"1\"><dgm:layoutNode name=\"line\" styleLbl=\"parChTrans1D2\"><dgm:alg type=\"conn\">");
        s.push_str("<dgm:param type=\"dim\" val=\"1D\"/><dgm:param type=\"endSty\" val=\"noArr\"/><dgm:param type=\"connRout\" val=\"bend\"/></dgm:alg><dgm:shape type=\"conn\"/></dgm:layoutNode>");
        s.push_str("</dgm:forEach><dgm:layoutNode name=\"hierRoot1\"><dgm:alg type=\"hierRoot\"/><dgm:shape/><dgm:layoutNode name=\"node\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/>");
        s.push_str("<dgm:shape type=\"roundRect\"><dgm:adjLst><dgm:adj idx=\"1\" val=\"0.1\"/></dgm:adjLst></dgm:shape><dgm:presOf axis=\"self\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode><dgm:layoutNode name=\"hierChild2\"><dgm:alg type=\"hierChild\"/>");
        s.push_str("<dgm:shape/><dgm:forEach name=\"under\" ref=\"branches\"/></dgm:layoutNode></dgm:layoutNode></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn lProcess2() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/lProcess2\">");
        s.push_str("<dgm:title val=\"Grouped List\"/><dgm:layoutNode name=\"lists\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:alg type=\"lin\"/><dgm:shape/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"group\" refType=\"w\" fact=\"0.32\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"group\" refType=\"h\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"group\" fact=\"0.08\"/><dgm:constr type=\"primFontSz\" for=\"des\" forName=\"header\" op=\"equ\" val=\"65\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"des\" forName=\"child\" op=\"equ\" val=\"65\"/></dgm:constrLst><dgm:forEach name=\"groups\" axis=\"ch\" ptType=\"node\">");
        s.push_str("<dgm:layoutNode name=\"group\"><dgm:alg type=\"composite\"/><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"bg\" refType=\"w\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"bg\" refType=\"h\" fact=\"1\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"bg\" refType=\"w\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"bg\" refType=\"h\" fact=\"0\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"header\" refType=\"w\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"header\" refType=\"h\" fact=\"0.18\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"header\" refType=\"w\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"header\" refType=\"h\" fact=\"0\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"kids\" refType=\"w\" fact=\"0.86\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"kids\" refType=\"h\" fact=\"0.78\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"kids\" refType=\"h\" fact=\"0.2\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"kids\" refType=\"w\" fact=\"0.5\"/></dgm:constrLst><dgm:layoutNode name=\"bg\" styleLbl=\"bgShp\"><dgm:alg type=\"tx\"/>");
        s.push_str("<dgm:shape type=\"roundRect\"/></dgm:layoutNode><dgm:layoutNode name=\"header\" styleLbl=\"revTx\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"mid\"/>");
        s.push_str("</dgm:alg><dgm:shape type=\"none\"/><dgm:presOf axis=\"self\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode><dgm:layoutNode name=\"kids\"><dgm:alg type=\"lin\"><dgm:param type=\"linDir\" val=\"fromT\"/><dgm:param type=\"nodeVertAlign\" val=\"t\"/></dgm:alg>");
        s.push_str("<dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"child\" refType=\"w\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"child\" refType=\"w\" refFor=\"ch\" refForName=\"child\" fact=\"0.42\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"child\" fact=\"0.06\"/></dgm:constrLst><dgm:forEach name=\"under\" axis=\"ch\" ptType=\"node\">");
        s.push_str("<dgm:layoutNode name=\"child\" styleLbl=\"node1\"><dgm:alg type=\"tx\"></dgm:alg><dgm:shape type=\"roundRect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:forEach></dgm:layoutNode></dgm:layoutNode></dgm:forEach>");
        s.push_str("</dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn matrix1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/matrix1\">");
        s.push_str("<dgm:title val=\"Titled Matrix\"/><dgm:layoutNode name=\"matrix\"><dgm:varLst><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:alg type=\"composite\"/><dgm:shape/>");
        s.push_str("<dgm:choose name=\"form\"><dgm:if name=\"titled\" func=\"maxDepth\" op=\"gte\" val=\"2\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"grid\" refType=\"w\" fact=\"1\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"grid\" refType=\"h\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"grid\" refType=\"w\" fact=\"0\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"grid\" refType=\"h\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"title\" refType=\"w\" fact=\"0.36\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"title\" refType=\"h\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"title\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"title\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"des\" forName=\"quad\" op=\"equ\" val=\"65\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"title\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"theTitle\" axis=\"ch\" ptType=\"node\" cnt=\"1\"><dgm:layoutNode name=\"grid\"><dgm:alg type=\"snake\">");
        s.push_str("<dgm:param type=\"bkpt\" val=\"fixed\"/><dgm:param type=\"bkPtFixedVal\" val=\"2\"/><dgm:param type=\"off\" val=\"off\"/></dgm:alg><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"quad\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"quad\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" fact=\"0\"/></dgm:constrLst><dgm:forEach name=\"quadrants\" axis=\"ch\" ptType=\"node\" cnt=\"4\"><dgm:choose name=\"position\">");
        s.push_str("<dgm:if name=\"p1\" func=\"pos\" op=\"equ\" val=\"1\"><dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/>");
        s.push_str("<dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:if name=\"p2\" func=\"pos\" op=\"equ\" val=\"2\">");
        s.push_str("<dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/><dgm:param type=\"parTxLTRAlign\" val=\"r\"/></dgm:alg>");
        s.push_str("<dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:if name=\"p3\" func=\"pos\" op=\"equ\" val=\"3\"><dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\">");
        s.push_str("<dgm:param type=\"txAnchorVert\" val=\"b\"/><dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:else name=\"p4\">");
        s.push_str("<dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"b\"/><dgm:param type=\"parTxLTRAlign\" val=\"r\"/></dgm:alg>");
        s.push_str("<dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:else></dgm:choose></dgm:forEach></dgm:layoutNode><dgm:layoutNode name=\"title\" styleLbl=\"fgAcc1\"><dgm:alg type=\"tx\"></dgm:alg>");
        s.push_str("<dgm:shape type=\"roundRect\"/><dgm:presOf axis=\"self\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:forEach></dgm:if><dgm:if name=\"alone\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"1\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"grid\" refType=\"w\" fact=\"1\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"grid\" refType=\"h\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"l\" for=\"ch\" forName=\"grid\" refType=\"w\" fact=\"0\"/><dgm:constr type=\"t\" for=\"ch\" forName=\"grid\" refType=\"h\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"title\" refType=\"w\" fact=\"0.36\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"title\" refType=\"h\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"title\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"title\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"des\" forName=\"quad\" op=\"equ\" val=\"65\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"title\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"theTitle\" axis=\"ch\" ptType=\"node\" cnt=\"1\"><dgm:layoutNode name=\"grid\"><dgm:alg type=\"snake\">");
        s.push_str("<dgm:param type=\"bkpt\" val=\"fixed\"/><dgm:param type=\"bkPtFixedVal\" val=\"2\"/><dgm:param type=\"off\" val=\"off\"/></dgm:alg><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"quad\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"quad\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" fact=\"0\"/></dgm:constrLst><dgm:forEach name=\"quadrants\" axis=\"ch\" ptType=\"node\" cnt=\"4\"><dgm:choose name=\"position\">");
        s.push_str("<dgm:if name=\"p1\" func=\"pos\" op=\"equ\" val=\"1\"><dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/>");
        s.push_str("<dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:if name=\"p2\" func=\"pos\" op=\"equ\" val=\"2\">");
        s.push_str("<dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/><dgm:param type=\"parTxLTRAlign\" val=\"r\"/></dgm:alg>");
        s.push_str("<dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:if name=\"p3\" func=\"pos\" op=\"equ\" val=\"3\"><dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\">");
        s.push_str("<dgm:param type=\"txAnchorVert\" val=\"b\"/><dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:else name=\"p4\">");
        s.push_str("<dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"b\"/><dgm:param type=\"parTxLTRAlign\" val=\"r\"/></dgm:alg>");
        s.push_str("<dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:else></dgm:choose></dgm:forEach></dgm:layoutNode><dgm:layoutNode name=\"title\" styleLbl=\"fgAcc1\"><dgm:alg type=\"tx\"></dgm:alg>");
        s.push_str("<dgm:shape type=\"roundRect\"/><dgm:presOf axis=\"self\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:forEach></dgm:if><dgm:else name=\"flat\"><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"grid\" refType=\"w\" fact=\"1\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"grid\" refType=\"h\" fact=\"1\"/><dgm:constr type=\"l\" for=\"ch\" forName=\"grid\" refType=\"w\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"t\" for=\"ch\" forName=\"grid\" refType=\"h\" fact=\"0\"/><dgm:constr type=\"primFontSz\" for=\"des\" forName=\"quad\" op=\"equ\" val=\"65\"/></dgm:constrLst>");
        s.push_str("<dgm:layoutNode name=\"grid\"><dgm:alg type=\"snake\"><dgm:param type=\"bkpt\" val=\"fixed\"/><dgm:param type=\"bkPtFixedVal\" val=\"2\"/><dgm:param type=\"off\" val=\"off\"/>");
        s.push_str("</dgm:alg><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"quad\" refType=\"w\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"quad\" refType=\"h\" fact=\"0.5\"/><dgm:constr type=\"sibSp\" refType=\"w\" fact=\"0\"/></dgm:constrLst>");
        s.push_str("<dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\" cnt=\"4\"><dgm:choose name=\"position\"><dgm:if name=\"p1\" func=\"pos\" op=\"equ\" val=\"1\">");
        s.push_str("<dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/><dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg>");
        s.push_str("<dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:if name=\"p2\" func=\"pos\" op=\"equ\" val=\"2\"><dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\">");
        s.push_str("<dgm:param type=\"txAnchorVert\" val=\"t\"/><dgm:param type=\"parTxLTRAlign\" val=\"r\"/></dgm:alg><dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:if name=\"p3\" func=\"pos\" op=\"equ\" val=\"3\">");
        s.push_str("<dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"b\"/><dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg>");
        s.push_str("<dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:else name=\"p4\"><dgm:layoutNode name=\"quad\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"b\"/>");
        s.push_str("<dgm:param type=\"parTxLTRAlign\" val=\"r\"/></dgm:alg><dgm:shape type=\"rect\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:else></dgm:choose></dgm:forEach></dgm:layoutNode></dgm:else>");
        s.push_str("</dgm:choose></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn orgChart1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/orgChart1\">");
        s.push_str("<dgm:title val=\"Organization Chart\"/><dgm:layoutNode name=\"hierChild1\"><dgm:varLst><dgm:orgChart val=\"1\"/><dgm:chPref val=\"1\"/><dgm:dir/>");
        s.push_str("<dgm:animOne val=\"branch\"/><dgm:animLvl val=\"lvl\"/><dgm:resizeHandles/></dgm:varLst><dgm:alg type=\"hierChild\"/><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"des\" forName=\"node\" refType=\"w\" fact=\"0.18\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"des\" forName=\"node\" refType=\"w\" refFor=\"des\" refForName=\"node\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"des\" forName=\"asstNode\" refType=\"w\" refFor=\"des\" refForName=\"node\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"des\" forName=\"asstNode\" refType=\"h\" refFor=\"des\" refForName=\"node\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"des\" refForName=\"node\" fact=\"0.12\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" for=\"des\" forName=\"hierChild2\" refType=\"w\" refFor=\"des\" refForName=\"node\" fact=\"0.12\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" for=\"des\" forName=\"hierChild3\" refType=\"w\" refFor=\"des\" refForName=\"node\" fact=\"0.12\"/>");
        s.push_str("<dgm:constr type=\"sp\" for=\"des\" forName=\"hierRoot1\" refType=\"h\" refFor=\"des\" refForName=\"node\" fact=\"0.6\"/>");
        s.push_str("<dgm:constr type=\"sp\" for=\"des\" forName=\"hierRoot2\" refType=\"h\" refFor=\"des\" refForName=\"node\" fact=\"0.6\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"des\" forName=\"node\" op=\"equ\" val=\"65\"/><dgm:constr type=\"primFontSz\" for=\"des\" forName=\"asstNode\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"branches\" axis=\"ch\" ptType=\"norm\"><dgm:forEach name=\"toBranch\" axis=\"precedSib\" ptType=\"parTrans\" cnt=\"1\">");
        s.push_str("<dgm:layoutNode name=\"line\" styleLbl=\"parChTrans1D2\"><dgm:alg type=\"conn\"><dgm:param type=\"dim\" val=\"1D\"/><dgm:param type=\"endSty\" val=\"noArr\"/>");
        s.push_str("<dgm:param type=\"connRout\" val=\"bend\"/></dgm:alg><dgm:shape type=\"conn\"/></dgm:layoutNode></dgm:forEach><dgm:layoutNode name=\"hierRoot1\">");
        s.push_str("<dgm:alg type=\"hierRoot\"/><dgm:shape/><dgm:layoutNode name=\"node\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"rect\"/><dgm:presOf axis=\"self\"/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode><dgm:layoutNode name=\"hierChild3\"><dgm:alg type=\"hierChild\"/>");
        s.push_str("<dgm:shape/><dgm:forEach name=\"assistants\" axis=\"ch\" ptType=\"asst\"><dgm:forEach name=\"toAssistant\" axis=\"precedSib\" ptType=\"parTrans\" cnt=\"1\">");
        s.push_str("<dgm:layoutNode name=\"asstLine\" styleLbl=\"parChTrans1D2\"><dgm:alg type=\"conn\"><dgm:param type=\"dim\" val=\"1D\"/><dgm:param type=\"endSty\" val=\"noArr\"/>");
        s.push_str("<dgm:param type=\"connRout\" val=\"bend\"/></dgm:alg><dgm:shape type=\"conn\"/></dgm:layoutNode></dgm:forEach><dgm:layoutNode name=\"hierRoot2\">");
        s.push_str("<dgm:alg type=\"hierRoot\"/><dgm:shape/><dgm:layoutNode name=\"asstNode\" styleLbl=\"node1\"><dgm:alg type=\"tx\"/><dgm:shape type=\"rect\"/><dgm:presOf axis=\"self\"/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode><dgm:layoutNode name=\"hierChild2\"><dgm:alg type=\"hierChild\"/>");
        s.push_str("<dgm:shape/><dgm:forEach name=\"underAssistant\" ref=\"branches\"/></dgm:layoutNode></dgm:layoutNode></dgm:forEach></dgm:layoutNode>");
        s.push_str("<dgm:layoutNode name=\"hierChild2\"><dgm:alg type=\"hierChild\"/><dgm:shape/><dgm:forEach name=\"under\" ref=\"branches\"/></dgm:layoutNode></dgm:layoutNode>");
        s.push_str("</dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn process1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/process1\">");
        s.push_str("<dgm:title val=\"Basic Process\"/><dgm:layoutNode name=\"diagram\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:choose name=\"direction\">");
        s.push_str("<dgm:if name=\"forward\" func=\"var\" arg=\"dir\" op=\"equ\" val=\"norm\"><dgm:alg type=\"lin\"/></dgm:if><dgm:else name=\"backward\"><dgm:alg type=\"lin\">");
        s.push_str("<dgm:param type=\"linDir\" val=\"fromR\"/></dgm:alg></dgm:else></dgm:choose><dgm:shape/><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"node\" refType=\"w\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"node\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.6\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"sibTrans\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.21\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"sibTrans\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.26\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.1\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"node\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"steps\" axis=\"ch\" ptType=\"node\"><dgm:layoutNode name=\"node\" styleLbl=\"node1\"><dgm:varLst><dgm:bulletEnabled val=\"1\"/>");
        s.push_str("</dgm:varLst><dgm:alg type=\"tx\"/><dgm:shape type=\"roundRect\"><dgm:adjLst><dgm:adj idx=\"1\" val=\"0.1\"/></dgm:adjLst></dgm:shape>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode><dgm:forEach name=\"arrows\" axis=\"followSib\" ptType=\"sibTrans\" cnt=\"1\"><dgm:layoutNode name=\"sibTrans\" styleLbl=\"sibTrans2D1\">");
        s.push_str("<dgm:alg type=\"conn\"/><dgm:shape type=\"conn\"><dgm:adjLst><dgm:adj idx=\"1\" val=\"0.6\"/><dgm:adj idx=\"2\" val=\"0.5\"/></dgm:adjLst></dgm:shape></dgm:layoutNode>");
        s.push_str("</dgm:forEach></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn pyramid1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/pyramid1\">");
        s.push_str("<dgm:title val=\"Basic Pyramid\"/><dgm:layoutNode name=\"pyramid\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:choose name=\"direction\">");
        s.push_str("<dgm:if name=\"forward\" func=\"var\" arg=\"dir\" op=\"equ\" val=\"norm\"><dgm:alg type=\"pyra\"><dgm:param type=\"linDir\" val=\"fromT\"/>");
        s.push_str("<dgm:param type=\"pyraLvlNode\" val=\"level\"/></dgm:alg></dgm:if><dgm:else name=\"backward\"><dgm:alg type=\"pyra\"><dgm:param type=\"linDir\" val=\"fromB\"/>");
        s.push_str("<dgm:param type=\"pyraLvlNode\" val=\"level\"/></dgm:alg></dgm:else></dgm:choose><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"level\" op=\"equ\" val=\"65\"/></dgm:constrLst><dgm:forEach name=\"levels\" axis=\"ch\" ptType=\"node\">");
        s.push_str("<dgm:layoutNode name=\"level\" styleLbl=\"node1\"><dgm:varLst><dgm:bulletEnabled val=\"1\"/></dgm:varLst><dgm:alg type=\"tx\"/><dgm:shape type=\"trapezoid\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:forEach>");
        s.push_str("</dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn radial1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/radial1\">");
        s.push_str("<dgm:title val=\"Basic Radial\"/><dgm:layoutNode name=\"radial\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:alg type=\"cycle\">");
        s.push_str("<dgm:param type=\"stAng\" val=\"0\"/><dgm:param type=\"spanAng\" val=\"360\"/><dgm:param type=\"ctrShpMap\" val=\"fNode\"/></dgm:alg><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"centerShape\" refType=\"w\" fact=\"0.26\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"centerShape\" refType=\"w\" refFor=\"ch\" refForName=\"centerShape\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"node\" refType=\"w\" refFor=\"ch\" refForName=\"centerShape\" fact=\"0.85\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"node\" refType=\"w\" refFor=\"ch\" refForName=\"node\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"0.1\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"centerShape\" op=\"equ\" val=\"65\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"node\" op=\"equ\" val=\"65\"/></dgm:constrLst><dgm:forEach name=\"centre\" axis=\"ch\" ptType=\"node\" cnt=\"1\">");
        s.push_str("<dgm:layoutNode name=\"centerShape\" styleLbl=\"node0\"><dgm:alg type=\"tx\"/><dgm:shape type=\"ellipse\"/><dgm:presOf axis=\"self\"/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.15\"/><dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.15\"/><dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.08\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.08\"/></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode><dgm:forEach name=\"spokes\" axis=\"ch\" ptType=\"node\"><dgm:forEach name=\"spoke\" axis=\"precedSib\" ptType=\"parTrans\" cnt=\"1\">");
        s.push_str("<dgm:layoutNode name=\"parTrans\" styleLbl=\"parChTrans1D2\"><dgm:alg type=\"conn\"><dgm:param type=\"dim\" val=\"1D\"/><dgm:param type=\"endSty\" val=\"noArr\"/></dgm:alg>");
        s.push_str("<dgm:shape type=\"conn\"/></dgm:layoutNode></dgm:forEach><dgm:layoutNode name=\"node\" styleLbl=\"node1\"><dgm:varLst><dgm:bulletEnabled val=\"1\"/></dgm:varLst>");
        s.push_str("<dgm:alg type=\"tx\"/><dgm:shape type=\"ellipse\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.15\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.15\"/><dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.08\"/><dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.08\"/>");
        s.push_str("</dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:forEach></dgm:forEach>");
        s.push_str("</dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn target1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/target1\">");
        s.push_str("<dgm:title val=\"Basic Target\"/><dgm:layoutNode name=\"target\"><dgm:varLst><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:alg type=\"composite\"/><dgm:shape/>");
        s.push_str("<dgm:choose name=\"count\"><dgm:if name=\"n1\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"1\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring1\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.0588\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.147\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.147\"/>");
        s.push_str("</dgm:constrLst></dgm:if><dgm:if name=\"n2\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"2\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring1\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.0294\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.735\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.294\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.294\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.49\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.49\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring2\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.0294\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.0735\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.0735\"/>");
        s.push_str("</dgm:constrLst></dgm:if><dgm:if name=\"n3\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"3\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring1\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.0196\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.816667\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.196\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.196\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.653333\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.653333\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring2\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.0196\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.49\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.196\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.196\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.326667\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.326667\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring3\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.0196\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.049\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.049\"/>");
        s.push_str("</dgm:constrLst></dgm:if><dgm:if name=\"n4\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"4\"><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring1\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.0147\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.8575\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.147\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.147\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.735\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.735\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring2\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.0147\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.6125\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.147\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.147\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.49\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.49\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring3\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.0147\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.3675\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.147\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.147\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.245\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.245\"/>");
        s.push_str("<dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring4\" refType=\"w\" fact=\"0.5\"/><dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.0147\"/><dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.03675\"/><dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.03675\"/>");
        s.push_str("</dgm:constrLst></dgm:if><dgm:else name=\"n5\"><dgm:constrLst><dgm:constr type=\"w\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.98\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring1\" refType=\"w\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.5\"/><dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.01176\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.882\"/><dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.1176\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring1\" refType=\"h\" fact=\"0.1176\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.784\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.784\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring2\" refType=\"w\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.5\"/><dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.01176\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.686\"/><dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.1176\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring2\" refType=\"h\" fact=\"0.1176\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.588\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.588\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring3\" refType=\"w\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.5\"/><dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.01176\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.49\"/><dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.1176\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring3\" refType=\"h\" fact=\"0.1176\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.392\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.392\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring4\" refType=\"w\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.5\"/><dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.01176\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.294\"/><dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.1176\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring4\" refType=\"h\" fact=\"0.1176\"/><dgm:constr type=\"w\" for=\"ch\" forName=\"ring5\" refType=\"h\" fact=\"0.196\"/>");
        s.push_str("<dgm:constr type=\"h\" for=\"ch\" forName=\"ring5\" refType=\"h\" fact=\"0.196\"/><dgm:constr type=\"ctrX\" for=\"ch\" forName=\"ring5\" refType=\"w\" fact=\"0.5\"/>");
        s.push_str("<dgm:constr type=\"ctrY\" for=\"ch\" forName=\"ring5\" refType=\"h\" fact=\"0.5\"/><dgm:constr type=\"tMarg\" for=\"ch\" forName=\"ring5\" refType=\"h\" fact=\"0.01176\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" for=\"ch\" forName=\"ring5\" refType=\"h\" fact=\"0\"/><dgm:constr type=\"lMarg\" for=\"ch\" forName=\"ring5\" refType=\"h\" fact=\"0.0294\"/>");
        s.push_str("<dgm:constr type=\"rMarg\" for=\"ch\" forName=\"ring5\" refType=\"h\" fact=\"0.0294\"/></dgm:constrLst></dgm:else></dgm:choose><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"ch\" op=\"equ\" val=\"65\"/></dgm:constrLst><dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\" cnt=\"5\"><dgm:choose name=\"position\">");
        s.push_str("<dgm:if name=\"p1\" func=\"pos\" op=\"equ\" val=\"1\"><dgm:layoutNode name=\"ring1\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/>");
        s.push_str("</dgm:alg><dgm:shape type=\"ellipse\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:if name=\"p2\" func=\"pos\" op=\"equ\" val=\"2\">");
        s.push_str("<dgm:layoutNode name=\"ring2\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/></dgm:alg><dgm:shape type=\"ellipse\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:if name=\"p3\" func=\"pos\" op=\"equ\" val=\"3\"><dgm:layoutNode name=\"ring3\" styleLbl=\"node1\"><dgm:alg type=\"tx\">");
        s.push_str("<dgm:param type=\"txAnchorVert\" val=\"t\"/></dgm:alg><dgm:shape type=\"ellipse\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst></dgm:constrLst>");
        s.push_str("<dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:if name=\"p4\" func=\"pos\" op=\"equ\" val=\"4\">");
        s.push_str("<dgm:layoutNode name=\"ring4\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/></dgm:alg><dgm:shape type=\"ellipse\"/>");
        s.push_str("<dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst></dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst>");
        s.push_str("</dgm:layoutNode></dgm:if><dgm:else name=\"p5\"><dgm:layoutNode name=\"ring5\" styleLbl=\"node1\"><dgm:alg type=\"tx\"><dgm:param type=\"txAnchorVert\" val=\"t\"/>");
        s.push_str("</dgm:alg><dgm:shape type=\"ellipse\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:else></dgm:choose></dgm:forEach></dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn vList2() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/vList2\">");
        s.push_str("<dgm:title val=\"Vertical Bullet List\"/><dgm:layoutNode name=\"linear\"><dgm:varLst><dgm:animLvl val=\"lvl\"/><dgm:resizeHandles val=\"exact\"/></dgm:varLst>");
        s.push_str("<dgm:alg type=\"lin\"><dgm:param type=\"linDir\" val=\"fromT\"/><dgm:param type=\"nodeHorzAlign\" val=\"l\"/></dgm:alg><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"parentText\" refType=\"w\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"parentText\" refType=\"w\" fact=\"0.13\"/>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"childText\" refType=\"w\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"childText\" refType=\"w\" fact=\"0.16\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" fact=\"0.015\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"parentText\" op=\"equ\" val=\"65\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"childText\" op=\"equ\" val=\"65\"/>");
        s.push_str("<dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"childText\" refType=\"primFontSz\" refFor=\"ch\" refForName=\"parentText\" op=\"lte\" fact=\"0.78\"/></dgm:constrLst>");
        s.push_str("<dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\"><dgm:layoutNode name=\"parentText\" styleLbl=\"node1\"><dgm:alg type=\"tx\">");
        s.push_str("<dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"roundRect\"><dgm:adjLst><dgm:adj idx=\"1\" val=\"0.16667\"/></dgm:adjLst></dgm:shape>");
        s.push_str("<dgm:presOf axis=\"self\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.3\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode><dgm:choose name=\"hasPoints\">");
        s.push_str("<dgm:if name=\"some\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"gte\" val=\"1\"><dgm:layoutNode name=\"childText\" styleLbl=\"revTx\"><dgm:alg type=\"tx\">");
        s.push_str("<dgm:param type=\"txAnchorVert\" val=\"t\"/><dgm:param type=\"parTxLTRAlign\" val=\"l\"/></dgm:alg><dgm:shape type=\"rect\"/><dgm:presOf axis=\"des\" ptType=\"node\"/>");
        s.push_str("<dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"primFontSz\" fact=\"0.1\"/><dgm:constr type=\"bMarg\" refType=\"primFontSz\" fact=\"0.1\"/>");
        s.push_str("<dgm:constr type=\"lMarg\" refType=\"primFontSz\" fact=\"0.3\"/><dgm:constr type=\"rMarg\" refType=\"primFontSz\" fact=\"0.3\"/></dgm:constrLst><dgm:ruleLst>");
        s.push_str("<dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:if><dgm:else name=\"none\"/></dgm:choose></dgm:forEach>");
        s.push_str("</dgm:layoutNode></dgm:layoutDef>");
        s
    }
    pub fn venn1() -> String {
        let mut s: String = String::new();
        s.push_str("<dgm:layoutDef xmlns:dgm=\"http://schemas.openxmlformats.org/drawingml/2006/diagram\" uniqueId=\"urn:microsoft.com/office/officeart/2005/8/layout/venn1\">");
        s.push_str("<dgm:title val=\"Basic Venn\"/><dgm:layoutNode name=\"venn\"><dgm:varLst><dgm:dir/><dgm:resizeHandles val=\"exact\"/></dgm:varLst><dgm:choose name=\"two\">");
        s.push_str("<dgm:if name=\"side\" axis=\"ch\" ptType=\"node\" func=\"cnt\" op=\"equ\" val=\"2\"><dgm:alg type=\"cycle\"><dgm:param type=\"stAng\" val=\"270\"/>");
        s.push_str("<dgm:param type=\"spanAng\" val=\"360\"/></dgm:alg></dgm:if><dgm:else name=\"round\"><dgm:alg type=\"cycle\"><dgm:param type=\"stAng\" val=\"0\"/>");
        s.push_str("<dgm:param type=\"spanAng\" val=\"360\"/></dgm:alg></dgm:else></dgm:choose><dgm:shape/><dgm:constrLst>");
        s.push_str("<dgm:constr type=\"w\" for=\"ch\" forName=\"node\" refType=\"w\" fact=\"0.34\"/><dgm:constr type=\"h\" for=\"ch\" forName=\"node\" refType=\"w\" refFor=\"ch\" refForName=\"node\"/>");
        s.push_str("<dgm:constr type=\"sibSp\" refType=\"w\" refFor=\"ch\" refForName=\"node\" fact=\"-0.3\"/><dgm:constr type=\"primFontSz\" for=\"ch\" forName=\"node\" op=\"equ\" val=\"65\"/>");
        s.push_str("</dgm:constrLst><dgm:forEach name=\"items\" axis=\"ch\" ptType=\"node\"><dgm:layoutNode name=\"node\" styleLbl=\"vennNode1\"><dgm:alg type=\"tx\"></dgm:alg>");
        s.push_str("<dgm:shape type=\"ellipse\"/><dgm:presOf axis=\"desOrSelf\" ptType=\"node\"/><dgm:constrLst><dgm:constr type=\"tMarg\" refType=\"w\" fact=\"0.25\"/>");
        s.push_str("<dgm:constr type=\"bMarg\" refType=\"w\" fact=\"0.25\"/><dgm:constr type=\"lMarg\" refType=\"w\" fact=\"0.25\"/><dgm:constr type=\"rMarg\" refType=\"w\" fact=\"0.25\"/>");
        s.push_str("</dgm:constrLst><dgm:ruleLst><dgm:rule type=\"primFontSz\" val=\"5\" fact=\"NaN\" max=\"NaN\"/></dgm:ruleLst></dgm:layoutNode></dgm:forEach></dgm:layoutNode>");
        s.push_str("</dgm:layoutDef>");
        s
    }
}
