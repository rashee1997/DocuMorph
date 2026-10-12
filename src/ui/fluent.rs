use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Dwm::DwmSetWindowAttribute;
use windows_sys::Win32::Graphics::Gdi::{
    CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, GetStockObject, RoundRect,
    SelectObject, SetBkMode, SetTextColor, DT_CENTER, DT_SINGLELINE, DT_VCENTER, HBRUSH, HDC,
    HPEN, NULL_PEN, PS_SOLID, TRANSPARENT,
};
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_SELECTED};

use crate::com::to_wide;

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

// WinUI 3 Fluent Palette
pub const COLOR_CANVAS_RGB: (u8, u8, u8) = (248, 250, 252);     // Slate-50 soft canvas
pub const COLOR_CARD_RGB: (u8, u8, u8) = (255, 255, 255);       // Pure white card
pub const COLOR_BORDER_RGB: (u8, u8, u8) = (226, 232, 240);     // Slate-200 border
pub const COLOR_BRAND_RGB: (u8, u8, u8) = (190, 18, 60);        // Crimson #BE123C
pub const COLOR_BRAND_HOVER_RGB: (u8, u8, u8) = (225, 29, 72);  // Crimson #E11D48
pub const COLOR_BRAND_PRESS_RGB: (u8, u8, u8) = (159, 18, 57);  // Dark Crimson #9F1239
pub const COLOR_TEXT_MAIN_RGB: (u8, u8, u8) = (15, 23, 42);     // Dark Slate #0F172A
pub const COLOR_TEXT_MUTED_RGB: (u8, u8, u8) = (100, 116, 139); // Slate-500 #64748B

pub struct FluentTheme {
    pub canvas_brush: HBRUSH,
    pub card_brush: HBRUSH,
    pub border_pen: HPEN,
    pub brand_brush: HBRUSH,
    pub brand_press_brush: HBRUSH,
    pub sec_hover_brush: HBRUSH,
}

impl FluentTheme {
    pub fn new() -> Self {
        unsafe {
            let canvas_brush = CreateSolidBrush(rgb(
                COLOR_CANVAS_RGB.0,
                COLOR_CANVAS_RGB.1,
                COLOR_CANVAS_RGB.2,
            ));
            let card_brush = CreateSolidBrush(rgb(
                COLOR_CARD_RGB.0,
                COLOR_CARD_RGB.1,
                COLOR_CARD_RGB.2,
            ));
            let border_pen = CreatePen(
                PS_SOLID,
                1,
                rgb(COLOR_BORDER_RGB.0, COLOR_BORDER_RGB.1, COLOR_BORDER_RGB.2),
            );
            let brand_brush = CreateSolidBrush(rgb(
                COLOR_BRAND_RGB.0,
                COLOR_BRAND_RGB.1,
                COLOR_BRAND_RGB.2,
            ));
            let brand_press_brush = CreateSolidBrush(rgb(
                COLOR_BRAND_PRESS_RGB.0,
                COLOR_BRAND_PRESS_RGB.1,
                COLOR_BRAND_PRESS_RGB.2,
            ));
            let sec_hover_brush = CreateSolidBrush(rgb(241, 245, 249));

            FluentTheme {
                canvas_brush,
                card_brush,
                border_pen,
                brand_brush,
                brand_press_brush,
                sec_hover_brush,
            }
        }
    }
}

impl Drop for FluentTheme {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.canvas_brush);
            DeleteObject(self.card_brush);
            DeleteObject(self.border_pen);
            DeleteObject(self.brand_brush);
            DeleteObject(self.brand_press_brush);
            DeleteObject(self.sec_hover_brush);
        }
    }
}

/// Applies Windows 11 DWM rounded corners to a top-level window
pub fn apply_fluent_window_frame(hwnd: HWND) {
    unsafe {
        // DWMWA_WINDOW_CORNER_PREFERENCE = 33, DWMWCP_ROUND = 2
        let preference = 2u32;
        let _ = DwmSetWindowAttribute(
            hwnd,
            33,
            &preference as *const _ as *const _,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

/// Renders a modern WinUI 3 Fluent card container with rounded corners and subtle border
pub fn draw_card(hdc: HDC, rect: &RECT, theme: &FluentTheme) {
    unsafe {
        let old_brush = SelectObject(hdc, theme.card_brush);
        let old_pen = SelectObject(hdc, theme.border_pen);
        RoundRect(hdc, rect.left, rect.top, rect.right, rect.bottom, 12, 12);
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
    }
}

/// Custom-paints an owner-drawn WinUI 3 style button
pub fn draw_fluent_button(
    dis: &DRAWITEMSTRUCT,
    theme: &FluentTheme,
    text: &str,
    is_primary: bool,
) {
    let hdc = dis.hDC;
    let is_pressed = (dis.itemState & ODS_SELECTED) != 0;
    let wide_text = to_wide(text);

    unsafe {
        SetBkMode(hdc, TRANSPARENT as i32);

        if is_primary {
            // Primary Accent Button (Crimson)
            let brush = if is_pressed {
                theme.brand_press_brush
            } else {
                theme.brand_brush
            };
            let old_brush = SelectObject(hdc, brush);
            let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN));

            RoundRect(
                hdc,
                dis.rcItem.left,
                dis.rcItem.top,
                dis.rcItem.right,
                dis.rcItem.bottom,
                8,
                8,
            );

            SelectObject(hdc, old_brush);
            SelectObject(hdc, old_pen);

            SetTextColor(hdc, rgb(255, 255, 255));
        } else {
            // Secondary Neutral Button (White card surface + border)
            let brush = if is_pressed {
                theme.sec_hover_brush
            } else {
                theme.card_brush
            };
            let old_brush = SelectObject(hdc, brush);
            let old_pen = SelectObject(hdc, theme.border_pen);

            RoundRect(
                hdc,
                dis.rcItem.left,
                dis.rcItem.top,
                dis.rcItem.right,
                dis.rcItem.bottom,
                8,
                8,
            );

            SelectObject(hdc, old_brush);
            SelectObject(hdc, old_pen);

            SetTextColor(
                hdc,
                rgb(
                    COLOR_TEXT_MAIN_RGB.0,
                    COLOR_TEXT_MAIN_RGB.1,
                    COLOR_TEXT_MAIN_RGB.2,
                ),
            );
        }

        let mut rc = dis.rcItem;
        DrawTextW(
            hdc,
            wide_text.as_ptr(),
            (wide_text.len() - 1) as i32,
            &mut rc,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );
    }
}

/// Paints the 4 Fluent card containers on the main window
pub fn paint_main_cards(hdc: HDC, theme: &FluentTheme) {
    let rc1 = RECT { left: 24, top: 46, right: 616, bottom: 172 };
    draw_card(hdc, &rc1, theme);
    let rc2 = RECT { left: 24, top: 182, right: 616, bottom: 244 };
    draw_card(hdc, &rc2, theme);
    let rc3 = RECT { left: 24, top: 254, right: 616, bottom: 338 };
    draw_card(hdc, &rc3, theme);
    let rc4 = RECT { left: 24, top: 348, right: 616, bottom: 464 };
    draw_card(hdc, &rc4, theme);
}

/// Checks if a client Y coordinate falls inside any of the main cards
pub fn is_point_inside_card(y: i32) -> bool {
    (y >= 46 && y <= 172)
        || (y >= 182 && y <= 244)
        || (y >= 254 && y <= 338)
        || (y >= 348 && y <= 464)
}

