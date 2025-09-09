use anyhow::Result;
use tracing::debug;

#[cfg(target_os = "windows")]
pub fn try_restore_main_window_by_title(title: &str) -> Result<bool> {
	use windows::core::PCWSTR;
	use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, ShowWindow, SetForegroundWindow, IsIconic, SW_RESTORE, SW_SHOW, SW_SHOWNORMAL};
	
	// Convertir a wide y a PCWSTR
	let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
	let pwstr = PCWSTR(wide.as_ptr());
	let hwnd = unsafe { FindWindowW(PCWSTR::null(), pwstr)? };
	
	unsafe {
		// Si está minimizada, restaurarla
		if IsIconic(hwnd).as_bool() {
			debug!("↗️ Ventana minimizada: restaurando");
			let _ = ShowWindow(hwnd, SW_RESTORE);
		} else {
			// Asegurar visible/normal
			let _ = ShowWindow(hwnd, SW_SHOWNORMAL);
			let _ = ShowWindow(hwnd, SW_SHOW);
		}
		// Traer al frente
		let _ = SetForegroundWindow(hwnd);
	}
	

	Ok(true)
}

#[cfg(not(target_os = "windows"))]
pub fn try_restore_main_window_by_title(_title: &str) -> Result<bool> {
	Ok(false)
} 