use serde::{Deserialize, Serialize};
use std::thread::sleep;
use std::time::Duration;
use windows::core::{Interface, BSTR};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationElementArray,
    IUIAutomationExpandCollapsePattern, IUIAutomationInvokePattern,
    IUIAutomationScrollPattern, IUIAutomationSelectionItemPattern,
    IUIAutomationTextPattern, IUIAutomationValuePattern, ScrollAmount_NoAmount,
    ScrollAmount_SmallDecrement, ScrollAmount_SmallIncrement,
    TextPatternRangeEndpoint_End, TextPatternRangeEndpoint_Start, TreeScope_Children,
    TreeScope_Descendants,
    UIA_ButtonControlTypeId, UIA_CheckBoxControlTypeId, UIA_ComboBoxControlTypeId,
    UIA_CustomControlTypeId, UIA_DocumentControlTypeId, UIA_EditControlTypeId,
    UIA_ExpandCollapsePatternId, UIA_GroupControlTypeId, UIA_HyperlinkControlTypeId,
    UIA_ImageControlTypeId, UIA_InvokePatternId, UIA_ListControlTypeId,
    UIA_ListItemControlTypeId, UIA_MenuControlTypeId, UIA_MenuItemControlTypeId,
    UIA_PaneControlTypeId, UIA_ProgressBarControlTypeId, UIA_RadioButtonControlTypeId,
    UIA_ScrollPatternId, UIA_SelectionItemPatternId, UIA_SliderControlTypeId,
    UIA_TabControlTypeId, UIA_TabItemControlTypeId, UIA_TextControlTypeId,
    UIA_TextPatternId, UIA_TreeControlTypeId, UIA_TreeItemControlTypeId,
    UIA_ValuePatternId, UIA_WindowControlTypeId,
};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementContent {
    pub value: Option<String>,
    pub text: Option<String>,
    pub is_read_only: bool,
    pub pattern_used: Option<String>,
    pub selected_text: Option<String>,
    pub caret_position: Option<usize>,
    pub selection_length: usize,
    pub has_keyboard_focus: bool,
    pub is_keyboard_focusable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollInfo {
    pub horizontally_scrollable: bool,
    pub vertically_scrollable: bool,
    pub horizontal_percent: f64,
    pub vertical_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusedControlDetails {
    pub window_hwnd: i64,
    pub window_title: String,
    pub window_process: String,
    pub is_window_active: bool,
    pub control_name: String,
    pub control_type: String,
    pub automation_id: String,
    pub class_name: String,
    pub is_enabled: bool,
    pub has_keyboard_focus: bool,
    pub is_keyboard_focusable: bool,
    pub bounding_rect: Option<super::windows::WindowRect>,
    pub content: ElementContent,
    pub scroll: Option<ScrollInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiElementNode {
    pub name: String,
    pub control_type: String,
    pub automation_id: String,
    pub class_name: String,
    pub is_enabled: bool,
    pub has_focus: bool,
    pub is_read_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caret_position: Option<usize>,
    pub bounding_rect: Option<super::windows::WindowRect>,
    pub children: Vec<UiElementNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiInspectResult {
    pub window_title: String,
    pub window_hwnd: i64,
    pub element_tree: UiElementNode,
    pub total_elements_found: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementMatch {
    pub name: String,
    pub control_type: String,
    pub automation_id: String,
    pub is_enabled: bool,
    pub has_focus: bool,
    pub is_read_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caret_position: Option<usize>,
    pub bounding_rect: Option<super::windows::WindowRect>,
}

/// Retrieve text value using ValuePattern, document text using TextPattern,
/// and selection/caret information using appropriate APIs where supported.
pub fn get_element_content(element: &IUIAutomationElement) -> ElementContent {
    let (val, is_ro, val_pattern_found) = unsafe {
        if let Ok(pattern_obj) = element.GetCurrentPattern(UIA_ValuePatternId) {
            if let Ok(val_pattern) = pattern_obj.cast::<IUIAutomationValuePattern>() {
                let v = val_pattern.CurrentValue().map(|b| b.to_string()).ok();
                let ro = val_pattern.CurrentIsReadOnly().map(|b| b.as_bool()).unwrap_or(false);
                (v, ro, true)
            } else {
                (None, false, false)
            }
        } else {
            (None, false, false)
        }
    };

    let (doc_text, sel_text, caret_pos, sel_len, text_pattern_found) = unsafe {
        if let Ok(pattern_obj) = element.GetCurrentPattern(UIA_TextPatternId) {
            if let Ok(tp) = pattern_obj.cast::<IUIAutomationTextPattern>() {
                let doc_txt = tp.DocumentRange().and_then(|dr| dr.GetText(-1)).map(|b| b.to_string()).ok();
                let (s_txt, c_pos, s_len) = if let Ok(sel_array) = tp.GetSelection() {
                    if sel_array.Length().unwrap_or(0) > 0 {
                        if let Ok(sel_range) = sel_array.GetElement(0) {
                            let selected = sel_range.GetText(-1).map(|b| b.to_string()).unwrap_or_default();
                            let len = selected.chars().count();
                            let caret = if let Ok(doc_range) = tp.DocumentRange() {
                                let mut pre_range = doc_range.Clone().ok();
                                if let Some(ref mut pr) = pre_range {
                                    if pr.MoveEndpointByRange(TextPatternRangeEndpoint_End, &sel_range, TextPatternRangeEndpoint_Start).is_ok() {
                                        pr.GetText(-1).map(|b| b.to_string().chars().count()).ok()
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            };
                            (if selected.is_empty() { None } else { Some(selected) }, caret, len)
                        } else {
                            (None, None, 0)
                        }
                    } else {
                        (None, None, 0)
                    }
                } else {
                    (None, None, 0)
                };
                (doc_txt, s_txt, c_pos, s_len, true)
            } else {
                (None, None, None, 0, false)
            }
        } else {
            (None, None, None, 0, false)
        }
    };

    let has_keyboard_focus = unsafe { element.CurrentHasKeyboardFocus().map(|b| b.as_bool()).unwrap_or(false) };
    let is_keyboard_focusable = unsafe { element.CurrentIsKeyboardFocusable().map(|b| b.as_bool()).unwrap_or(false) };

    let pattern_used = if val_pattern_found && text_pattern_found {
        Some("ValuePattern+TextPattern".to_string())
    } else if val_pattern_found {
        Some("ValuePattern".to_string())
    } else if text_pattern_found {
        Some("TextPattern".to_string())
    } else {
        None
    };

    ElementContent {
        value: val,
        text: doc_text,
        is_read_only: is_ro,
        pattern_used,
        selected_text: sel_text,
        caret_position: caret_pos,
        selection_length: sel_len,
        has_keyboard_focus,
        is_keyboard_focusable,
    }
}

pub fn get_element_scroll(element: &IUIAutomationElement) -> Option<ScrollInfo> {
    unsafe {
        if let Ok(pattern_obj) = element.GetCurrentPattern(UIA_ScrollPatternId) {
            if let Ok(sp) = pattern_obj.cast::<IUIAutomationScrollPattern>() {
                let h_scrollable = sp.CurrentHorizontallyScrollable().map(|b| b.as_bool()).unwrap_or(false);
                let v_scrollable = sp.CurrentVerticallyScrollable().map(|b| b.as_bool()).unwrap_or(false);
                let h_pct = sp.CurrentHorizontalScrollPercent().unwrap_or(-1.0);
                let v_pct = sp.CurrentVerticalScrollPercent().unwrap_or(-1.0);
                return Some(ScrollInfo {
                    horizontally_scrollable: h_scrollable,
                    vertically_scrollable: v_scrollable,
                    horizontal_percent: h_pct,
                    vertical_percent: v_pct,
                });
            }
        }
        None
    }
}

fn init_com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn get_automation() -> Result<IUIAutomation, String> {
    init_com();
    unsafe {
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("Failed to create IUIAutomation instance: {e}"))
    }
}

pub fn control_type_to_string(type_id: i32) -> &'static str {
    match type_id {
        x if x == UIA_ButtonControlTypeId.0 as i32 => "Button",
        x if x == UIA_EditControlTypeId.0 as i32 => "Edit",
        x if x == UIA_TextControlTypeId.0 as i32 => "Text",
        x if x == UIA_WindowControlTypeId.0 as i32 => "Window",
        x if x == UIA_PaneControlTypeId.0 as i32 => "Pane",
        x if x == UIA_MenuControlTypeId.0 as i32 => "Menu",
        x if x == UIA_MenuItemControlTypeId.0 as i32 => "MenuItem",
        x if x == UIA_TabControlTypeId.0 as i32 => "Tab",
        x if x == UIA_TabItemControlTypeId.0 as i32 => "TabItem",
        x if x == UIA_ComboBoxControlTypeId.0 as i32 => "ComboBox",
        x if x == UIA_CheckBoxControlTypeId.0 as i32 => "CheckBox",
        x if x == UIA_RadioButtonControlTypeId.0 as i32 => "RadioButton",
        x if x == UIA_ListControlTypeId.0 as i32 => "List",
        x if x == UIA_ListItemControlTypeId.0 as i32 => "ListItem",
        x if x == UIA_DocumentControlTypeId.0 as i32 => "Document",
        x if x == UIA_HyperlinkControlTypeId.0 as i32 => "Hyperlink",
        x if x == UIA_ImageControlTypeId.0 as i32 => "Image",
        x if x == UIA_ProgressBarControlTypeId.0 as i32 => "ProgressBar",
        x if x == UIA_SliderControlTypeId.0 as i32 => "Slider",
        x if x == UIA_TreeControlTypeId.0 as i32 => "Tree",
        x if x == UIA_TreeItemControlTypeId.0 as i32 => "TreeItem",
        x if x == UIA_GroupControlTypeId.0 as i32 => "Group",
        x if x == UIA_CustomControlTypeId.0 as i32 => "Custom",
        _ => "Unknown",
    }
}

fn extract_node(
    uia: &IUIAutomation,
    element: &IUIAutomationElement,
    current_depth: u32,
    max_depth: u32,
    total_count: &mut usize,
) -> UiElementNode {
    *total_count += 1;

    let name = unsafe {
        element
            .CurrentName()
            .map(|b| b.to_string())
            .unwrap_or_default()
    };

    let control_type_id = unsafe { element.CurrentControlType().map(|id| id.0 as i32).unwrap_or(0) };
    let control_type = control_type_to_string(control_type_id).to_string();

    let automation_id = unsafe {
        element
            .CurrentAutomationId()
            .map(|b| b.to_string())
            .unwrap_or_default()
    };

    let class_name = unsafe {
        element
            .CurrentClassName()
            .map(|b| b.to_string())
            .unwrap_or_default()
    };

    let is_enabled = unsafe { element.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(false) };

    let bounding_rect = unsafe {
        element.CurrentBoundingRectangle().ok().and_then(|r| {
            let w = r.right - r.left;
            let h = r.bottom - r.top;
            if w > 0 && h > 0 {
                Some(super::windows::WindowRect {
                    x: r.left,
                    y: r.top,
                    width: w,
                    height: h,
                })
            } else {
                None
            }
        })
    };

    let is_textual = control_type == "Edit"
        || control_type == "Document"
        || control_type == "Text"
        || control_type == "ComboBox";

    let (content_val, content_txt, content_sel, content_caret, has_focus, is_ro) = if is_textual {
        let c = get_element_content(element);
        (c.value, c.text, c.selected_text, c.caret_position, c.has_keyboard_focus, c.is_read_only)
    } else {
        let hf = unsafe { element.CurrentHasKeyboardFocus().map(|b| b.as_bool()).unwrap_or(false) };
        (None, None, None, None, hf, false)
    };

    let mut children_nodes = Vec::new();
    if current_depth < max_depth {
        unsafe {
            if let Ok(cond) = uia.CreateTrueCondition() {
                if let Ok(children_arr) = element.FindAll(TreeScope_Children, &cond) {
                    let len = children_arr.Length().unwrap_or(0);
                    // Bounded slice of children to keep AI payload concise
                    let max_children = 25.min(len);
                    for i in 0..max_children {
                        if let Ok(child) = children_arr.GetElement(i) {
                            let child_node =
                                extract_node(uia, &child, current_depth + 1, max_depth, total_count);
                            // Only include children that have meaningful content
                            if !child_node.name.is_empty()
                                || !child_node.automation_id.is_empty()
                                || child_node.control_type == "Button"
                                || child_node.control_type == "Edit"
                                || child_node.control_type == "Document"
                                || child_node.control_type == "MenuItem"
                                || child_node.control_type == "TabItem"
                                || child_node.value.is_some()
                                || child_node.text.is_some()
                                || !child_node.children.is_empty()
                            {
                                children_nodes.push(child_node);
                            }
                        }
                    }
                }
            }
        }
    }

    UiElementNode {
        name,
        control_type,
        automation_id,
        class_name,
        is_enabled,
        has_focus,
        is_read_only: is_ro,
        value: content_val,
        text: content_txt,
        selected_text: content_sel,
        caret_position: content_caret,
        bounding_rect,
        children: children_nodes,
    }
}

/// Inspect the UI Accessibility tree of foreground window or selected HWND
pub fn inspect_window_tree(
    target_hwnd: Option<i64>,
    max_depth: Option<u32>,
) -> Result<UiInspectResult, String> {
    let uia = get_automation()?;

    let hwnd = match target_hwnd {
        Some(h) => HWND(h as *mut _),
        None => {
            let fg = unsafe { GetForegroundWindow() };
            if fg.0.is_null() {
                return Err("No active foreground window found".to_string());
            }
            fg
        }
    };

    let root_element: IUIAutomationElement = unsafe {
        uia.ElementFromHandle(hwnd)
            .map_err(|e| format!("Failed to get IUIAutomationElement from HWND: {e}"))?
    };

    let window_title = unsafe {
        root_element
            .CurrentName()
            .map(|b| b.to_string())
            .unwrap_or_else(|_| "Window".to_string())
    };

    let depth = max_depth.unwrap_or(3).clamp(1, 5);
    let mut total_count = 0;
    let tree = extract_node(&uia, &root_element, 0, depth, &mut total_count);

    Ok(UiInspectResult {
        window_title,
        window_hwnd: hwnd.0 as i64,
        element_tree: tree,
        total_elements_found: total_count,
    })
}

/// Search for controls matching name, control type, or automation ID
pub fn find_elements(
    target_hwnd: Option<i64>,
    query: Option<&str>,
    control_type: Option<&str>,
    max_results: Option<usize>,
) -> Result<Vec<ElementMatch>, String> {
    let uia = get_automation()?;

    let hwnd = match target_hwnd {
        Some(h) => HWND(h as *mut _),
        None => unsafe { GetForegroundWindow() },
    };

    if hwnd.0.is_null() {
        return Err("No valid window target found".to_string());
    }

    let root: IUIAutomationElement = unsafe {
        uia.ElementFromHandle(hwnd)
            .map_err(|e| format!("Failed to get element from handle: {e}"))?
    };

    let cond = unsafe {
        uia.CreateTrueCondition()
            .map_err(|e| format!("Failed to create condition: {e}"))?
    };

    let arr: IUIAutomationElementArray = unsafe {
        root.FindAll(TreeScope_Descendants, &cond)
            .map_err(|e| format!("FindAll failed: {e}"))?
    };

    let total = unsafe { arr.Length().unwrap_or(0) };
    let limit = max_results.unwrap_or(20).clamp(1, 50);

    let mut matches = Vec::new();
    let q_lower = query.map(|q| q.to_lowercase());
    let ct_lower = control_type.map(|ct| ct.to_lowercase());

    for i in 0..total {
        if matches.len() >= limit {
            break;
        }

        unsafe {
            if let Ok(elem) = arr.GetElement(i) {
                let name = elem.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                let type_id = elem.CurrentControlType().map(|id| id.0 as i32).unwrap_or(0);
                let ct_str = control_type_to_string(type_id);
                let auto_id = elem.CurrentAutomationId().map(|b| b.to_string()).unwrap_or_default();

                let mut matched = true;

                if let Some(ref q) = q_lower {
                    let name_match = name.to_lowercase().contains(q);
                    let id_match = auto_id.to_lowercase().contains(q);
                    if !name_match && !id_match {
                        matched = false;
                    }
                }

                if let Some(ref req_ct) = ct_lower {
                    if !ct_str.to_lowercase().contains(req_ct) {
                        matched = false;
                    }
                }

                if matched && (!name.is_empty() || !auto_id.is_empty()) {
                    let is_enabled = elem.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(false);
                    let content = get_element_content(&elem);
                    let bounding_rect = elem.CurrentBoundingRectangle().ok().and_then(|r| {
                        let w = r.right - r.left;
                        let h = r.bottom - r.top;
                        if w > 0 && h > 0 {
                            Some(super::windows::WindowRect {
                                x: r.left,
                                y: r.top,
                                width: w,
                                height: h,
                            })
                        } else {
                            None
                        }
                    });

                    matches.push(ElementMatch {
                        name,
                        control_type: ct_str.to_string(),
                        automation_id: auto_id,
                        is_enabled,
                        has_focus: content.has_keyboard_focus,
                        is_read_only: content.is_read_only,
                        value: content.value,
                        text: content.text,
                        selected_text: content.selected_text,
                        caret_position: content.caret_position,
                        bounding_rect,
                    });
                }
            }
        }
    }

    Ok(matches)
}

/// Find a specific element and click / invoke it
pub fn invoke_element(
    target_hwnd: Option<i64>,
    query: &str,
    control_type: Option<&str>,
) -> Result<String, String> {
    let uia = get_automation()?;

    let hwnd = match target_hwnd {
        Some(h) => HWND(h as *mut _),
        None => unsafe { GetForegroundWindow() },
    };

    if hwnd.0.is_null() {
        return Err("No active window to invoke control on".to_string());
    }

    let root: IUIAutomationElement = unsafe {
        uia.ElementFromHandle(hwnd)
            .map_err(|e| format!("ElementFromHandle failed: {e}"))?
    };

    let (target_elem, elem_name, rect_opt) = find_single_element_internal(&root, query, control_type)?;

    // Attempt 1: Semantic Invoke pattern
    unsafe {
        if let Ok(pattern_obj) = target_elem.GetCurrentPattern(UIA_InvokePatternId) {
            if let Ok(invoke) = pattern_obj.cast::<IUIAutomationInvokePattern>() {
                if invoke.Invoke().is_ok() {
                    return Ok(format!("Successfully invoked control '{elem_name}' via UIA InvokePattern"));
                }
            }
        }

        // Attempt 2: SelectionItem pattern
        if let Ok(pattern_obj) = target_elem.GetCurrentPattern(UIA_SelectionItemPatternId) {
            if let Ok(sel) = pattern_obj.cast::<IUIAutomationSelectionItemPattern>() {
                if sel.Select().is_ok() {
                    return Ok(format!("Successfully selected control '{elem_name}' via UIA SelectionItemPattern"));
                }
            }
        }
    }

    // Attempt 3: If pattern unsupported or failed, click center of bounding rect
    if let Some(r) = rect_opt {
        let center_x = r.left + (r.right - r.left) / 2;
        let center_y = r.top + (r.bottom - r.top) / 2;
        super::input::click_mouse("left", Some(center_x), Some(center_y))?;
        return Ok(format!(
            "Clicked control '{elem_name}' at center coordinates ({center_x}, {center_y})"
        ));
    }

    Err(format!(
        "Control '{elem_name}' found, but does not support Invoke/Select and has no clickable bounds"
    ))
}

/// Set text value of a control via ValuePattern or focus + type, with pre-read and post-verification
pub fn set_element_value(
    target_hwnd: Option<i64>,
    query: &str,
    value: &str,
) -> Result<serde_json::Value, String> {
    let uia = get_automation()?;

    let hwnd = match target_hwnd {
        Some(h) => HWND(h as *mut _),
        None => unsafe { GetForegroundWindow() },
    };

    if hwnd.0.is_null() {
        return Err("No active window found".to_string());
    }

    let root: IUIAutomationElement = unsafe {
        uia.ElementFromHandle(hwnd)
            .map_err(|e| format!("ElementFromHandle failed: {e}"))?
    };

    let (target_elem, elem_name, rect_opt) = find_single_element_internal(&root, query, Some("Edit"))
        .or_else(|_| find_single_element_internal(&root, query, Some("Document")))
        .or_else(|_| find_single_element_internal(&root, query, None))?;

    // Pre-inspection: Check current value / text to prevent redundant writes
    let pre_content = get_element_content(&target_elem);
    let pre_text = pre_content.value.as_ref().or(pre_content.text.as_ref());
    if let Some(current) = pre_text {
        if current == value {
            return Ok(serde_json::json!({
                "status": "success",
                "verified": true,
                "message": format!("Control '{elem_name}' already contains the requested value"),
                "control_name": elem_name,
                "previous_value": current,
                "current_value": current,
            }));
        }
    }

    // Attempt 1: Semantic ValuePattern::SetValue
    let mut set_via_value_pattern = false;
    unsafe {
        if let Ok(pattern_obj) = target_elem.GetCurrentPattern(UIA_ValuePatternId) {
            if let Ok(val_pattern) = pattern_obj.cast::<IUIAutomationValuePattern>() {
                let is_ro = val_pattern.CurrentIsReadOnly().map(|b| b.as_bool()).unwrap_or(false);
                if !is_ro {
                    let bstr_val = BSTR::from(value);
                    if val_pattern.SetValue(&bstr_val).is_ok() {
                        set_via_value_pattern = true;
                    }
                }
            }
        }
    }

    if set_via_value_pattern {
        // Post-verification
        sleep(Duration::from_millis(40));
        let post_content = get_element_content(&target_elem);
        let post_text = post_content.value.as_ref().or(post_content.text.as_ref()).cloned();
        let matches = post_text.as_deref() == Some(value);
        return Ok(serde_json::json!({
            "status": if matches { "success" } else { "verification_warning" },
            "verified": matches,
            "method": "ValuePattern",
            "control_name": elem_name,
            "previous_value": pre_text,
            "current_value": post_text,
        }));
    }

    // Attempt 2: SetFocus + select all + type Unicode value
    let _ = unsafe { target_elem.SetFocus() };
    sleep(Duration::from_millis(30));

    let has_focus = unsafe { target_elem.CurrentHasKeyboardFocus().map(|b| b.as_bool()).unwrap_or(false) };
    if !has_focus {
        if let Some(r) = rect_opt {
            let center_x = r.left + (r.right - r.left) / 2;
            let center_y = r.top + (r.bottom - r.top) / 2;
            super::input::click_mouse("left", Some(center_x), Some(center_y))?;
            sleep(Duration::from_millis(50));
        }
    }

    let _ = super::input::select_all();
    sleep(Duration::from_millis(25));
    let _ = super::input::clear_field();
    sleep(Duration::from_millis(25));
    super::input::type_unicode_text(value)?;
    sleep(Duration::from_millis(60));

    // Post-verification
    let post_content = get_element_content(&target_elem);
    let post_text = post_content.value.as_ref().or(post_content.text.as_ref()).cloned();
    let matches = post_text.as_deref().map(|t| t.contains(value)).unwrap_or(false)
        || post_text.as_deref() == Some(value);

    Ok(serde_json::json!({
        "status": if matches { "success" } else { "verification_warning" },
        "verified": matches,
        "method": "FocusAndType",
        "control_name": elem_name,
        "previous_value": pre_text,
        "current_value": post_text,
    }))
}

/// Expand or collapse a control (e.g. ComboBox, TreeViewItem, Expander)
pub fn expand_collapse_element(
    target_hwnd: Option<i64>,
    query: &str,
    expand: bool,
) -> Result<String, String> {
    let uia = get_automation()?;

    let hwnd = match target_hwnd {
        Some(h) => HWND(h as *mut _),
        None => unsafe { GetForegroundWindow() },
    };

    if hwnd.0.is_null() {
        return Err("No active window found".to_string());
    }

    let root: IUIAutomationElement = unsafe {
        uia.ElementFromHandle(hwnd)
            .map_err(|e| format!("ElementFromHandle failed: {e}"))?
    };

    let (target_elem, elem_name, _) = find_single_element_internal(&root, query, None)?;

    unsafe {
        if let Ok(pattern_obj) = target_elem.GetCurrentPattern(UIA_ExpandCollapsePatternId) {
            if let Ok(ec_pattern) = pattern_obj.cast::<IUIAutomationExpandCollapsePattern>() {
                if expand {
                    ec_pattern
                        .Expand()
                        .map_err(|e| format!("Expand failed: {e}"))?;
                    return Ok(format!("Expanded control '{elem_name}'"));
                } else {
                    ec_pattern
                        .Collapse()
                        .map_err(|e| format!("Collapse failed: {e}"))?;
                    return Ok(format!("Collapsed control '{elem_name}'"));
                }
            }
        }
    }

    Err(format!(
        "Control '{elem_name}' does not support ExpandCollapsePattern"
    ))
}

/// Internal helper to find a single element matching query
fn find_single_element_internal(
    root: &IUIAutomationElement,
    query: &str,
    control_type: Option<&str>,
) -> Result<(IUIAutomationElement, String, Option<RECT>), String> {
    let uia = get_automation()?;
    let cond = unsafe {
        uia.CreateTrueCondition()
            .map_err(|e| format!("CreateTrueCondition failed: {e}"))?
    };

    let arr: IUIAutomationElementArray = unsafe {
        root.FindAll(TreeScope_Descendants, &cond)
            .map_err(|e| format!("FindAll failed: {e}"))?
    };

    let total = unsafe { arr.Length().unwrap_or(0) };
    let q_lower = query.to_lowercase();
    let ct_lower = control_type.map(|ct| ct.to_lowercase());

    for i in 0..total {
        unsafe {
            if let Ok(elem) = arr.GetElement(i) {
                let name = elem.CurrentName().map(|b| b.to_string()).unwrap_or_default();
                let type_id = elem.CurrentControlType().map(|id| id.0 as i32).unwrap_or(0);
                let ct_str = control_type_to_string(type_id);
                let auto_id = elem.CurrentAutomationId().map(|b| b.to_string()).unwrap_or_default();

                let mut matched = false;
                if name.to_lowercase().contains(&q_lower) || auto_id.to_lowercase().contains(&q_lower) {
                    matched = true;
                }

                if let Some(ref req_ct) = ct_lower {
                    if !ct_str.to_lowercase().contains(req_ct) {
                        matched = false;
                    }
                }

                if matched {
                    let rect_opt = elem.CurrentBoundingRectangle().ok();
                    return Ok((elem, name.is_empty().then(|| auto_id.clone()).unwrap_or(name), rect_opt));
                }
            }
        }
    }

    Err(format!(
        "Control matching query '{query}' (type: {:?}) not found",
        control_type
    ))
}

/// Inspect the currently focused element on the active desktop
pub fn get_focused_element() -> Result<UiElementNode, String> {
    init_com();
    let uia = get_automation()?;
    let focused = unsafe {
        uia.GetFocusedElement()
            .map_err(|e| format!("Failed to retrieve focused element: {e}"))?
    };

    let mut total_count = 0;
    Ok(extract_node(&uia, &focused, 0, 1, &mut total_count))
}

/// Select an item control using IUIAutomationSelectionItemPattern or center-click fallback
pub fn select_element_item(target_hwnd: Option<i64>, query: &str) -> Result<String, String> {
    let uia = get_automation()?;

    let hwnd = match target_hwnd {
        Some(h) => HWND(h as *mut _),
        None => unsafe { GetForegroundWindow() },
    };

    if hwnd.0.is_null() {
        return Err("No active window found".to_string());
    }

    let root: IUIAutomationElement = unsafe {
        uia.ElementFromHandle(hwnd)
            .map_err(|e| format!("ElementFromHandle failed: {e}"))?
    };

    let (target_elem, elem_name, rect_opt) = find_single_element_internal(&root, query, None)?;

    unsafe {
        if let Ok(pattern_obj) = target_elem.GetCurrentPattern(UIA_SelectionItemPatternId) {
            if let Ok(sel_pattern) = pattern_obj.cast::<IUIAutomationSelectionItemPattern>() {
                sel_pattern
                    .Select()
                    .map_err(|e| format!("SelectionItem.Select failed: {e}"))?;
                return Ok(format!("Selected item '{elem_name}'"));
            }
        }
    }

    // Fallback: click element center
    if let Some(rect) = rect_opt {
        let cx = rect.left + (rect.right - rect.left) / 2;
        let cy = rect.top + (rect.bottom - rect.top) / 2;
        super::input::click_mouse("left", Some(cx), Some(cy))?;
        return Ok(format!("Selected item '{elem_name}' via center click at ({cx}, {cy})"));
    }

    Err(format!("Could not select control '{elem_name}'"))
}

fn find_focused_or_editor_inside(uia: &IUIAutomation, root: &IUIAutomationElement) -> Option<IUIAutomationElement> {
    unsafe {
        let cond = uia.CreateTrueCondition().ok()?;
        let arr = root.FindAll(TreeScope_Descendants, &cond).ok()?;
        let len = arr.Length().unwrap_or(0);

        // First pass: look for element with keyboard focus
        for i in 0..len {
            if let Ok(elem) = arr.GetElement(i) {
                if elem.CurrentHasKeyboardFocus().map(|b| b.as_bool()).unwrap_or(false) {
                    return Some(elem);
                }
            }
        }

        // Second pass: look for Document or Edit control
        for i in 0..len {
            if let Ok(elem) = arr.GetElement(i) {
                let ct = elem.CurrentControlType().map(|id| id.0 as i32).unwrap_or(0);
                if ct == UIA_DocumentControlTypeId.0 as i32 || ct == UIA_EditControlTypeId.0 as i32 {
                    let _ = elem.SetFocus();
                    return Some(elem);
                }
            }
        }
    }
    None
}

/// Retrieve comprehensive details on the currently focused control across the desktop or target window
pub fn get_focused_control_details_for_target(target_window_id: Option<i64>) -> Result<FocusedControlDetails, String> {
    init_com();
    let uia = get_automation()?;

    let focused = unsafe {
        let target_h = target_window_id.filter(|&h| h > 0).map(|h| HWND(h as *mut _));
        if let Some(th) = target_h {
            if let Ok(root) = uia.ElementFromHandle(th) {
                find_focused_or_editor_inside(&uia, &root).unwrap_or(root)
            } else {
                uia.GetFocusedElement().map_err(|e| format!("Failed to retrieve focused element: {e}"))?
            }
        } else {
            uia.GetFocusedElement()
                .or_else(|_| {
                    let fg = GetForegroundWindow();
                    if !fg.0.is_null() {
                        uia.ElementFromHandle(fg)
                    } else {
                        uia.GetRootElement()
                    }
                })
                .map_err(|e| format!("Failed to retrieve focused element: {e}"))?
        }
    };

    let control_type_id = unsafe { focused.CurrentControlType().map(|id| id.0 as i32).unwrap_or(0) };
    let control_type = control_type_to_string(control_type_id).to_string();
    let control_name = unsafe { focused.CurrentName().map(|b| b.to_string()).unwrap_or_default() };
    let automation_id = unsafe { focused.CurrentAutomationId().map(|b| b.to_string()).unwrap_or_default() };
    let class_name = unsafe { focused.CurrentClassName().map(|b| b.to_string()).unwrap_or_default() };
    let is_enabled = unsafe { focused.CurrentIsEnabled().map(|b| b.as_bool()).unwrap_or(false) };
    let has_keyboard_focus = unsafe { focused.CurrentHasKeyboardFocus().map(|b| b.as_bool()).unwrap_or(false) };
    let is_keyboard_focusable = unsafe { focused.CurrentIsKeyboardFocusable().map(|b| b.as_bool()).unwrap_or(false) };

    let bounding_rect = unsafe {
        focused.CurrentBoundingRectangle().ok().and_then(|r| {
            let w = r.right - r.left;
            let h = r.bottom - r.top;
            if w > 0 && h > 0 {
                Some(super::windows::WindowRect { x: r.left, y: r.top, width: w, height: h })
            } else {
                None
            }
        })
    };

    let fg_hwnd = unsafe { GetForegroundWindow() };
    let window_hwnd = if target_window_id.unwrap_or(0) > 0 {
        target_window_id.unwrap()
    } else if !fg_hwnd.0.is_null() {
        fg_hwnd.0 as i64
    } else {
        unsafe { focused.CurrentNativeWindowHandle().map(|h| h.0 as i64).unwrap_or(0) }
    };

    let (window_title, window_process) = super::windows::list_windows(false, None, None)
        .ok()
        .and_then(|wins| wins.into_iter().find(|w| w.hwnd == window_hwnd))
        .map(|w| (w.title, w.process_name))
        .unwrap_or_else(|| ("Active Window".to_string(), "Unknown".to_string()));

    let content = get_element_content(&focused);
    let scroll = get_element_scroll(&focused);

    Ok(FocusedControlDetails {
        window_hwnd,
        window_title,
        window_process,
        is_window_active: window_hwnd != 0,
        control_name,
        control_type,
        automation_id,
        class_name,
        is_enabled,
        has_keyboard_focus,
        is_keyboard_focusable,
        bounding_rect,
        content,
        scroll,
    })
}

pub fn get_focused_control_details() -> Result<FocusedControlDetails, String> {
    get_focused_control_details_for_target(None)
}

/// Prepare target window and control for typing: focus window, verify focus, inspect target state
pub fn prepare_target_for_typing(
    target_window_id: Option<i64>,
    target_element_query: Option<&str>,
    target_control_type: Option<&str>,
) -> Result<FocusedControlDetails, String> {
    // 1. If target_window_id is specified (> 0), ensure it has foreground focus
    if let Some(hwnd) = target_window_id {
        if hwnd > 0 {
            let target_h = HWND(hwnd as *mut _);
            let mut target_pid = 0u32;
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(target_h, Some(&mut target_pid));
            }

            super::windows::focus_window(hwnd)?;
            // Verify window focus
            let mut focused = false;
            for _ in 0..25 {
                let fg = unsafe { GetForegroundWindow() };
                if fg.0 as i64 == hwnd || fg == target_h {
                    focused = true;
                    break;
                }
                if target_pid != 0 {
                    let mut fg_pid = 0u32;
                    unsafe {
                        let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(fg, Some(&mut fg_pid));
                    }
                    if fg_pid != 0 && fg_pid == target_pid {
                        focused = true;
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            if !focused {
                // If GetForegroundWindow was 0 (background console / test session), verify if target window can be accessed and focused via UI Automation
                let uia = get_automation()?;
                if let Ok(elem) = unsafe { uia.ElementFromHandle(target_h) } {
                    let _ = unsafe { elem.SetFocus() };
                    focused = true;
                }
            }
            if !focused {
                let current_fg = unsafe { GetForegroundWindow() };
                return Err(format!(
                    "Target window (hwnd: {hwnd}) did not gain foreground focus. Current foreground HWND is {}",
                    current_fg.0 as i64
                ));
            }
        }
    }

    // 2. If target_element_query is specified, find and focus the element
    if let Some(query) = target_element_query {
        let uia = get_automation()?;
        let fg_hwnd = match target_window_id {
            Some(h) if h > 0 => HWND(h as *mut _),
            _ => unsafe { GetForegroundWindow() },
        };
        if !fg_hwnd.0.is_null() {
            let root = unsafe {
                uia.ElementFromHandle(fg_hwnd)
                    .map_err(|e| format!("ElementFromHandle failed: {e}"))?
            };
            if let Ok((elem, _, rect_opt)) = find_single_element_internal(&root, query, target_control_type) {
                // Programmatic focus
                let _ = unsafe { elem.SetFocus() };
                std::thread::sleep(Duration::from_millis(30));

                let has_focus = unsafe { elem.CurrentHasKeyboardFocus().map(|b| b.as_bool()).unwrap_or(false) };
                if !has_focus {
                    // Fallback click center to acquire focus
                    if let Some(rect) = rect_opt {
                        let cx = rect.left + (rect.right - rect.left) / 2;
                        let cy = rect.top + (rect.bottom - rect.top) / 2;
                        let _ = super::input::click_mouse("left", Some(cx), Some(cy));
                        std::thread::sleep(Duration::from_millis(40));
                    }
                }
            }
        }
    }

    let details = get_focused_control_details_for_target(target_window_id)?;
    if target_window_id.is_some() || target_element_query.is_some() {
        if let Some(r) = &details.bounding_rect {
            let cx = r.x + r.width / 2;
            let cy = r.y + r.height / 2;
            let _ = super::input::click_mouse("left", Some(cx), Some(cy));
            std::thread::sleep(Duration::from_millis(80));
        }
    }
    get_focused_control_details_for_target(target_window_id)
}

/// Verified scrolling via ScrollPattern or mouse wheel
pub fn scroll_element_verified(
    target_hwnd: Option<i64>,
    query: Option<&str>,
    scroll_x: Option<i32>,
    scroll_y: Option<i32>,
) -> Result<serde_json::Value, String> {
    let uia = get_automation()?;
    let hwnd = match target_hwnd {
        Some(h) => HWND(h as *mut _),
        None => unsafe { GetForegroundWindow() },
    };

    if hwnd.0.is_null() {
        return Err("No active window found".to_string());
    }

    let elem = if let Some(q) = query {
        let root = unsafe {
            uia.ElementFromHandle(hwnd)
                .map_err(|e| format!("ElementFromHandle failed: {e}"))?
        };
        find_single_element_internal(&root, q, None).map(|(e, _, _)| e)?
    } else {
        unsafe {
            uia.GetFocusedElement()
                .or_else(|_| uia.ElementFromHandle(hwnd))
                .map_err(|e| format!("Failed to get target element: {e}"))?
        }
    };

    let pre_scroll = get_element_scroll(&elem);
    let sy = scroll_y.unwrap_or(0);
    let sx = scroll_x.unwrap_or(0);

    let mut pattern_succeeded = false;
    unsafe {
        if let Ok(pattern_obj) = elem.GetCurrentPattern(UIA_ScrollPatternId) {
            if let Ok(sp) = pattern_obj.cast::<IUIAutomationScrollPattern>() {
                let h_amount = if sx > 0 {
                    ScrollAmount_SmallIncrement
                } else if sx < 0 {
                    ScrollAmount_SmallDecrement
                } else {
                    ScrollAmount_NoAmount
                };

                let v_amount = if sy > 0 {
                    ScrollAmount_SmallDecrement // positive sy = scroll up
                } else if sy < 0 {
                    ScrollAmount_SmallIncrement // negative sy = scroll down
                } else {
                    ScrollAmount_NoAmount
                };

                if sp.Scroll(h_amount, v_amount).is_ok() {
                    pattern_succeeded = true;
                }
            }
        }
    }

    if !pattern_succeeded {
        // Fallback: mouse wheel at element bounding rect center or current pos
        let rect_opt = unsafe { elem.CurrentBoundingRectangle().ok() };
        if let Some(r) = rect_opt {
            let cx = r.left + (r.right - r.left) / 2;
            let cy = r.top + (r.bottom - r.top) / 2;
            let _ = super::input::move_mouse(cx, cy);
            sleep(Duration::from_millis(20));
        }
        super::input::scroll_mouse(scroll_x, scroll_y)?;
    }

    sleep(Duration::from_millis(60));
    let post_scroll = get_element_scroll(&elem);

    let changed = match (&pre_scroll, &post_scroll) {
        (Some(pre), Some(post)) => {
            (pre.vertical_percent - post.vertical_percent).abs() > 0.001
                || (pre.horizontal_percent - post.horizontal_percent).abs() > 0.001
        }
        _ => true,
    };

    Ok(serde_json::json!({
        "status": "success",
        "verified": changed,
        "method": if pattern_succeeded { "ScrollPattern" } else { "MouseWheel" },
        "scroll_pattern_supported": pre_scroll.is_some(),
        "pre_scroll": pre_scroll,
        "post_scroll": post_scroll,
    }))
}
