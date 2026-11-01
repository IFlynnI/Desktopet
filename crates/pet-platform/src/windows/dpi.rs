use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};

/// Makes every coordinate in this process a physical pixel (WIN-11). Must run before the window exists.
pub fn make_aware() {
    // SAFETY: takes a constant. winit's own later call fails harmlessly because this one came first.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}
